mod linear_rg;

mod halving;
mod linux;
mod linux_version;
mod spx;
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
    pub moon_phase: f64,
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
    Ok(Stats {
        tides: t.ok(),
        moon_phase: get_moon_phase_fraction(now.year(), now.month(), now.day()),
    })
}

fn get_moon_phase_fraction(year: i32, month: u32, day: u32) -> f64 {
    // This is a simplified version based on Conway's algorithm
    let mut r = year % 100;
    r %= 19;
    if r > 9 {
        r -= 19;
    }
    let mut t = ((r * 11) as i32 + month as i32 + day as i32) % 30;
    if t < 0 {
        t += 30;
    }
    (t as f64) / 29.53  // approximate synodic month length
}
