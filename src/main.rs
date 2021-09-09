use std::path::Path;
use std::vec::Vec;

use futures::future::try_join_all;

use anyhow::{anyhow, Context, Error, Result};

use clap::{crate_version, App, Arg};

use reqwest::Client;

use console::Style;
use dialoguer::{Confirm, Input};
use indicatif::{HumanBytes, MultiProgress};

mod download;
mod psa;

//type Error = Box<dyn std::error::Error>;
//type Error = anyhow::Error; <- currently in use

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();

    let matches = App::new("PSA firmware update.")
        .version(crate_version!())
        .about("CLI alternative to Peugeot/Citroën/Open update for NAC/RCC firmware updates, hopefully more robust. Supports for resume of downloads.")
        .arg(Arg::with_name("VIN")
            .help("Sets the VIN to check for update")
            .required(true)
            .index(1))
        .arg(Arg::with_name("map")
            .help("Sets the map to check for update. Supported maps:\n\
                - afr: Africa\n\
                - alg: Algeria\n\
                - asia: Asia\n\
                - eur: Europe\n\
                - isr: Israel\n\
                - latam: Latin America\n\
                - latam-chile: Latin America Chile\n\
                - mea: Middle East\n\
                - oce: Oceania\n\
                - russia: Russia\n\
                - taiwan: Taiwan")
            .required(false)
            .long("map")
            .takes_value(true))
        .get_matches();

    let vin = matches.value_of("VIN").expect("VIN is missing");
    let map = matches.value_of("map");

    let client = Client::new();

    let update_response = psa::request_available_updates(&client, vin, map).await?;

    if update_response.software.is_none() {
        println!("No update found");
        return Ok(());
    }

    let mut selected_updates: Vec<psa::SoftwareUpdate> = Vec::new();

    for software in update_response.software.unwrap() {
        for update in &software.update {
            // A empty update can be sent by the server when there are no available update
            if !update.update_id.is_empty() {
                let cyan = Style::new().cyan();
                println!(
                    "Update available: {}",
                    cyan.apply_to(&update.update_version)
                );
                let software_type = if software.software_type.starts_with("map") {
                    "Map "
                } else {
                    "Firmware"
                };
                println!("\tType: {}", cyan.apply_to(&software_type));
                println!("\tRelease date: {}", cyan.apply_to(&update.update_date));
                let update_size: u64 = update.update_size.parse().with_context(|| {
                    format!("Failed to parse update size: {}", update.update_size)
                })?;
                println!("\tSize: {}", cyan.apply_to(HumanBytes(update_size)));
                println!("\tURL: {}", cyan.apply_to(&update.update_url));
                if !update.license_url.is_empty() {
                    println!("\tLicense URL: {}", cyan.apply_to(&update.license_url));
                }
                if confirm("Download update?")? {
                    selected_updates.push(update.clone());
                }
            }
        }
    }

    if selected_updates.is_empty() {
        return Ok(());
    }

    let multi_progress = MultiProgress::new();

    // Download concurrently
    let downloads = selected_updates
        .iter()
        .map(|update| psa::download_update(&client, update, &multi_progress));

    let downloaded_updates: Vec<psa::DownloadedUpdate> = try_join_all(downloads).await?;

    let destination = prompt("Location where to extract the update files (IMPORTANT: Should be the root of an EMPTY USB device formatted as FAT32): ")?;
    if destination.is_empty() {
        println!("No location, skipping extraction");
    } else {
        let destination_path = Path::new(&destination);
        if !destination_path.is_dir() {
            return Err(anyhow!(
                "Destination does not exist or is not a directory: {}",
                destination_path.to_string_lossy()
            ));
        }
        // TODO check destination available space. Warn if not USB root folder, not empty, not FAT32

        for update in downloaded_updates {
            psa::extract_update(&update, destination_path)
                .with_context(|| format!("Failed to extract update"))?;
        }
    }

    Ok(())
}

fn confirm(message: &str) -> Result<bool, Error> {
    Ok(Confirm::new().with_prompt(message).interact()?)
}

fn prompt(message: &str) -> Result<String, Error> {
    Ok(Input::new().with_prompt(message).interact_text()?)
}
