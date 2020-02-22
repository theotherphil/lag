use chrono::prelude::*;
use chrono::{Duration, Utc};
use std::io;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::Terminal;

mod app;
use app::App;
mod chart;
mod event;
use event::{Event, Events};
mod gaugagraph;
mod render;
use render::draw;

// TODO
// Custom timestamp parsing
// Handle lines with missing timestamps
// Upper bound for times used for determining lengths of gauges
// Support > 65k lines
// Search/filtering
// Add ability to select region of chart using shift + arrows then enter to expand selected region
// to fill chart area
// Show distribution of times (on a second tab)
// Handle edge cases, e.g. no lines, max diff of 0, no lines with timestamps, timestamps decreasing
// ctrl+g for go-to line (and esc to cancel)
// Less fun, but possibly more useful: just generate a new file with extra info at the start of
// each line and view it in a regular log viewer
// Abilility to run a list of commands from a file for (perf) testing
// Option to vary gaugagraph scale (i/o keys?)
// Up/down to smoothly zoom in/out of chart
// Filter list of largest diffs to those in the currently visible region of the chart

fn main() -> Result<(), failure::Error> {
    generate_log("gen_log.txt", Utc::now(), 2115);

    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let log = std::fs::read_to_string("gen_log.txt")?;
    let mut app =
        App::new(log.lines().map(|l| l.to_string()).collect()).cutoff(Duration::seconds(5));
    let events = Events::new();

    loop {
        draw(&mut terminal, &mut app)?;
        match events.next()? {
            Event::Input(key) => match key {
                Key::Char('q') => break,
                Key::Char(c) => app.on_char(c),
                Key::Down => app.on_down(),
                Key::Up => app.on_up(),
                Key::PageDown => app.on_page_down(),
                Key::PageUp => app.on_page_up(),
                Key::Home => app.on_home(),
                Key::End => app.on_end(),
                Key::Left => app.on_left(),
                Key::Right => app.on_right(),
                Key::Esc => app.on_esc(),
                _ => {}
            },
            _ => {}
        }
    }
    Ok(())
}

fn generate_log(path: &str, start: DateTime<Utc>, count: usize) {
    use rand::Rng;
    use std::io::Write;

    if std::path::Path::new(path).exists() {
        std::fs::remove_file(path).unwrap();
    }

    let mut rng = rand::thread_rng();
    let mut output = std::fs::File::create(path).unwrap();

    let words = vec!["apple", "orange", "banana"];

    let mut timestamp = start;

    let outsize_line = rng.gen_range(5, count.min(30));

    for i in 0..count {
        if i != 0 && i != 4 {
            //write!(output, "{} ", timestamp.to_rfc3339()).unwrap();
            write!(output, "{} ", timestamp.format("%Y-%m-%d %H:%M:%S.%3fZ")).unwrap();
        }

        write!(output, "{} ", i).unwrap();
        for _ in 0..rng.gen_range(1, 30usize) {
            write!(output, "{} ", words[i % words.len()]).unwrap();
        }
        writeln!(output, "").unwrap();

        let mut delay_ms: i64 = rng.gen_range(0, 1000);
        if rng.gen_range(0, 100) == 99i64 {
            delay_ms += 3000;
        }
        if i == outsize_line {
            delay_ms = 150000;
        }
        timestamp = timestamp + Duration::milliseconds(delay_ms);
    }
}
