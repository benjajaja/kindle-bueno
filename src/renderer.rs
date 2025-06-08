use crate::radar;
use crate::stats;
use crate::stats::tides::Tide;
use crate::weather;

use crate::weather::DayData;

use image::{DynamicImage, ImageBuffer, Rgba};
use resvg;
use tiny_skia::{PixmapMut, Transform, BYTES_PER_PIXEL};
use usvg::Tree;

use std::{env, process::Command};

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
    short_stats: Option<stats::Stats>,
    weather: Option<Vec<weather::DayData>>,
    image: Option<DynamicImage>,
}

async fn build_all_data() -> KindleDisplayData {
    info!("Fetching all data...");
    let now = Instant::now();

    let timeout = stdDuration::from_secs(30);

    let short_stats = future::timeout(timeout, stats::fetch_stats());
    let weather = future::timeout(timeout, weather::fetch_weather());
    let image = future::timeout(timeout, radar::fetch_radar());

    let (short_stats, weather, image) = join!(short_stats, weather, image);

    let elapsed = format!("{:.2?}", now.elapsed());
    info!("Fetched all kindle data in {elapsed}");

    // Checking timeout messages
    let short_stats = match short_stats {
        Ok(r) => r,
        Err(e) => Err(format!("Timeout: {e}").into()),
    };
    let weather = match weather {
        Ok(r) => r,
        Err(e) => Err(format!("Timeout: {e}").into()),
    };
    let image = match image {
        Ok(r) => r,
        Err(e) => Err(format!("Timeout: {e}").into()),
    };

    // Warning on error
    match &short_stats {
        Ok(_) => {}
        Err(e) => warn!("Short stats failed: {e}"),
    }
    match &weather {
        Ok(_) => {}
        Err(e) => warn!("Weather failed: {e}"),
    }
    match &image {
        Ok(_) => {}
        Err(e) => warn!("Radar failed: {e}"),
    }

    KindleDisplayData {
        short_stats: short_stats.ok(),
        weather: weather.ok(),
        image: image.ok(),
    }
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

            template = replace_image(
                template,
                "moon/1.svg",
                &moon_to_icon(short_stats.moon_phase),
            );

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

fn weather_to_icon(day: &DayData) -> String {
    let icon1 = include_str!("icons/1.svg").to_string();
    let icon2 = include_str!("icons/2.svg").to_string();
    let icon3 = include_str!("icons/3.svg").to_string();
    let icon4 = include_str!("icons/4.svg").to_string();
    let icon5 = include_str!("icons/5.svg").to_string();
    let icon6 = include_str!("icons/6.svg").to_string();
    let icon7 = include_str!("icons/7.svg").to_string();
    let icon8 = include_str!("icons/8.svg").to_string();

    let avg_rain = day.rain_sum / day.data_points as f64;
    let avg_cloud = day.cloud_sum / day.data_points as f64;

    let mut result = icon1;

    if avg_cloud > 20.0 {
        result = icon2
    }
    if avg_cloud > 50.0 {
        result = icon3
    }
    if avg_cloud > 80.0 {
        result = icon4
    }

    if avg_rain > 0.1 {
        result = icon5
    }
    if avg_rain > 0.5 {
        result = icon6
    }
    if avg_rain > 1.0 {
        result = icon7
    }
    if avg_rain > 5.0 {
        result = icon8
    }

    result
}

// (0 = new moon, 0.5 = full moon)
fn moon_to_icon(phase: f64) -> String {
    let mut phases = [
        (0.0, include_str!("moon/1.svg").to_string()),
        (0.125, include_str!("moon/2.svg").to_string()),
        (0.25, include_str!("moon/3.svg").to_string()),
        (0.375, include_str!("moon/4.svg").to_string()),
        (0.5, include_str!("moon/5.svg").to_string()),
        (0.625, include_str!("moon/6.svg").to_string()),
        (0.75, include_str!("moon/7.svg").to_string()),
        (0.875, include_str!("moon/8.svg").to_string()),
    ];
    phases.reverse();
    // .reverse();

    let mut closest = phases[0].1.clone();
    let mut smallest_diff = 1.0;
    for &(p, ref url) in phases.iter() {
        let diff = (phase - p).abs();
        if diff < smallest_diff {
            smallest_diff = diff;
            closest = url.clone();
        }
    }
    closest
}

fn format_weather(template: String, data: &KindleDisplayData) -> String {
    let mut template = template.clone();

    match &data.weather {
        Some(weather) => {
            // Trust me, I'm not happy with this code either

            template = match weather.get(0) {
                Some(day) => {
                    template = template.replace("#D1", &format!("{:0>2} {}", day.date, day.day));
                    template = template.replace("#T1", &format!("{:.1}", day.max_c));
                    template = template.replace("#T2", &format!("{:.1}", day.min_c));
                    template = replace_image(template, "icons/1.svg", &&weather_to_icon(day));

                    template
                }
                None => {
                    template = template.replace("#D1", "NA");
                    template = template.replace("#T1", "NA");
                    template = template.replace("#T2", "NA");
                    template
                }
            };

            template = match weather.get(1) {
                Some(day) => {
                    template = template.replace("#D2", &format!("{:0>2} {}", day.date, day.day));
                    template = template.replace("#T3", &format!("{:.1}", day.max_c));
                    template = template.replace("#T4", &format!("{:.1}", day.min_c));
                    template = replace_image(template, "icons/2.svg", &&weather_to_icon(day));
                    template
                }
                None => {
                    template = template.replace("#D2", "NA");
                    template = template.replace("#T3", "NA");
                    template = template.replace("#T4", "NA");
                    template
                }
            };

            template = match weather.get(2) {
                Some(day) => {
                    template = template.replace("#D3", &format!("{:0>2} {}", day.date, day.day));
                    template = template.replace("#T5", &format!("{:.1}", day.max_c));
                    template = template.replace("#T6", &format!("{:.1}", day.min_c));
                    template = replace_image(template, "icons/3.svg", &&weather_to_icon(day));
                    template
                }
                None => {
                    template = template.replace("#D3", "NA");
                    template = template.replace("#T5", "NA");
                    template = template.replace("#T6", "NA");
                    template
                }
            };
        }
        None => {
            template = template.replace("#D1", "ERR");
            template = template.replace("#D2", "ERR");
            template = template.replace("#D3", "ERR");
            template = template.replace("#T1", "ERR");
            template = template.replace("#T2", "ERR");
            template = template.replace("#T3", "ERR");
            template = template.replace("#T4", "ERR");
            template = template.replace("#T5", "ERR");
            template = template.replace("#T6", "ERR");
        }
    };

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

async fn create_output_svg() -> String {
    let mut template = include_str!("template.svg").to_string();

    //let data = build_some_data().await;
    let data = build_all_data().await;

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

    let mut image: Vec<u8> = vec![0; width * height * BYTES_PER_PIXEL];

    info!("Rendering the svg...");
    let now = Instant::now();
    resvg::render(
        &svg_tree,
        Transform::identity(),
        &mut PixmapMut::from_bytes(&mut image, size.width() as u32, size.height() as u32).unwrap(),
    );
    let elapsed = format!("{:.2?}", now.elapsed());
    info!("Rendering took {elapsed}");

    let image_vec = image.to_vec();
    let img_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width as u32, height as u32, image_vec).unwrap();
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

pub async fn render_png() {
    let start = Instant::now();

    let template = create_output_svg().await;
    let image = render_svg(template).await;
    let output_pth = save(image.clone());
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
