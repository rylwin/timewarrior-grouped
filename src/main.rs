use serde::{Deserialize, Serialize};
use std::io::*;

#[derive(Debug)]
struct Setting {
    name: String,
    value: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Interval {
    id: usize,
    start: String,
    end: String,
    tags: Vec<String>,
}

#[derive(Debug)]
struct Data {
    settings: Vec<Setting>,
    intervals: Vec<Interval>,
}

fn get_data() -> Data {
    let mut settings = vec![];
    let mut interval_lines = vec![];
    // let mut intervals = vec![];
    std::io::stdin().lines().for_each(|line| {
        let line = line.unwrap();
        // println!("{}", line);
        let separator_index = line.find(": ");
        if let Some(separator_index) = separator_index {
            let setting = Setting {
                name: line[0..separator_index].into(),
                value: line[(separator_index + 2)..].into(),
            };
            settings.push(setting);
        } else if line != "\n" {
            interval_lines.push(line);
        }
    });
    let intervals: Vec<Interval> = serde_json::from_str(&interval_lines.join("")).unwrap();
    Data {
        settings,
        intervals,
    }
}

fn main() {
    let data = get_data();
    dbg!(data);
}
