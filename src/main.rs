use chrono::prelude::*;
use chrono::{Duration, Utc};
use std::io;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use structopt::StructOpt;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::Terminal;

mod app;
use app::App;
mod chart;
mod cursor;
mod event;
use event::{Event, Events};
mod gaugagraph;
mod render;
use render::draw;
mod replay;
use replay::{read_action_log, write_action_log};

// TODO
// Search/filtering
// Show distribution of times (on a second tab)
// Handle edge cases, e.g. no lines, max diff of 0, no lines with timestamps, timestamps decreasing
// ctrl+g for go-to line (and esc to cancel)
// Filter list of largest diffs to those in the currently visible region of the chart
// Help tab

#[derive(Debug, StructOpt)]
#[structopt(name = "Lag", about = "A TUI for viewing elapsed times in log files")]
struct Opt {
    /// Log file to open.
    #[structopt(short, long, parse(from_os_str))]
    input: Option<PathBuf>,

    /// If set then the input commands are logged to enable later replay.
    #[structopt(short, long, parse(from_os_str))]
    write_actions: Option<PathBuf>,

    /// If set then inputs are read from this file and user inputs are ignored.
    #[structopt(short, long, parse(from_os_str))]
    read_actions: Option<PathBuf>,

    /// If true then a randomly generated input file is used.
    #[structopt(long, short)]
    generate: bool,
}

#[inline(never)]
fn read_log(path: &PathBuf) -> Result<String, failure::Error> {
    std::fs::read_to_string(&path).map_err(|e| e.into())
}

fn main() -> Result<(), failure::Error> {
    let opt = Opt::from_args();

    let log_file = if opt.generate {
        generate_log("gen_log.txt", Utc::now(), 750_000);
        PathBuf::from("gen_log.txt")
    } else {
        opt.input.expect("No log file provided")
    };

    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let log = read_log(&log_file)?;
    let lines: Vec<_> = log.lines().collect();
    let mut app = App::new(&lines);

    if let Some(file) = opt.read_actions {
        let actions = read_action_log(&file)?;
        for key in &actions {
            draw(&mut terminal, &mut app)?;
            handle_key(*key, &mut app);
        }
    } else {
        let events = Events::new();
        let mut actions = Vec::new();
        loop {
            draw(&mut terminal, &mut app)?;
            match events.next()? {
                Event::Input(key) => {
                    if opt.write_actions.is_some() {
                        actions.push(key);
                    }
                    if handle_key(key, &mut app) {
                        break;
                    }
                }
                _ => {}
            }
        }

        if let Some(file) = opt.write_actions {
            write_action_log(&file, &actions)?;
        }
    }

    Ok(())
}

fn handle_key(key: Key, app: &mut App) -> bool {
    match key {
        Key::Char('q') => return true,
        Key::Char(c) => app.on_char(c),
        Key::Down => app.on_down(),
        Key::Up => app.on_up(),
        Key::PageDown => app.on_page_down(),
        Key::PageUp => app.on_page_up(),
        Key::Home => app.on_home(),
        Key::End => app.on_end(),
        Key::Left => app.on_left(),
        Key::Right => app.on_right(),
        _ => {}
    }
    false
}

fn generate_log(path: &str, start: DateTime<Utc>, count: usize) {
    use rand::Rng;

    if std::path::Path::new(path).exists() {
        std::fs::remove_file(path).unwrap();
    }

    let mut rng = rand::thread_rng();
    let mut output = BufWriter::new(std::fs::File::create(path).unwrap());

    let words = vec!["apple", "orange", "banana"];

    let mut timestamp = start;

    for i in 0..count {
        if i != 0 && i != 4 {
            write!(output, "{} ", timestamp.format("%Y-%m-%d %H:%M:%S.%3fZ")).unwrap();
        }

        write!(output, "{} ", i).unwrap();
        for _ in 0..rng.gen_range(1, 30usize) {
            write!(output, "{} ", words[i % words.len()]).unwrap();
        }
        writeln!(output, "").unwrap();

        let mut delay_ms: i64 = rng.gen_range(0, 1000);
        if rng.gen_range(0, 100) == 99i64 {
            delay_ms += 10000;
        }
        if i == 250 {
            delay_ms = 150000;
        }
        timestamp = timestamp + Duration::milliseconds(delay_ms);
    }
}
