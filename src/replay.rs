//! Support for reading and writing lists of user inputs for replaying and perf testing.

use serde::{Deserialize, Serialize};
use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::PathBuf;
use termion::event::Key;

pub fn write_action_log(file: &PathBuf, keys: &[Key]) -> Result<(), failure::Error> {
    let mut file = File::create(file)?;
    let serialisable: Vec<_> = keys.iter().map(to_serialisable).collect();
    let serialised = serde_json::to_string(&serialisable).unwrap();
    write!(file, "{}", serialised)?;
    Ok(())
}

pub fn read_action_log(file: &PathBuf) -> Result<Vec<Key>, failure::Error> {
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
}

fn to_serialisable(key: &Key) -> KeyData {
    match key {
        Key::Char(c) => KeyData::Char(*c),
        Key::Down => KeyData::Down,
        Key::Up => KeyData::Up,
        Key::PageDown => KeyData::PageDown,
        Key::PageUp => KeyData::PageUp,
        Key::Home => KeyData::Home,
        Key::End => KeyData::End,
        Key::Left => KeyData::Left,
        Key::Right => KeyData::Right,
        _ => panic!("Unsupported key"),
    }
}

fn from_serialisable(key: &KeyData) -> Key {
    match key {
        KeyData::Char(c) => Key::Char(*c),
        KeyData::Down => Key::Down,
        KeyData::Up => Key::Up,
        KeyData::PageDown => Key::PageDown,
        KeyData::PageUp => Key::PageUp,
        KeyData::Home => Key::Home,
        KeyData::End => Key::End,
        KeyData::Left => Key::Left,
        KeyData::Right => Key::Right,
    }
}
