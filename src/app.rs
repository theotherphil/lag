use crate::chart::ChartState;
use crate::cursor::Cursor;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use lazycell::LazyCell;
use lazysort::SortedBy;
use rayon::prelude::*;
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

// Total hack to deal with "PT" prefix added by Duration::Display. TODO: replace
fn render_duration(dur: Duration) -> String {
    format!("{}", dur)[2..].to_string()
}

#[inline(never)]
fn create_annotated_lines<'a>(
    lines: &'a [&'a str],
    timestamps: &[DateTime<Utc>],
) -> Vec<AnnotatedLine<'a>> {
    assert_eq!(lines.len(), timestamps.len());

    let mut annotated = Vec::with_capacity(lines.len());
    let mut prev = timestamps[0];

    for i in 0..lines.len() {
        let line = lines[i];
        let timestamp = timestamps[i];
        let diff = timestamp - prev;
        prev = timestamp;
        annotated.push(AnnotatedLine::new(i, line, timestamp, diff));
    }

    annotated
}

pub fn extract_timestamp(line: &str) -> Option<DateTime<Utc>> {
    let p = NaiveDateTime::parse_from_str(&line[0..24], "%Y-%m-%d %H:%M:%S.%3fZ").ok();
    if let Some(d) = p {
        let p = DateTime::<Utc>::from_utc(d, Utc);
        return Some(p);
    }
    let t = line.split_whitespace().nth(0)?;
    let p = t.parse::<DateTime<Utc>>().ok();
    p
}

// Handle lines without timestamps by using keep-last.
// If there are leading lines without timestamps then give them all the
// first timestamp encountered.
#[inline(never)]
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

#[derive(Clone, Debug)]
pub struct AnnotatedLine<'a> {
    pub line_number: usize,
    pub line: &'a str,
    pub timestamp: DateTime<Utc>,
    pub elapsed: Duration,
    pub elapsed_string: LazyCell<String>,
    pub elapsed_millis: f64,
}

impl<'a> AnnotatedLine<'a> {
    fn new(
        line_number: usize,
        line: &'a str,
        timestamp: DateTime<Utc>,
        elapsed: Duration,
    ) -> AnnotatedLine {
        AnnotatedLine {
            line_number,
            line,
            timestamp,
            elapsed,
            elapsed_string: LazyCell::new(),
            elapsed_millis: elapsed.num_milliseconds() as f64,
        }
    }

    pub fn elapsed_string(&self) -> &str {
        if !self.elapsed_string.filled() {
            self.elapsed_string
                .fill(render_duration(self.elapsed))
                .unwrap();
        }
        self.elapsed_string.borrow().unwrap()
    }
}

#[derive(Debug)]
pub struct App<'a> {
    pub lines: Vec<AnnotatedLine<'a>>,
    // The top 1000 lines by decreasing elapsed time
    pub largest_diffs: Vec<AnnotatedLine<'a>>,
    pub log_cursor: Cursor,
    pub diff_cursor: Cursor,
    pub active: Cell,
    pub chart_state: ChartState,
    pub log_bar_zoom: f64,
}

impl<'a> App<'a> {
    pub fn new(log: &'a [&'a str]) -> App<'a> {
        let num_lines = log.len();
        let max_len = log.iter().map(|l| l.len()).max().unwrap();
        let timestamps: Vec<_> = log.par_iter().map(|l| extract_timestamp(l)).collect();
        let timestamps = fill_in_timestamps(&timestamps);
        let lines = create_annotated_lines(&log, &timestamps);

        let largest_diffs: Vec<_> = lines
            .iter()
            .sorted_by(|x, y| y.elapsed.cmp(&x.elapsed))
            .take(1000)
            .cloned()
            .collect();

        let total_time = lines[lines.len() - 1].timestamp - lines[0].timestamp;
        let total_millis = total_time.num_milliseconds() as f64;
        let deltas = lines
            .iter()
            .map(|l| l.elapsed_millis / total_millis)
            .collect();

        App {
            lines,
            largest_diffs,
            log_cursor: Cursor::new(max_len - 1, num_lines - 1),
            diff_cursor: Cursor::new(max_len - 1, num_lines - 1),
            active: Cell::Log,
            chart_state: ChartState::new(deltas),
            log_bar_zoom: 1.0,
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

    pub fn elapsed_time_ratios(&self, from: usize, to: usize) -> Vec<f64> {
        let max_diff = self.largest_diffs[0].elapsed_millis;
        self.lines
            .iter()
            .skip(from)
            .take(to - from + 1)
            .map(|l| l.elapsed_millis / max_diff)
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
            let target_line = self.largest_diffs[selected_line].line_number;
            self.log_cursor.y = if target_line == 0 { 0 } else { target_line - 1 };
        }
        // +/-
        if self.active == Cell::Log {
            if c == '+' {
                self.log_bar_zoom = 1000.0f64.min(self.log_bar_zoom * 1.5);
            }
            if c == '-' {
                self.log_bar_zoom = 1.0f64.max(self.log_bar_zoom / 1.5);
            }
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
