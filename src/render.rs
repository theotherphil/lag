use crate::app::{AnnotatedLine, App, Cell, Status};
use crate::chart::ChartSection;
use crate::gaugagraph::Gaugagraph;
use std::io;
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{
    Axis, Block, Borders, Chart, Dataset, Marker, Paragraph, SelectableList, Text, Widget,
};
use tui::{Frame, Terminal};

const FOREGROUND: Color = Color::Rgb(248, 248, 242);
const BACKGROUND: Color = Color::Rgb(40, 42, 54);
const RED: Color = Color::Rgb(255, 85, 85);
const ORANGE: Color = Color::Rgb(255, 184, 108);
const CYAN: Color = Color::Rgb(139, 233, 253);

fn default_style() -> Style {
    Style::default().bg(BACKGROUND)
}

trait BlockStatusExt {
    fn status(self, status: Status) -> Self;
}

impl BlockStatusExt for Block<'_> {
    fn status(self, status: Status) -> Self {
        match status {
            Status::Active => self
                .borders(Borders::TOP)
                .title_style(default_style().fg(RED))
                .border_style(default_style().fg(RED)),
            Status::Inactive => self
                .borders(Borders::TOP)
                .title_style(default_style().fg(FOREGROUND))
                .border_style(default_style()),
        }
    }
}

pub fn draw<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<(), io::Error> {
    terminal.draw(|mut f| {
        let size = f.size();

        Block::default().style(default_style()).render(&mut f, size);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
            .split(size);

        draw_top_row(&mut f, app, rows[0]);
        draw_bottom_row(&mut f, app, rows[1]);
    })?;
    Ok(())
}

pub fn draw_top_row<B: Backend>(frame: &mut Frame<B>, app: &mut App, rect: Rect) {
    Block::default()
        .style(default_style())
        .border_style(default_style())
        .status(app.status(Cell::Log))
        .title(&format!("Log (bars scaled by {:.2})", app.log_bar_zoom))
        .render(frame, rect);

    // Line number | Elapsed time | Log line
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints(
            [
                Constraint::Percentage(5),
                Constraint::Percentage(5),
                Constraint::Percentage(90),
            ]
            .as_ref(),
        )
        .split(rect);

    draw_line_numbers(frame, app, split[0]);
    draw_elapsed_times(frame, app, split[1]);
    draw_log_lines(frame, app, split[2]);
}

pub fn draw_bottom_row<B: Backend>(frame: &mut Frame<B>, app: &mut App, rect: Rect) {
    // Chart | Spacer | Diff list
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(37),
                Constraint::Percentage(3),
                Constraint::Percentage(60),
            ]
            .as_ref(),
        )
        .split(rect);

    draw_chart(frame, app, split[0]);
    draw_diff_list(frame, app, split[2]);
}

#[inline(never)]
pub fn draw_log_lines<B: Backend>(frame: &mut Frame<B>, app: &mut App, rect: Rect) {
    let scroll = app.vertical_log_scroll();

    let log_text: Vec<_> = app
        .lines
        .iter()
        .skip(scroll)
        .take(rect.height as usize)
        .map(|l| {
            let offset = app.horizontal_log_scroll().min(l.line.len() - 1);
            &l.line[offset..]
        })
        .collect();

    let data = app.elapsed_time_ratios(scroll, scroll + rect.height as usize);

    Gaugagraph::new(
        log_text,
        default_style(),
        default_style().bg(ORANGE).fg(BACKGROUND),
        data,
        app.log_bar_zoom,
    )
    .block(
        Block::default()
            .title_style(default_style().modifier(Modifier::BOLD))
            .border_style(default_style()),
    )
    .render(frame, rect);
}

#[inline(never)]
pub fn draw_elapsed_times<B: Backend>(frame: &mut Frame<B>, app: &mut App, rect: Rect) {
    let text: Vec<_> = app
        .lines
        .iter()
        .skip(app.vertical_log_scroll())
        .take(rect.height as usize)
        .map(|l| Text::Raw(format!("{}\n", l.elapsed_string()).into()))
        .collect();

    Paragraph::new(text.iter())
        .block(
            Block::default()
                .border_style(default_style())
                .title_style(default_style().modifier(Modifier::BOLD)),
        )
        .style(default_style().fg(ORANGE))
        .render(frame, rect);
}

#[inline(never)]
pub fn draw_line_numbers<B: Backend>(frame: &mut Frame<B>, app: &mut App, rect: Rect) {
    let text: Vec<_> = app
        .line_numbers()
        .skip(app.vertical_log_scroll())
        .take(rect.height as usize)
        .map(|x| Text::Raw(format!("{}\n", x).into()))
        .collect();

    Paragraph::new(text.iter())
        .block(
            Block::default()
                .border_style(default_style())
                .title_style(default_style().modifier(Modifier::BOLD)),
        )
        .alignment(Alignment::Left)
        .wrap(false)
        .style(default_style())
        .render(frame, rect);
}

#[inline(never)]
pub fn draw_chart<B: Backend>(frame: &mut Frame<B>, app: &mut App, rect: Rect) {
    let (lower, upper) = app.chart_state.interval;
    let ChartSection {
        points,
        x_bounds,
        y_bounds,
    } = app.chart_state.section();
    let label_step_y = (y_bounds.1 - y_bounds.0) / 4.0;

    let y_labels: Vec<_> = vec![
        y_bounds.0,
        y_bounds.0 + label_step_y,
        y_bounds.0 + 2.0 * label_step_y,
        y_bounds.0 + 3.0 * label_step_y,
        y_bounds.0 + 4.0 * label_step_y,
    ]
    .iter()
    .map(|x| format!("{:.2}", x))
    .collect();

    let cdf = Dataset::default()
        .name("CumulativeTime")
        .marker(Marker::Braille)
        .style(default_style().fg(CYAN))
        .data(&points);

    let x_labels: Vec<_> = (lower..upper + 1)
        .step_by(20 * app.lines_per_pixel())
        .map(|x| x.to_string())
        .collect();

    let loc_data = vec![(
        app.vertical_log_scroll() as f64,
        0.5 * y_bounds.0 + 0.5 * y_bounds.1,
    )];
    let location = Dataset::default()
        .name("CurrentLine")
        .marker(Marker::Dot)
        .style(default_style().fg(RED))
        .data(&loc_data);

    let chart_block = Block::default()
        .style(default_style())
        .border_style(default_style());

    let is_active = app.status(Cell::Chart) == Status::Active;

    let styled_axis = |title| {
        Axis::default()
            .title(title)
            .title_style(default_style())
            .style(default_style().fg(if is_active { RED } else { FOREGROUND }))
            .labels_style(default_style().modifier(Modifier::ITALIC))
    };

    let (lower, upper) = (lower as f64, upper as f64);

    let y_title = format!(
        "Fraction of cumulative time (zoom: {:.2})",
        app.chart_state.current_zoom_level()
    );

    Chart::default()
        .block(chart_block)
        .x_axis(
            styled_axis("Line number")
                .bounds([lower, upper])
                .labels(&x_labels),
        )
        .y_axis(
            styled_axis(&y_title)
                .bounds([y_bounds.0, y_bounds.1])
                .labels(&y_labels),
        )
        .style(default_style())
        .datasets(&[cdf, location])
        .render(frame, rect);
}

// Show both current & prev?
fn render_diff_list_item(line: &AnnotatedLine) -> String {
    format!("{:10} {}", line.elapsed_string(), line.line)
}

#[inline(never)]
pub fn draw_diff_list<B: Backend>(frame: &mut Frame<B>, app: &mut App, rect: Rect) {
    let deltas: Vec<_> = app
        .largest_diffs
        .iter()
        .map(|line| {
            let mut line = render_diff_list_item(line);
            // Selectable list renders black cells past the final log character
            // Think this is a bug in tui-rs. In the meantime, here's a massive hack to make
            // the POC less ugly.
            if line.len() < 150 {
                line = line + "                                                                ";
            }
            line
        })
        .map(|l| {
            let offset = app.horizontal_diff_scroll().min(l.len() - 1);
            format!("{}\n", &l[offset..])
        })
        .collect();

    let deltas: Vec<_> = deltas.iter().map(|x| x as &str).collect();

    SelectableList::default()
        .block(
            Block::default()
                .title("Largest diffs")
                .title_style(default_style())
                .border_style(default_style())
                .style(default_style())
                .status(app.status(Cell::List)),
        )
        .items(&deltas)
        .select(Some(app.vertical_diff_scroll()))
        .style(default_style().fg(FOREGROUND))
        .highlight_style(Style::default().bg(FOREGROUND).fg(BACKGROUND))
        .highlight_symbol(">>")
        .render(frame, rect)
}
