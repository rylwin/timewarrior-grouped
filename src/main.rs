use chrono::prelude::*;
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

#[derive(Debug)]
struct Setting {
    name: String,
    value: String,
}

impl Setting {
    pub fn value_to_date_string(&self) -> String {
        match timewarrior_datetime::parse(&self.value[..]) {
            Ok(datetime) => format!("{}", datetime.date_naive().format("%Y-%m-%d")),
            Err(_) => "Invalid date".into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Interval {
    id: usize,
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
    annotations: Vec<String>,
}

impl Data {
    pub fn report_title(&self) -> String {
        let start = self.find_setting("temp.report.start");
        let end = self.find_setting("temp.report.end");
        if start.is_some() && !start.unwrap().value.is_empty() && end.is_some() {
            format!(
                "{} - {}",
                start.unwrap().value_to_date_string(),
                end.unwrap().value_to_date_string(),
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
            let title = interval.tags.join(", ");
            let row = rows.iter_mut().find(|row| row.title == title);
            match row {
                Some(row) => {
                    row.duration = row.duration.checked_add(&interval.duration()).unwrap();
                }
                None => rows.push(GroupReportRow {
                    title,
                    duration: interval.duration(),
                    annotations: vec![],
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

fn main() {
    let data = get_data();
    println!("{}", data.report_title());
    let mut rows = data.grouped_report_rows();
    rows.sort_by_key(|r| r.duration);
    rows.iter().rev().for_each(|row| {
        println!(
            "{:>20} {:10} {:10}",
            row.title,
            row.duration.num_minutes(),
            row.duration.num_minutes() as f32 / 60.0
        );
    });
    // dbg!(&data);
    // dbg!(&data.find_setting("temp.report.start"));
}
