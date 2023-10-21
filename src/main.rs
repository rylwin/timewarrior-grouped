use chrono::prelude::*;
use colored::*;
use serde::Deserialize;

mod timewarrior_datetime {
    use chrono::{DateTime, Local, NaiveDateTime, ParseResult};
    use serde::{self, Deserialize, Deserializer};

    const FORMAT: &str = "%Y%m%dT%H%M%SZ";

    pub fn parse(s: &str) -> ParseResult<DateTime<Local>> {
        let dt = NaiveDateTime::parse_from_str(s, FORMAT)?;
        Ok(DateTime::<Local>::from_naive_utc_and_offset(
            dt,
            *Local::now().offset(),
        ))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
    where
        D: Deserializer<'de>,
    {
        parse(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

fn date_time_to_date_string(datetime: DateTime<Local>) -> String {
    datetime.date_naive().format("%Y-%m-%d").to_string()
}

#[derive(Debug)]
struct Setting {
    name: String,
    value: String,
}

impl Setting {
    pub fn value_to_date_time(&self) -> DateTime<Local> {
        timewarrior_datetime::parse(&self.value[..]).unwrap()
    }
}

#[derive(Debug, Deserialize)]
struct Interval {
    #[serde(with = "timewarrior_datetime")]
    start: DateTime<Local>,
    #[serde(with = "timewarrior_datetime")]
    end: DateTime<Local>,
    tags: Vec<String>,
    annotation: Option<String>,
}

impl Interval {
    pub fn duration(&self) -> chrono::Duration {
        self.end.signed_duration_since(self.start)
    }

    pub fn title(&self) -> String {
        self.tags.join(", ")
    }
}

#[derive(Debug)]
struct Data {
    settings: Vec<Setting>,
    intervals: Vec<Interval>,
}

#[derive(Debug)]
struct GroupReportRow {
    title: String,
    duration: chrono::Duration,
}

fn pad_string(s: &str, len: usize) -> String {
    match len.checked_sub(s.len()) {
        Some(padding) => {
            let mut padded_string = String::with_capacity(len);
            for _ in 0..padding {
                padded_string.push(' ');
            }
            padded_string.push_str(s);
            padded_string
        }
        None => s.to_string(),
    }
}

impl GroupReportRow {
    pub fn padded_title(&self, len: usize) -> String {
        pad_string(&self.title, len)
    }
}

impl Data {
    pub fn report_title(&self) -> String {
        let start = self.find_setting("temp.report.start");
        let end = self.find_setting("temp.report.end");
        if start.is_some() && !start.unwrap().value.is_empty() && end.is_some() {
            format!(
                "{} - {}",
                date_time_to_date_string(start.unwrap().value_to_date_time()),
                date_time_to_date_string(
                    end.unwrap()
                        .value_to_date_time()
                        .checked_sub_signed(chrono::Duration::seconds(1))
                        .unwrap()
                ),
            )
        } else {
            String::from("")
        }
    }

    pub fn find_setting(&self, name: &str) -> Option<&Setting> {
        self.settings.iter().find(|setting| setting.name == name)
    }

    pub fn grouped_report_rows(&self) -> Vec<GroupReportRow> {
        let mut rows: Vec<GroupReportRow> = vec![];
        self.intervals.iter().for_each(|interval| {
            let title = interval.title();
            let row = rows.iter_mut().find(|row| row.title == title);
            match row {
                Some(row) => {
                    row.duration = row.duration.checked_add(&interval.duration()).unwrap();
                }
                None => rows.push(GroupReportRow {
                    title,
                    duration: interval.duration(),
                }),
            };
        });
        rows
    }
}

fn get_data() -> Data {
    let mut settings = vec![];
    let mut interval_lines = vec![];
    std::io::stdin().lines().for_each(|line| {
        let line = line.unwrap();
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

const MINIMUM_TAGS_WIDTH: usize = 12;

fn main() {
    colored::control::set_override(true);

    let data = get_data();
    println!("{}", data.report_title().dimmed());
    println!();
    let mut rows = data.grouped_report_rows();
    let mut lengths = rows
        .iter()
        .map(|row| row.title.len())
        .collect::<Vec<usize>>();
    lengths.extend([MINIMUM_TAGS_WIDTH].iter());
    let max_title = lengths.into_iter().max().unwrap_or(0);
    let mut total_duration = chrono::Duration::zero();
    rows.iter().for_each(|row| {
        total_duration = total_duration.checked_add(&row.duration).unwrap();
    });

    println!(
        "{}",
        format!(
            "{} {:>10} {:>10} {:>5}",
            pad_string("TAGS", max_title),
            "MINUTES",
            "HOURS",
            "%"
        )
        .bold()
        .underline()
    );

    rows.sort_by_key(|r| r.duration);
    let mut it = rows.iter().rev().peekable();
    while let Some(row) = it.next() {
        let mut string: ColoredString = format!(
            "{} {:>10} {:10.1} {:5.0}",
            row.padded_title(max_title),
            row.duration.num_minutes(),
            row.duration.num_seconds() as f64 / 3600.0,
            row.duration.num_minutes() as f64 / (total_duration.num_minutes() as f64) * 100.0,
        )
        .normal();
        if it.peek().is_none() {
            string = string.underline();
        }
        println!("{}", string);
    }
    println!(
        "{}",
        format!(
            "{} {:>10} {:10.1}",
            pad_string("TOTAL", max_title),
            total_duration.num_minutes(),
            total_duration.num_seconds() as f64 / 3600.0,
        )
        .bold()
    );

    let annotated_intervals: Vec<&Interval> = data
        .intervals
        .iter()
        .filter(|interval| interval.annotation.is_some())
        .collect();
    if !annotated_intervals.is_empty() {
        println!();
        println!("{}", "annotations".dimmed());
        annotated_intervals.iter().for_each(|interval| {
            let string = format!(
                "{} {:>10} {:10.1} {:5.0} {}",
                pad_string(&interval.title(), max_title),
                interval.duration().num_minutes(),
                interval.duration().num_seconds() as f64 / 3600.0,
                interval.duration().num_minutes() as f64 / (total_duration.num_minutes() as f64)
                    * 100.0,
                interval.annotation.as_ref().unwrap(),
            );
            println!("{}", string.dimmed());
        });
    }
}
