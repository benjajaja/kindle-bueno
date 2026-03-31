extern crate reqwest;
use std::str::FromStr;

use reqwest::header;

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::{Atlantic::Canary, Tz};

use crate::include_sensitive;

fn get_date() -> String {
    let today = Utc::now().naive_utc();
    today.format("%Y%m%d").to_string()
}

const STATION_ID: &str = include_sensitive!("/tide_station_id");

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

    // https://ideihm.covam.es/api-ihm/getmarea?request=getlist&format=txt
    let url = format!(
        "https://ideihm.covam.es/api-ihm/getmarea?request=gettide&id={STATION_ID}&date={date}"
    );
    let response = client.get(url).headers(headers).send().await?;

    let response = response.error_for_status()?;
    let data: String = response.text().await?;

    let lines: Vec<&str> = data.lines().collect();
    let date = Utc::now().naive_utc().date();
    let parsed: Vec<TideEntry> = lines
        .iter()
        .filter_map(|&line| parse_line(line, date))
        .collect();
    let now = Utc::now().with_timezone(&Canary);
    get_two_tides(&parsed, now)
}

#[derive(Debug, Clone)]
pub enum Tide {
    Low(String),
    High(String),
}

#[derive(Debug, Clone)]
struct TideEntry {
    time: DateTime<Tz>,
    tide: Tide,
}

fn parse_line(line: &str, date: NaiveDate) -> Option<TideEntry> {
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() < 3 {
        return None;
    }

    let time_str = parts[0].to_string();
    let naive_time = NaiveTime::from_str(&time_str).ok()?;

    // API returns times in UTC, convert to Canary timezone
    let naive_datetime = date.and_time(naive_time);
    let utc_time = Utc.from_utc_datetime(&naive_datetime);
    let canary_time = utc_time.with_timezone(&Canary);

    let tide_type = parts[2].trim();

    let tide = match tide_type {
        "bajamar" => Tide::Low(canary_time.format("%H:%M").to_string()),
        "pleamar" => Tide::High(canary_time.format("%H:%M").to_string()),
        _ => return None,
    };

    Some(TideEntry {
        time: canary_time,
        tide,
    })
}

fn get_two_tides(
    tides: &[TideEntry],
    ref_time: DateTime<Tz>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_two_tides() {
        let tides = vec![
            TideEntry {
                time: "2025-01-15T02:00:00+00:00".parse::<DateTime<Utc>>().unwrap().with_timezone(&Canary),
                tide: Tide::Low("02:00".to_string()),
            },
            TideEntry {
                time: "2025-01-15T08:00:00+00:00".parse::<DateTime<Utc>>().unwrap().with_timezone(&Canary),
                tide: Tide::High("08:00".to_string()),
            },
            TideEntry {
                time: "2025-01-15T14:00:00+00:00".parse::<DateTime<Utc>>().unwrap().with_timezone(&Canary),
                tide: Tide::Low("14:00".to_string()),
            },
        ];

        let ref_time = "2025-01-15T10:00:00+00:00".parse::<DateTime<Utc>>().unwrap().with_timezone(&Canary);
        let result = get_two_tides(&tides, ref_time);

        assert!(result.is_ok());
        let (first, second) = result.unwrap();

        match first {
            Tide::High(time) => assert_eq!(time, "08:00"),
            _ => panic!("Expected High tide"),
        }
        match second {
            Tide::Low(time) => assert_eq!(time, "14:00"),
            _ => panic!("Expected Low tide"),
        }
    }
}
