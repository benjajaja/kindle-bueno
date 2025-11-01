use crate::aemet::WindObservation;
use crate::aemet::{self, AemetPredictionDay};
use crate::tides;
use crate::tides::tides::Tide;
use crate::weather;

use crate::weather::DayData;

use image::{DynamicImage, ImageBuffer, Rgba};
use once_cell::sync::Lazy;
use resvg;
use tiny_skia::{PixmapMut, Transform, BYTES_PER_PIXEL};
use usvg::Tree;

use std::{env, future::Future, process::Command};

use base64::prelude::*;
use regex::Regex;
use std::io::Cursor;

use chrono::Timelike;
use std::time::Instant;

use futures::join;
use log::{info, warn};

use async_std::future;
use std::time::Duration as stdDuration;

#[derive(Debug)]
struct KindleDisplayData {
    short_stats: Option<tides::Stats>,
    weather: Option<Vec<weather::DayData>>,
    image: Option<DynamicImage>,
    wind: Option<WindObservation>,
    beach: Option<Vec<AemetPredictionDay>>,
}

async fn build_all_data() -> KindleDisplayData {
    info!("Fetching all data...");
    let now = Instant::now();

    let timeout = stdDuration::from_secs(30);
    let retries = 15;

    let (short_stats, weather, image, wind, beach) = join!(
        retry_with_timeout("Short stats", retries, timeout, || tides::fetch_tides()),
        retry_with_timeout("Weather", retries, timeout, || weather::fetch_weather()),
        retry_with_timeout("Radar", retries, timeout, || aemet::fetch_radar()),
        retry_with_timeout("Wind", retries, timeout, || aemet::fetch_wind_observation()),
        retry_with_timeout("Beach", retries, timeout, || aemet::fetch_beach_prediction(
        )),
    );

    let elapsed = format!("{:.2?}", now.elapsed());
    info!("Fetched all kindle data in {elapsed}");

    KindleDisplayData {
        short_stats: short_stats.ok(),
        weather: weather.ok(),
        image: image.ok(),
        wind: wind.ok(),
        beach: beach.ok(),
    }
}

async fn retry_with_timeout<T, F, Fut>(
    name: &str,
    max_retries: usize,
    timeout: stdDuration,
    mut operation: F,
) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, Box<dyn std::error::Error>>>,
{
    let mut last_error = None;

    for attempt in 1..=max_retries {
        match future::timeout(timeout, operation()).await {
            Ok(Ok(result)) => {
                if attempt > 1 {
                    info!("{} succeeded on attempt {}", name, attempt);
                }
                return Ok(result);
            }
            Ok(Err(e)) => {
                warn!("{} failed on attempt {}: {}", name, attempt, e);
                last_error = Some(format!("Operation error: {}", e));
            }
            Err(e) => {
                warn!("{} timed out on attempt {}: {}", name, attempt, e);
                last_error = Some(format!("Timeout: {}", e));
            }
        }

        if attempt < max_retries {
            const BASE_DELAY: u64 = 500;
            const MAX_DELAY: u64 = 60_000;
            let exponential_delay = BASE_DELAY * (2_u64.pow(attempt as u32 - 1)).min(MAX_DELAY);
            let delay = tokio::time::Duration::from_millis(exponential_delay); // Exponential backoff
            tokio::time::sleep(delay).await;
        }
    }

    let final_error = last_error.unwrap_or_else(|| "Unknown error".to_string());
    warn!("{} failed after {} attempts", name, max_retries);
    Err(final_error.into())
}

fn format_stats(template: String, data: &KindleDisplayData) -> String {
    let mut template = template.clone();
    match &data.short_stats {
        Some(short_stats) => {
            template = template.replace(
                "#I1a",
                &match &short_stats.tides {
                    Some((first, _)) => match first {
                        Tide::High(_) => format!("Pleamar"),
                        Tide::Low(_) => format!("Bajamar"),
                    },
                    None => "NA".to_string(),
                },
            );
            template = template.replace(
                "#I1b",
                &match &short_stats.tides {
                    Some((first, _)) => match first {
                        Tide::High(time) => format!("{time}"),
                        Tide::Low(time) => format!("{time}"),
                    },
                    None => "NA".to_string(),
                },
            );

            template = template.replace(
                "#I2a",
                &match &short_stats.tides {
                    Some((_, second)) => match second {
                        Tide::High(_) => format!("Pleamar"),
                        Tide::Low(_) => format!("Bajamar"),
                    },
                    None => "NA".to_string(),
                },
            );
            template = template.replace(
                "#I2b",
                &match &short_stats.tides {
                    Some((_, second)) => match second {
                        Tide::High(time) => format!("{time}"),
                        Tide::Low(time) => format!("{time}"),
                    },
                    None => "NA".to_string(),
                },
            );

            template = replace_image(template, "moon/1.svg", &moon_to_icon(short_stats.moon_age));

            // template = template.replace(
            // "#I4",
            // &match short_stats.linux_share {
            // Some(v) => format!("{:.2}%", v),
            // None => "NA".to_string(),
            // },
            // );
            //
            // template = template.replace(
            // "#I5",
            // &match short_stats.btc_halving {
            // Some(v) => time_remaining(v),
            // None => "NA".to_string(),
            // },
            // );
            //
            // template = template.replace(
            // "#I6",
            // &match short_stats.kernel_version.clone() {
            // Some(v) => v,
            // None => "NA".to_string(),
            // },
            // );
        }
        None => {
            template = template.replace("#I1", "ERR");
            template = template.replace("#I2", "ERR");
            template = template.replace("#I3", "ERR");
            template = template.replace("#I4", "ERR");
            template = template.replace("#I5", "ERR");
            template = template.replace("#I6", "ERR");
        }
    };

    return template;
}

fn format_time(template: String, _data: &KindleDisplayData) -> String {
    // We assume that making the primary requests take less than a minute to create the nice "every 15 minute" effect.

    let mut template = template.clone();

    // let now = chrono::offset::Utc::now() + Duration::hours(10);
    let now = chrono::offset::Utc::now().with_timezone(&chrono_tz::Atlantic::Canary);

    let hour = now.hour();
    let minute = now.minute();

    template = template.replace("#time", &format!("{:0>2}:{:0>2}", hour, minute));
    // template = template.replace("#2", &format!("{:0>2}", minute));
    return template;
}

static WEATHER_SVGS: Lazy<[&'static str; 8]> = Lazy::new(|| {
    [
        include_str!("icons/1.svg"),
        include_str!("icons/2.svg"),
        include_str!("icons/3.svg"),
        include_str!("icons/4.svg"),
        include_str!("icons/5.svg"),
        include_str!("icons/6.svg"),
        include_str!("icons/7.svg"),
        include_str!("icons/8.svg"),
    ]
});

fn weather_to_icon(day: &DayData) -> &'static str {
    let avg_rain = day.rain_sum / day.data_points as f64;
    let avg_cloud = day.cloud_sum / day.data_points as f64;

    let mut result = WEATHER_SVGS[0];

    if avg_cloud > 20.0 {
        result = WEATHER_SVGS[1];
    }
    if avg_cloud > 50.0 {
        result = WEATHER_SVGS[2];
    }
    if avg_cloud > 80.0 {
        result = WEATHER_SVGS[3];
    }

    if avg_rain > 0.1 {
        result = WEATHER_SVGS[4];
    }
    if avg_rain > 0.5 {
        result = WEATHER_SVGS[5];
    }
    if avg_rain > 1.0 {
        result = WEATHER_SVGS[6];
    }
    if avg_rain > 5.0 {
        result = WEATHER_SVGS[7];
    }

    result
}

static MOON_SVGS: Lazy<[&'static str; 8]> = Lazy::new(|| {
    [
        include_str!("moon/1.svg"),
        include_str!("moon/2.svg"),
        include_str!("moon/3.svg"),
        include_str!("moon/4.svg"),
        include_str!("moon/5.svg"),
        include_str!("moon/6.svg"),
        include_str!("moon/7.svg"),
        include_str!("moon/8.svg"),
    ]
});

// (0 = new moon, 0.5 = full moon)
fn moon_to_icon(age: f64) -> &'static str {
    const SYNODIC_MONTH: f64 = 29.53058868;
    const PHASE_WIDTH: f64 = SYNODIC_MONTH / 8.0;

    let shifted_age = (age + PHASE_WIDTH / 2.0).rem_euclid(SYNODIC_MONTH);
    let phase = (shifted_age / PHASE_WIDTH).floor() as u8;

    info!("rendering moon at step {phase}");
    MOON_SVGS[phase as usize]
}

fn format_weather(template: String, data: &KindleDisplayData) -> String {
    let mut template = template.clone();

    if let Some(beach) = &data.beach {
        for i in 0..3 {
            template = match beach.get(i) {
                Some(day) => {
                    let day_marker = i + 1;
                    template =
                        template.replace(&format!("#D{}-Day", day_marker), day.get_week_day());
                    template = template.replace(
                        &format!("#D{}-Wi-1", day_marker),
                        &format!("{}", day.wind.morning.to_str()),
                    );
                    template = template.replace(
                        &format!("#D{}-Wi-2", day_marker),
                        &format!("{}", day.wind.afternoon.to_str()),
                    );
                    template = template.replace(
                        &format!("#D{}-Wa-1", day_marker),
                        &format!("{}", day.waves.morning.to_str()),
                    );
                    template = template.replace(
                        &format!("#D{}-Wa-2", day_marker),
                        &format!("{}", day.waves.afternoon.to_str()),
                    );
                    template
                }
                None => {
                    template = template.replace("#D1", "NA");
                    template = template.replace("#T1", "NA");
                    template = template.replace("#T2", "NA");
                    template
                }
            }
        }
    }

    return template;
}

fn format_radar(template: String, data: &KindleDisplayData) -> String {
    let mut template = template.clone();
    match &data.image {
        Some(image) => {
            let mut buffer = Cursor::new(Vec::new());

            let r = image.write_to(&mut buffer, image::ImageFormat::Png);
            match r {
                Ok(_r) => {
                    let encoded_image = BASE64_STANDARD.encode(buffer.get_ref());
                    template = template
                        .replace("map.png", &format!("data:image/png;base64,{encoded_image}"));
                }
                Err(e) => {
                    warn!("Could not write to buffer: {e}")
                }
            }
        }
        None => {}
    };

    if let Some(wind) = &data.wind {
        template = template.replace("#wind", &format!("{:.1?}", wind.speed));
        template = template.replace(
            "rotate(45 1115 84)",
            &format!("rotate({:.0} 1100 60)", wind.direction + 180.0),
        );
        info!("Wind direction: {} deg", wind.direction);
    } else {
        template = template.replace("#wind", "?");
    }

    return template;
}

struct Screen {
    width: u32,
    height: u32,
}

fn get_screen_dim() -> Option<Screen> {
    // Run xrandr to get screen data
    let output = Command::new("xrandr").output();

    match output {
        Ok(output) => {
            let output_str = String::from_utf8_lossy(&output.stdout);

            // regex to get the current screen size
            let re = Regex::new(r"current (\d+) x (\d+)").expect("Failed to compile regex");

            if let Some(caps) = re.captures(&output_str) {
                let width = caps.get(1).map_or("", |m| m.as_str()).parse::<u32>();
                let height = caps.get(2).map_or("", |m| m.as_str()).parse::<u32>();

                match width {
                    Ok(width) => match height {
                        Ok(height) => Some(Screen { width, height }),
                        Err(e) => {
                            warn!("Could not determine screen size (height) from: {output_str} due to {e}");
                            None
                        }
                    },
                    Err(e) => {
                        warn!(
                            "Could not determine screen size (width) from: {output_str} due to {e}"
                        );
                        None
                    }
                }
            } else {
                None
            }
        }

        Err(e) => {
            warn!("Could not run xrandr to get screen size: {e}");
            None
        }
    }
}

fn create_output_svg(data: KindleDisplayData) -> String {
    let mut template = include_str!("template.svg").to_string();

    template = format_stats(template, &data);
    template = format_time(template, &data);
    template = format_weather(template, &data);
    template = format_radar(template, &data);

    template
}

async fn render_svg(template: String) -> DynamicImage {
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_font_data(include_bytes!("fonts/FreeSans.ttf").to_vec());
    fontdb.load_font_data(include_bytes!("fonts/FreeSansBold.ttf").to_vec());

    let mut options = usvg::Options::default();
    options.fontdb = std::sync::Arc::new(fontdb);

    let svg_tree = Tree::from_str(&template, &options).unwrap();

    let size = svg_tree.size();
    let (width, height) = (size.width() as usize, size.height() as usize);

    let mut image: Vec<u8> = vec![255; width * height * BYTES_PER_PIXEL];

    info!("Rendering the svg...");
    let now = Instant::now();
    resvg::render(
        &svg_tree,
        Transform::identity(),
        &mut PixmapMut::from_bytes(&mut image, size.width() as u32, size.height() as u32).unwrap(),
    );
    let elapsed = format!("{:.2?}", now.elapsed());
    info!("Rendering took {elapsed}");

    let img_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width as u32, height as u32, image).unwrap();
    let result = DynamicImage::ImageRgba8(img_buffer);
    return result;
}

async fn clear_screen() {
    Command::new("eips")
        .arg("-d")
        .arg("l=0,w=9999,h=9999")
        .output()
        .ok();
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    Command::new("eips").arg("-c").output().ok();
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    Command::new("eips")
        .arg("-d")
        .arg("l=0,w=9999,h=9999")
        .output()
        .ok();
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
}

pub async fn update_screen(path: String) -> Result<std::process::Output, std::io::Error> {
    clear_screen().await;
    Command::new("eips").arg("-g").arg(path).output()
}

pub async fn show_panic(panic: &String) -> Result<(), Box<dyn std::error::Error>> {
    // As minimal as possible to avoid any "dangerous" code
    if std::env::var("NOT_KINDLE").is_err() {
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        Command::new("eips").arg("-c").output().ok();
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        let output = Command::new("eips")
            .arg("2")
            .arg("1")
            .arg(format!("\"{panic}\""))
            .output();
        match output {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Could not show error: {e}").into()),
        }
    } else {
        info!("Skipping showing the panic due to env NOT_KINDLE");
        Ok(())
    }
}

pub fn save(mut image: DynamicImage) -> String {
    let output_path = "output.png".to_string();

    info!("Saving the rendering...");
    let now = Instant::now();

    let screen = get_screen_dim().unwrap_or_else(|| {
        warn!("Could not determine screen size, switching to 600x800");
        Screen {
            width: 600,
            height: 800,
        }
    });
    info!("screen width,height: {},{}", screen.width, screen.height);

    image = image.resize_exact(
        screen.height,
        screen.width,
        image::imageops::FilterType::Lanczos3,
    );
    if env::var("NOT_KINDLE").is_err() {
        image = image.rotate90();
    }
    let result: image::GrayImage = DynamicImage::ImageRgb8(image.into()).into_luma8();
    result.save(output_path.clone()).unwrap();
    let elapsed = format!("{:.2?}", now.elapsed());
    info!("Saving took {elapsed} {output_path}");
    output_path
}

pub async fn fetch_and_render() {
    let start = Instant::now();

    let data = build_all_data().await;

    let template = create_output_svg(data);
    let image = render_svg(template).await;
    let output_pth = save(image);
    let eips_result = update_screen(output_pth).await;

    match eips_result {
        Ok(_r) => {
            info!("Success! Now showing the result!")
        }
        Err(e) => warn!("Could not show result! Is eips available? {e}"), // Mainly for testing
    }

    let elapsed = format!("{:.2?}", start.elapsed());
    info!("Finished in {elapsed}");
}

pub fn replace_image(template: String, href: &str, tag_replacement: &str) -> String {
    // Escape the href to safely insert it in a regex
    let href_pattern = regex::escape(href);

    // Regex to match <image ... href="exact match" ... />
    let pattern = format!(r#"<image\b[^>]*?\bhref\s*=\s*"{}"[^>]*/?>"#, href_pattern);
    let re = Regex::new(&pattern).unwrap();

    re.replace_all(&template, tag_replacement).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_image_exact_href() {
        let input = r#"
            <svg>
                <image
                    href="./moon/1.svg"
                    id="quirky"
                />
            </svg>
        "#;

        let expected = r#"
            <svg>
                <path />
            </svg>
        "#;

        let output = replace_image(input.to_string(), "./moon/1.svg", r#"<path />"#);

        assert_eq!(output.trim(), expected.trim());
    }
}
