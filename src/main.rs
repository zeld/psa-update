use std::path::Path;
use std::vec::Vec;

use futures::future::try_join_all;

use anyhow::{anyhow, Context, Error, Result};

use clap::{crate_version, Arg, Command};

use log::debug;

use reqwest::Client;

use dialoguer::{Confirm, Input};
use indicatif::{DecimalBytes, MultiProgress};

use sysinfo::{System, SystemExt};

mod disk;
mod download;
mod psa;

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();

    let matches = Command::new("PSA firmware update.")
        .version(crate_version!())
        .about("CLI alternative to Peugeot/Citroën/Open update for NAC/RCC firmware updates, hopefully more robust. Supports for resume of downloads.")
        .arg(Arg::new("VIN")
            .help("Sets the VIN to check for update")
            .required(false)
            .index(1))
        .arg(Arg::new("map")
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
        .arg(Arg::new("non-interactive")
            .help("Sets non-interactive mode")
            .required(false)
            .long("non-interactive")
            .takes_value(false))
        .get_matches();

    let interactive = !matches.contains_id("non-interactive");
    let vin = matches.value_of("VIN");
    let vin = if vin.is_none() && interactive {
        prompt("Please enter VIN").ok()
    } else {
        vin.map(|v| v.to_string())
    };
    if vin.is_none() {
        return Err(anyhow!("No VIN provided"));
    }
    let vin = vin.unwrap();

    let map = matches.value_of("map");

    // TODO investigate compression such as gzip for faster download
    let client = Client::builder()
        .build()
        .with_context(|| format!("Failed to create HTTP client"))?;

    let update_response = psa::request_available_updates(&client, &vin, map).await?;

    if update_response.software.is_none() {
        println!("No update found");
        return Ok(());
    }

    let mut selected_updates: Vec<psa::SoftwareUpdate> = Vec::new();
    let mut total_update_size = 0_u64;

    for software in update_response.software.unwrap() {
        for update in &software.update {
            // A empty update can be sent by the server when there are no available update
            if !update.update_id.is_empty() {
                psa::print(&software, update);
                if !interactive || confirm("Download update?")? {
                    selected_updates.push(update.clone());
                    let update_size = match update.update_size.parse() {
                        Ok(size) => size,
                        Err(_) => {
                            debug!("Failed to parse update size: {}", update.update_size);
                            0
                        }
                    };
                    total_update_size = total_update_size + update_size;
                }
            }
        }
    }

    if selected_updates.is_empty() {
        return Ok(());
    }

    // Check available disk size
    let mut sys: System = System::new();
    sys.refresh_disks_list();
    sys.refresh_disks();
    let disk_space = disk::get_current_dir_available_space(&sys);
    disk_space.map(|s| if s < total_update_size {
        println!("Warning, not enough space on disk to proceed with download. Available disk space in current directory: {}", DecimalBytes(s))
    });

    let multi_progress = MultiProgress::new();

    // Download concurrently
    let downloads = selected_updates
        .iter()
        .map(|update| psa::download_update(&client, update, &multi_progress));

    let downloaded_updates: Vec<psa::DownloadedUpdate> = try_join_all(downloads).await?;

    if !interactive {
        println!("Extraction not supported in non-interactive mode for now, exiting");
        return Ok(());
    }

    if !confirm(
        "To proceed to extraction of update(s), please insert an empty USB disk formatted as FAT32. Continue?",
    )? {
        return Ok(());
    }

    // Listing available disks for extraction
    sys.refresh_disks_list();
    sys.refresh_disks();
    // TODO check destination available space.
    disk::print_disks(&sys);

    let destination = prompt("Location where to extract the update files (IMPORTANT: Should be the root of an EMPTY USB device formatted as FAT32)")?;
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
    //FIXME interact_text() should be used instead but there is currently a bug
    // on Windows that triggers an error when the user presses the Shift/AltGr keys
    // https://github.com/mitsuhiko/dialoguer/issues/128
    Ok(Input::new().with_prompt(message).interact()?)
}
