extern crate reqwest;
use std::str::FromStr;

use reqwest::header;

use chrono::{Local, NaiveTime, Utc};

fn get_date() -> String {
    let today = Utc::now().naive_utc();
    today.format("%Y%m%d").to_string()
}

pub async fn fetch() -> Result<(Tide, Tide), Box<dyn std::error::Error>> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "User-Agent",
        "Mozilla/5.0 (X11; Linux x86_64; rv:126.0) Gecko/20100101 Firefox/126.0".parse()?,
    );

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let date = get_date();

    let response = client
        .get(format!(
            "https://ideihm.covam.es/api-ihm/getmarea?request=gettide&id=53&date={date}"
        ))
        .headers(headers)
        .send()
        .await?;

    let response = response.error_for_status()?;
    let data: String = response.text().await?;

    let lines: Vec<&str> = data.lines().collect();
    let parsed: Vec<TideEntry> = lines.iter().filter_map(|&line| parse_line(line)).collect();
    get_two_tides(&parsed, Local::now().time())
}

#[derive(Debug, Clone)]
pub enum Tide {
    Low(String),
    High(String),
}

#[derive(Debug, Clone)]
struct TideEntry {
    time: NaiveTime,
    tide: Tide,
}

fn parse_line(line: &str) -> Option<TideEntry> {
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() < 3 {
        return None;
    }

    let time_str = parts[0].to_string();
    let time = NaiveTime::from_str(&time_str).ok()?;
    let tide_type = parts[2].trim();

    let tide = match tide_type {
        "bajamar" => Tide::Low(time_str.clone()),
        "pleamar" => Tide::High(time_str.clone()),
        _ => return None,
    };

    Some(TideEntry { time, tide })
}

fn get_two_tides(
    tides: &[TideEntry],
    ref_time: NaiveTime,
) -> Result<(Tide, Tide), Box<dyn std::error::Error>> {
    if tides.len() < 2 {
        return Err("Not enough tides overall".into());
    }

    // If the first tide is after ref_time, return first two tides
    if tides[0].time > ref_time {
        return Ok((tides[0].tide.clone(), tides[1].tide.clone()));
    }

    // Find the last tide <= ref_time
    let last_before_idx = tides.iter().rposition(|t| t.time <= ref_time);

    match last_before_idx {
        Some(idx) if idx + 1 < tides.len() => {
            Ok((tides[idx].tide.clone(), tides[idx + 1].tide.clone()))
        }
        Some(_) => {
            // If last tide before is the last tide, just return last two tides in list
            Ok((
                tides[tides.len() - 2].tide.clone(),
                tides[tides.len() - 1].tide.clone(),
            ))
        }
        None => {
            // No tide before ref_time (should not happen due to previous check), fallback to first two tides
            Ok((tides[0].tide.clone(), tides[1].tide.clone()))
        }
    }
}
