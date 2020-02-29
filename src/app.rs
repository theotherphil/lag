use crate::chart::ChartState;
use crate::cursor::Cursor;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use std::ops::Range;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Cell {
    Log,
    Chart,
    List,
}

impl Cell {
    fn next(self) -> Self {
        match self {
            Cell::Log => Cell::Chart,
            Cell::Chart => Cell::List,
            Cell::List => Cell::Log,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Active,
    Inactive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnotatedLine {
    pub line_number: usize,
    pub line: String,
    pub timestamp: DateTime<Utc>,
    pub elapsed: Duration,
}

impl AnnotatedLine {
    fn new(
        line_number: usize,
        line: String,
        timestamp: DateTime<Utc>,
        elapsed: Duration,
    ) -> AnnotatedLine {
        AnnotatedLine {
            line_number,
            line,
            timestamp,
            elapsed,
        }
    }
}

fn create_annotated_lines(lines: &[String], timestamps: &[DateTime<Utc>]) -> Vec<AnnotatedLine> {
    let diffs = diffs(timestamps);
    lines
        .iter()
        .enumerate()
        .zip(timestamps)
        .zip(&diffs)
        .map(|(((i, l), t), d)| AnnotatedLine::new(i, l.to_string(), *t, *d))
        .collect()
}

fn diffs(timestamps: &[DateTime<Utc>]) -> Vec<Duration> {
    let mut diffs = Vec::new();
    if timestamps.len() > 0 {
        diffs.push(Duration::milliseconds(0));
    }
    for i in 1..timestamps.len() {
        diffs.push(timestamps[i] - timestamps[i - 1]);
    }
    diffs
}

#[derive(Debug)]
pub struct App {
    pub lines: Vec<AnnotatedLine>,
    // Lines sorted by decreasing elapsed time
    pub sorted_lines: Vec<AnnotatedLine>,
    pub log_cursor: Cursor,
    pub diff_cursor: Cursor,
    pub cutoff: Duration,
    pub active: Cell,
    pub chart_state: ChartState,
}

pub fn extract_timestamp(line: &str) -> Option<DateTime<Utc>> {
    let t = line.split_whitespace().nth(0)?;
    let p = t.parse::<DateTime<Utc>>().ok();
    if let Some(d) = p {
        return Some(d);
    }
    let t: Vec<_> = line.split_whitespace().take(2).collect();
    let t = t.join(" ");
    let p = NaiveDateTime::parse_from_str(&t, "%Y-%m-%d %H:%M:%S.%3fZ").ok();
    if let Some(d) = p {
        let p = DateTime::<Utc>::from_utc(d, Utc);
        return Some(p);
    }
    None
}

// Handle lines without timestamps by using keep-last.
// If there are leading lines without timestamps then give them all the
// first timestamp encountered.
fn fill_in_timestamps(lines: &[Option<DateTime<Utc>>]) -> Vec<DateTime<Utc>> {
    let first = lines
        .iter()
        .find(|l| l.is_some())
        .expect("Unable to extract valid timestamp from any line")
        .unwrap();

    let mut prev = first;
    let mut result = Vec::with_capacity(lines.len());
    for line in lines {
        if let Some(t) = line {
            result.push(*t);
            prev = *t;
        } else {
            result.push(prev);
        }
    }

    result
}

impl App {
    pub fn new(log: Vec<String>) -> App {
        let num_lines = log.len();
        let max_len = log.iter().map(|l| l.len()).max().unwrap();
        let timestamps: Vec<_> = log.iter().map(|l| extract_timestamp(l)).collect();
        let timestamps = fill_in_timestamps(&timestamps);
        let lines = create_annotated_lines(&log, &timestamps);
        let mut sorted_lines = lines.clone();
        sorted_lines.sort_by(|x, y| y.elapsed.cmp(&x.elapsed));

        let total_time = lines[lines.len() - 1].timestamp - lines[0].timestamp;
        let total_millis = total_time.num_milliseconds() as f64;
        let deltas = lines
            .iter()
            .map(|l| l.elapsed.num_milliseconds() as f64 / total_millis)
            .collect();

        App {
            lines,
            sorted_lines,
            log_cursor: Cursor::new(max_len - 1, num_lines - 1),
            diff_cursor: Cursor::new(max_len - 1, num_lines - 1),
            cutoff: Duration::seconds(0),
            active: Cell::Log,
            chart_state: ChartState::new(deltas),
        }
    }

    pub fn cutoff(self, d: Duration) -> App {
        App {
            lines: self.lines,
            sorted_lines: self.sorted_lines,
            log_cursor: self.log_cursor,
            diff_cursor: self.diff_cursor,
            cutoff: d,
            active: self.active,
            chart_state: self.chart_state,
        }
    }

    pub fn vertical_log_scroll(&self) -> usize {
        self.log_cursor.y
    }

    pub fn horizontal_log_scroll(&self) -> usize {
        self.log_cursor.x
    }

    pub fn vertical_diff_scroll(&self) -> usize {
        self.diff_cursor.y
    }

    pub fn horizontal_diff_scroll(&self) -> usize {
        self.diff_cursor.x
    }

    pub fn lines_per_pixel(&self) -> usize {
        self.chart_state.interval_length() / self.chart_state.horizontal_resolution
    }

    pub fn elapsed_time_ratios_with_cutoff(&self) -> Vec<f64> {
        // This is a rendering decision, not a property of the data - move it into Gaugagraph
        let max_diff = self
            .lines
            .iter()
            .map(|l| l.elapsed)
            .max()
            .unwrap()
            .min(self.cutoff);

        self.lines
            .iter()
            .map(|l| l.elapsed)
            .map(|d| d.num_milliseconds() as f64 / max_diff.num_milliseconds() as f64)
            .collect()
    }

    pub fn line_numbers(&self) -> Range<usize> {
        (0..self.lines.len())
    }

    fn scroll_log(&mut self, n: isize) {
        self.log_cursor.move_y(n);
        self.chart_state.update(self.log_cursor.y);
    }

    pub fn on_up(&mut self) {
        match self.active {
            Cell::Log => self.scroll_log(-1),
            Cell::Chart => self.chart_state.zoom_in(self.log_cursor.y),
            Cell::List => self.diff_cursor.move_y(-1),
        }
    }

    pub fn on_down(&mut self) {
        match self.active {
            Cell::Log => self.scroll_log(1),
            Cell::Chart => self.chart_state.zoom_out(self.log_cursor.y),
            Cell::List => self.diff_cursor.move_y(1),
        }
    }

    pub fn on_page_up(&mut self) {
        match self.active {
            Cell::Log => self.scroll_log(-15),
            Cell::Chart => {
                for _ in 0..3 {
                    self.chart_state.zoom_in(self.log_cursor.y);
                }
            }
            Cell::List => self.diff_cursor.move_y(-15),
        }
    }

    pub fn on_page_down(&mut self) {
        match self.active {
            Cell::Log => self.scroll_log(15),
            Cell::Chart => {
                for _ in 0..3 {
                    self.chart_state.zoom_out(self.log_cursor.y);
                }
            }
            Cell::List => self.diff_cursor.move_y(15),
        }
    }

    pub fn on_right(&mut self) {
        match self.active {
            Cell::Log => self.log_cursor.move_x(3),
            Cell::Chart => self.scroll_log(1 * self.lines_per_pixel() as isize),
            Cell::List => self.diff_cursor.move_x(3),
        }
    }

    pub fn on_left(&mut self) {
        match self.active {
            Cell::Log => self.log_cursor.move_x(-3),
            Cell::Chart => self.scroll_log(-1 * self.lines_per_pixel() as isize),
            Cell::List => self.diff_cursor.move_x(-3),
        }
    }

    pub fn on_home(&mut self) {
        match self.active {
            Cell::Log => self.log_cursor.move_to_left_boundary(),
            Cell::Chart => self.scroll_log(-15 * self.lines_per_pixel() as isize),
            Cell::List => self.diff_cursor.move_to_left_boundary(),
        }
    }

    pub fn on_end(&mut self) {
        match self.active {
            Cell::Log => self.log_cursor.move_to_right_boundary(),
            Cell::Chart => self.scroll_log(15 * self.lines_per_pixel() as isize),
            Cell::List => self.diff_cursor.move_to_right_boundary(),
        }
    }

    pub fn on_char(&mut self, c: char) {
        // Tab
        if c as u32 == 9 {
            self.active = self.active.next();
        }
        // Enter
        if self.active == Cell::List && c as u32 == 10 {
            let selected_line = self.diff_cursor.y;
            let target_line = self.sorted_lines[selected_line].line_number;
            self.log_cursor.y = if target_line == 0 { 0 } else { target_line - 1 };
        }
    }

    pub fn status(&self, cell: Cell) -> Status {
        if cell == self.active {
            Status::Active
        } else {
            Status::Inactive
        }
    }
}
