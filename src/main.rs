
use chrono::Utc;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use structopt::StructOpt;
use tui::{backend::CrosstermBackend, Terminal};
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

mod app;
use app::App;
mod chart;
mod cursor;
mod gaugagraph;
mod generate;
use generate::generate_log;
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

pub enum Event<I> {
    Input(I),
    Tick,
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

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    terminal.clear()?;

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
        let mut actions = Vec::new();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            loop {
                // Poll for tick rate duration. If no events then send tick event.
                if event::poll(std::time::Duration::from_millis(250)).unwrap() {
                    if let CEvent::Key(key) = event::read().unwrap() {
                        tx.send(Event::Input(key)).unwrap();
                    }
                }
                tx.send(Event::Tick).unwrap();
            }
        });

        loop {
            draw(&mut terminal, &mut app)?;
            match rx.recv()? {
                Event::Input(key) => {
                    if opt.write_actions.is_some() {
                        actions.push(key.code);
                    }
                    if handle_key(key.code, &mut app) {
                        disable_raw_mode()?;
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                        terminal.show_cursor()?;
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

fn handle_key(key: KeyCode, app: &mut App) -> bool {
    match key {
        KeyCode::Char('q') => return true,
        KeyCode::Char(c) => app.on_char(c),
        KeyCode::Down => app.on_down(),
        KeyCode::Up => app.on_up(),
        KeyCode::PageDown => app.on_page_down(),
        KeyCode::PageUp => app.on_page_up(),
        KeyCode::Home => app.on_home(),
        KeyCode::End => app.on_end(),
        KeyCode::Left => app.on_left(),
        KeyCode::Right => app.on_right(),
        KeyCode::Tab => app.on_tab(),
        KeyCode::Enter => app.on_enter(),
        _ => {}
    }
    false
}
