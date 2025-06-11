pub mod tides;

use chrono::{Datelike, Local};

use log::{info, warn};
use std::time::Instant;
use tides::Tide;

use async_std::future;
use std::time::Duration as stdDuration;

#[derive(Debug)]
pub struct Stats {
    pub tides: Option<(Tide, Tide)>,
    pub moon_age: f64,
}

impl TryFrom<&str> for Tide {
    type Error = Box<dyn std::error::Error>;

    fn try_from(line: &str) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            return Err(format!("not 3 parts").into());
        }

        let time = parts[0].to_string();
        let tide_type = parts[2].trim();

        match tide_type {
            "bajamar" => Ok(Tide::Low(time)),
            "pleamar" => Ok(Tide::High(time)),
            _ => Err(format!("not 3 bajamar/pleamar").into()),
        }
    }
}

pub async fn fetch_stats() -> Result<Stats, Box<dyn std::error::Error>> {
    info!("Fetching statistics...");
    let now = Instant::now();

    let timeout = stdDuration::from_secs(25);

    let t = future::timeout(timeout, tides::fetch()).await;

    let t = match t {
        Ok(r) => r,
        Err(e) => Err(format!("Timeout: {e}").into()),
    };

    match &t {
        Ok(_) => {}
        Err(e) => warn!("Tides stats failed: {e}"),
    }

    let elapsed = format!("{:.2?}", now.elapsed());
    info!("Statistics took {elapsed}");

    let now = Local::now();
    let moon_age = get_moon_age(now.year(), now.month(), now.day());
    info!("moon age: {moon_age}");
    Ok(Stats {
        tides: t.ok(),
        moon_age,
    })
}

fn get_moon_age(year: i32, month: u32, day: u32) -> f64 {
    let astro_date = pracstro::time::Date::from_calendar(
        year as i64,
        month as u8,
        day as u8,
        pracstro::time::Angle::from_clock(12, 0, 0.0),
    );
    pracstro::moon::MOON.phaseage(astro_date)
}
