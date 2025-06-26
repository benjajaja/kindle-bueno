use log::info;
use std::process::Command;

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
