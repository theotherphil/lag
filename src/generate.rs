
use chrono::prelude::*;
use chrono::{Duration, Utc};
use std::io::{BufWriter, Write};

pub fn generate_log(path: &str, start: DateTime<Utc>, count: usize) {
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
