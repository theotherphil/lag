//! Support for reading and writing lists of user inputs for replaying and perf testing.

use serde::{Deserialize, Serialize};
use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::PathBuf;
use crossterm::event::KeyCode;

pub fn write_action_log(file: &PathBuf, keys: &[KeyCode]) -> Result<(), failure::Error> {
    let mut file = File::create(file)?;
    let serialisable: Vec<_> = keys.iter().map(to_serialisable).collect();
    let serialised = serde_json::to_string(&serialisable).unwrap();
    write!(file, "{}", serialised)?;
    Ok(())
}

pub fn read_action_log(file: &PathBuf) -> Result<Vec<KeyCode>, failure::Error> {
    let contents = read_to_string(file)?;
    let deserialized: Vec<KeyData> = serde_json::from_str(&contents)?;
    Ok(deserialized.iter().map(from_serialisable).collect())
}

/// Key representation used for writing and reading replay files.
#[derive(Serialize, Deserialize, Debug)]
enum KeyData {
    Char(char),
    Down,
    Up,
    PageDown,
    PageUp,
    Home,
    End,
    Left,
    Right,
    Tab,
    Enter,
}

fn to_serialisable(key: &KeyCode) -> KeyData {
    match key {
        KeyCode::Char(c) => KeyData::Char(*c),
        KeyCode::Down => KeyData::Down,
        KeyCode::Up => KeyData::Up,
        KeyCode::PageDown => KeyData::PageDown,
        KeyCode::PageUp => KeyData::PageUp,
        KeyCode::Home => KeyData::Home,
        KeyCode::End => KeyData::End,
        KeyCode::Left => KeyData::Left,
        KeyCode::Right => KeyData::Right,
        KeyCode::Tab => KeyData::Tab,
        KeyCode::Enter => KeyData::Enter,
        _ => panic!("Unsupported key"),
    }
}

fn from_serialisable(key: &KeyData) -> KeyCode {
    match key {
        KeyData::Char(c) => KeyCode::Char(*c),
        KeyData::Down => KeyCode::Down,
        KeyData::Up => KeyCode::Up,
        KeyData::PageDown => KeyCode::PageDown,
        KeyData::PageUp => KeyCode::PageUp,
        KeyData::Home => KeyCode::Home,
        KeyData::End => KeyCode::End,
        KeyData::Left => KeyCode::Left,
        KeyData::Right => KeyCode::Right,
        KeyData::Tab => KeyCode::Tab,
        KeyData::Enter => KeyCode::Enter,
    }
}
