use image::{self, ImageBuffer, ImageFormat, Luma};
use reqwest;
use serde::{de::DeserializeOwned, Deserialize, Deserializer};

use image::{DynamicImage, GenericImageView};

use reqwest::header::USER_AGENT;

use chrono::{Datelike, Local, NaiveDate, Timelike};
use log::{error, info};

use crate::include_sensitive;

#[derive(Deserialize, Debug)]
// This just gives us a unique link to the data we actually want.
struct AemetLinkRes {
    #[serde(rename = "estado")]
    status: u32,
    #[serde(rename = "datos")]
    data: String,
}

#[derive(Deserialize, Debug)]
struct AemetObservationRes {
    #[serde(rename = "vv")]
    wind_speed: f32,
    #[serde(rename = "dv")]
    wind_direction: f32,
}

pub async fn get_image(url: &str) -> Result<image::DynamicImage, String> {
    let client = reqwest::Client::new();

    let img_bytes = match client
        .get(url)
        .header(
            USER_AGENT,
            "Mozilla/5.0 (Android 4.4; Mobile; rv:41.0) Gecko/41.0 Firefox/41.0",
        )
        .send()
        .await
    {
        Ok(response) => match response.bytes().await {
            Ok(img_bytes) => img_bytes,
            Err(err) => return Err(format!("Failed to read response bytes: {}", err)),
        },
        Err(err) => return Err(format!("Failed to fetch image: {}", err)),
    };

    let image = match image::load_from_memory_with_format(&img_bytes, ImageFormat::Png) {
        Ok(img) => img,
        Err(err) => return Err(format!("Failed to load image: {}", err)),
    };

    Ok(image)
}

pub fn get_image_url() -> String {
    let now = Local::now();

    let year = now.year();
    let month = now.month();
    let day = now.day();
    let hour = now.hour();
    let rounded_hour = (hour / 3) * 3;
    let url = format!("https://www.aemet.es//imagenes_d/eltiempo/prediccion/mod_maritima/{year:04}{month:02}{day:02}00+0{rounded_hour:02}_aewam_can_martot.png");
    url
}

pub async fn fetch_radar() -> Result<DynamicImage, Box<dyn std::error::Error>> {
    let image_url = get_image_url();
    info!("Fetching AEMET radar image: {image_url}");
    let mut image1 = get_image(&image_url).await?;
    image1 = DynamicImage::ImageLuma8(remap_colors_to_grayscale_fuzzy(&image1));
    Ok(image1)
}

/// Computes squared Euclidean distance between two RGB colors
fn color_distance_sq(c1: (u8, u8, u8), c2: (u8, u8, u8)) -> u32 {
    let dr = c1.0 as i32 - c2.0 as i32;
    let dg = c1.1 as i32 - c2.1 as i32;
    let db = c1.2 as i32 - c2.2 as i32;
    (dr * dr + dg * dg + db * db) as u32
}

fn remap_colors_to_grayscale_fuzzy(img: &DynamicImage) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    let color_map = [
        ((205, 255, 255), 255), // light teal - brightest
        ((0, 0, 254), 242),     // new blue, between light teal and teal
        ((129, 243, 255), 230), // teal
        ((0, 255, 0), 190),     // green
        ((255, 255, 75), 160),  // yellow
        ((255, 218, 0), 140),   // light orange
        ((255, 181, 0), 120),   // orange
        ((255, 0, 0), 80),      // red
        ((231, 0, 129), 60),    // purple
        ((181, 0, 181), 40),    // dark purple
    ];

    let (width, height) = img.dimensions();
    let mut gray_img = ImageBuffer::new(width, height);

    for (x, y, pixel) in img.pixels() {
        let rgb = (pixel[0], pixel[1], pixel[2]);

        // Find nearest color in color_map by minimal distance
        let mut best_gray = 255;
        let mut best_dist = u32::MAX;
        for (col, gray_val) in &color_map {
            let dist = color_distance_sq(rgb, *col);
            if dist < best_dist {
                best_dist = dist;
                best_gray = *gray_val;
            }
        }

        gray_img.put_pixel(x, y, Luma([best_gray]));
    }

    gray_img
}

#[derive(Debug)]
pub struct WindObservation {
    pub speed: f32,
    pub direction: f32,
}

const AEMET_KEY: &str = include_sensitive!("/aemet_key");

pub async fn fetch_aemet_res<T: DeserializeOwned>(
    url: String,
) -> Result<T, Box<dyn std::error::Error>> {
    let url = format!("{url}?api_key={AEMET_KEY}");
    info!("Fetching AEMET data link");

    let client = reqwest::Client::new();

    let response = client.get(url).send().await?;
    let ares: AemetLinkRes = response.json().await?;

    if ares.status != 200 {
        error!("Bad status AEMET {}", ares.status);
        return Err(format!("aemet status {}", ares.status).into());
    }

    let url = ares.data;

    info!("Following AEMET data link url");
    let response = client.get(url).send().await.inspect_err(|err| {
        error!("Aemet request error: {err:?}");
    })?;

    let text = response.text().await?;

    let data: T = serde_json::from_str(&text).inspect_err(|err| {
        error!("Failed to parse response: {err:?}, body: {text}");
    })?;

    return Ok(data);
}

const WIND_STATION: &str = include_sensitive!("/aemet_station");

pub async fn fetch_wind_observation() -> Result<WindObservation, Box<dyn std::error::Error>> {
    info!("Fetching AEMET observation data station {WIND_STATION}");
    let data: Vec<AemetObservationRes> = fetch_aemet_res(format!(
        "https://opendata.aemet.es/opendata/api/observacion/convencional/datos/estacion/{WIND_STATION}"
    ))
    .await?;

    let Some(last) = data.last() else {
        return Err(format!("no aemet wind data").into());
    };
    Ok(WindObservation {
        speed: last.wind_speed,
        direction: last.wind_direction,
    })
}

#[derive(Deserialize, Debug)]
struct AemetPredictionRes {
    #[serde(rename = "prediccion")]
    prediction: AemetPrediction,
}

#[derive(Deserialize, Debug)]
struct AemetPrediction {
    #[serde(rename = "dia")]
    days: Vec<AemetPredictionDay>,
}

#[derive(Deserialize, Debug)]
pub struct AemetPredictionDay {
    #[serde(rename = "fecha")]
    pub date: i64,
    #[serde(rename = "viento")]
    pub wind: AemetPredictionDayWind,
    #[serde(rename = "oleaje")]
    pub waves: AemetPredictionDayWaves,
}

impl AemetPredictionDay {
    pub fn get_week_day(&self) -> &'static str {
        let date_str = format!("{:08}", self.date);
        let parsed = NaiveDate::parse_from_str(&date_str, "%Y%m%d").unwrap();

        match parsed.weekday() {
            chrono::Weekday::Mon => "Lunes",
            chrono::Weekday::Tue => "Martes",
            chrono::Weekday::Wed => "Miércoles",
            chrono::Weekday::Thu => "Jueves",
            chrono::Weekday::Fri => "Viernes",
            chrono::Weekday::Sat => "Sábado",
            chrono::Weekday::Sun => "Domingo",
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct AemetPredictionDayWind {
    #[serde(rename = "f1", deserialize_with = "deserialize_wind_force")]
    pub morning: Force,
    #[serde(rename = "f2", deserialize_with = "deserialize_wind_force")]
    pub afternoon: Force,
}

#[derive(Deserialize, Debug)]
pub struct AemetPredictionDayWaves {
    #[serde(rename = "f1", deserialize_with = "deserialize_wave_force")]
    pub morning: Force,
    #[serde(rename = "f2", deserialize_with = "deserialize_wave_force")]
    pub afternoon: Force,
}

#[derive(Debug)]
pub enum Force {
    Weak,
    Moderate,
    Strong,
}

impl Force {
    pub fn to_str(&self) -> &'static str {
        match self {
            Force::Weak => "Flojo",
            Force::Moderate => "Moderado",
            Force::Strong => "Fuerte",
        }
    }
}

fn deserialize_wind_force<'de, D>(deserializer: D) -> Result<Force, D::Error>
where
    D: Deserializer<'de>,
{
    match u32::deserialize(deserializer)? {
        210 => Ok(Force::Weak),
        220 => Ok(Force::Moderate),
        230 => Ok(Force::Strong),
        other => Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(other as u64),
            &"210, 220, or 230",
        )),
    }
}

fn deserialize_wave_force<'de, D>(deserializer: D) -> Result<Force, D::Error>
where
    D: Deserializer<'de>,
{
    match u32::deserialize(deserializer)? {
        310 => Ok(Force::Weak),
        320 => Ok(Force::Moderate),
        330 => Ok(Force::Strong),
        other => Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(other as u64),
            &"310, 320, or 330",
        )),
    }
}

const BEACH: &str = include_sensitive!("/aemet_prediction_beach");

pub async fn fetch_beach_prediction() -> Result<Vec<AemetPredictionDay>, Box<dyn std::error::Error>>
{
    info!("Fetching AEMET observation prediccion beach {BEACH}");
    let data: Vec<AemetPredictionRes> = fetch_aemet_res(format!(
        "https://opendata.aemet.es/opendata/api/prediccion/especifica/playa/{BEACH}"
    ))
    .await?;

    let Some(last) = data.into_iter().next() else {
        return Err(format!("no aemet data").into());
    };

    Ok(last.prediction.days)
}
