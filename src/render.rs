use crate::app::{AnnotatedLine, App, Panel, Status};
use crate::chart::ChartSection;
use crate::gaugagraph::Gaugagraph;
use std::io;
use std::iter;
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{
    Axis, Block, Borders, Chart, Dataset, Marker, Paragraph, SelectableList, Text, Widget,
};
use tui::{Frame, Terminal};
use HelpText::{Body, Title, Gap};

const FOREGROUND: Color = Color::Rgb(248, 248, 242);
const BACKGROUND: Color = Color::Rgb(40, 42, 54);
const RED: Color = Color::Rgb(255, 85, 85);
const ORANGE: Color = Color::Rgb(255, 184, 108);
const CYAN: Color = Color::Rgb(139, 233, 253);
const WHITE: Color = Color::Rgb(255, 255, 255);

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
            .constraints(
                [
                    Constraint::Percentage(2),
                    Constraint::Percentage(68),
                    Constraint::Percentage(30),
                ]
                .as_ref(),
            )
            .split(size);

        draw_help(&mut f, rows[0]);
        draw_log_panel(&mut f, app, rows[1]);
        draw_bottom_row(&mut f, app, rows[2]);
    })?;
    Ok(())
}

fn draw_help<B: Backend>(frame: &mut Frame<B>, rect: Rect) {
    Paragraph::new([Text::Raw("(Press 'h' to toggle help)".into())].iter())
        .alignment(Alignment::Right)
        .style(default_style())
        .render(frame, rect);
}

enum HelpText {
    Title(Color, &'static str),
    Body(&'static str),
    Gap(usize),
}

fn help_text(help_section: &[HelpText]) -> Vec<Text> {
    help_section
        .iter()
        .map(|s| match s {
            HelpText::Title(c, t) => Text::Styled(format!("{}\n", t).into(), default_style().fg(*c)),
            HelpText::Body(b) => Text::Raw(format!("{}\n", b).into()),
            HelpText::Gap(n) => Text::Raw(iter::repeat('\n').take(*n).collect::<String>().into()),
        })
        .collect()
}

fn draw_log_panel<B: Backend>(frame: &mut Frame<B>, app: &mut App, rect: Rect) {
    Block::default()
        .style(default_style())
        .status(app.status(Panel::Log))
        .title(&format!("Log (bars scaled by {:.2})", app.log_bar_zoom))
        .render(frame, rect);

    if app.help_mode {
        let text = vec![
            Gap(1),
            Body("Press tab to move between panels. The active panel is highlighted in red
Navigation instructions are shown within each panel"),
            Gap(2),
            Title(ORANGE, "Log panel (this one)"),
            Body("Each line from the log file is shown alongside its line number and the elapsed time between it and the previous line
Coloured bars are shown behind each log line, whose lengths are proportional to the elapsed times
The bars are initially scaled so that the bar for the largest elapsed time fills the width of the panel"),
            Gap(1),
            Title(ORANGE, "Chart panel"),
            Body("This panel plots line numbers against the cumulative elapsed time up to that point, as a fraction of the total time
The red dot shows the position of the current line, which can be moved by scrolling within this panel"),
            Gap(1),
            Title(ORANGE, "Largest diffs panel"),
            Body("This panel shows the lines with largest elapsed times.
Hitting enter on a selected line moves the current line to that location"),
            Gap(2),
            Title(CYAN, "Navigation"),
            Gap(1),
            Title(WHITE, "Vertical scrolling"),
            Body("Up/Down, PageUp/PageDown"),
            Gap(1),
            Title(WHITE, "Horizontal scrolling"),
            Body("Left/Right, Home/End"),
            Gap(1),
            Title(WHITE, "Zoom"),
            Body("+ stretches the bars, - shrinks them
Escape resets the zoom"),
        ];
        let text = help_text(&text);

        Paragraph::new(text.iter())
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .status(app.status(Panel::Log))
                    .borders(Borders::ALL)
                    .title("Log panel"),
            )
            .style(default_style())
            .render(frame, rect);

        return;
    }

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

fn draw_bottom_row<B: Backend>(frame: &mut Frame<B>, app: &mut App, rect: Rect) {
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

fn draw_log_lines<B: Backend>(frame: &mut Frame<B>, app: &mut App, rect: Rect) {
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

fn draw_elapsed_times<B: Backend>(frame: &mut Frame<B>, app: &mut App, rect: Rect) {
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

fn draw_line_numbers<B: Backend>(frame: &mut Frame<B>, app: &mut App, rect: Rect) {
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

fn draw_chart<B: Backend>(frame: &mut Frame<B>, app: &mut App, rect: Rect) {
    if app.help_mode {
        let text = vec![
            Title(CYAN, "Navigation"),
            Gap(1),
            Title(WHITE, "Increase/decrease current line"),
            Body("Left/Right, or Home/End for larger steps"),
            Gap(1),
            Title(WHITE, "Zoom"),
            Body("Up/Down to zoom in/out, or PageUp/PageDown for larger steps\nEscape resets the zoom level"),
        ];
        let text = help_text(&text);

        Paragraph::new(text.iter())
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .status(app.status(Panel::Chart))
                    .borders(Borders::ALL)
                    .title("Chart panel"),
            )
            .style(default_style())
            .render(frame, rect);

        return;
    }

    let (lower, upper) = app.chart_state.interval;
    let ChartSection {
        points, y_bounds, ..
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

    let is_active = app.status(Panel::Chart) == Status::Active;

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

fn render_diff_list_item(line: &AnnotatedLine, offset: usize) -> String {
    let contents = if offset >= line.line.len() {
        ""
    } else {
        &line.line[offset..]
    };
    format!(
        "{:<10} {:10} {}",
        line.line_number,
        line.elapsed_string(),
        contents
    )
}

fn draw_diff_list<B: Backend>(frame: &mut Frame<B>, app: &mut App, rect: Rect) {
    if app.help_mode {
        let text = vec![
            Title(CYAN, "Navigation"),
            Gap(1),
            Title(WHITE, "Vertical scrolling"),
            Body("Up/Down, PageUp/PageDown"),
            Gap(1),
            Title(WHITE, "Horizontal scrolling"),
            Body("Left/Right, Home/End"),
            Gap(1),
            Title(WHITE, "Jump-to-line"),
            Body("Enter"),
        ];
        let text = help_text(&text);

        Paragraph::new(text.iter())
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .status(app.status(Panel::List))
                    .borders(Borders::ALL)
                    .title("Largest diffs panel"),
            )
            .style(default_style())
            .render(frame, rect);

        return;
    }

    let deltas: Vec<_> = app
        .largest_diffs
        .iter()
        .map(|line| render_diff_list_item(line, app.horizontal_diff_scroll()))
        .collect();

    let deltas: Vec<_> = deltas.iter().map(|x| x as &str).collect();

    SelectableList::default()
        .block(
            Block::default()
                .title("Largest diffs")
                .style(default_style())
                .status(app.status(Panel::List)),
        )
        .items(&deltas)
        .select(Some(app.vertical_diff_scroll()))
        .style(default_style().fg(FOREGROUND))
        .highlight_style(Style::default().bg(FOREGROUND).fg(BACKGROUND))
        .highlight_symbol(">>")
        .render(frame, rect)
}
