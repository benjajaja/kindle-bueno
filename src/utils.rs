use log::{info, warn};
use std::{process::Command, time::Duration};
use tokio::time::sleep;

pub fn check_xrandr() -> Result<(), String> {
    let output = Command::new("xrandr").output();

    match output {
        Ok(_r) => {
            info!("Found xrandr!");
            Ok(())
        }
        Err(e) => Err(format!("Could not find xrandr: {e}")),
    }
}

pub fn check_eips() -> Result<(), String> {
    // eips MUST have at least one argument or it "fails"
    let output = Command::new("eips").arg("-c").output();

    match output {
        Ok(_r) => {
            info!("Found eips!");
            Ok(())
        }
        Err(e) => Err(format!("Could not find eips: {e}")),
    }
}

pub async fn check_internet(client: &reqwest::Client) -> bool {
    info!("Checking for internet...");
    match client
        .get("https://www.google.com/generate_204")
        .send()
        .await
    {
        Ok(response) if response.status() == 204 => true,
        _ => false,
    }
}

pub async fn check_internet_with_retries(max_retries: u32, delay: Duration) -> Result<(), String> {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            return Err(format!("Could not build reqwest client"));
        }
    };
    for _ in 0..max_retries {
        if check_internet(&client).await {
            return Ok(());
        }
        warn!("No internet, retry in {delay:?}...");
        let _ = sleep(delay).await;
    }
    Err(format!("No internet after {max_retries} retries"))
}
