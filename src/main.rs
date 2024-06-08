use std::path::Path;
use std::vec::Vec;

use futures::future::try_join_all;

use anyhow::{anyhow, Context, Error, Result};

use clap::{crate_version, Arg, Command};

use log::debug;

use reqwest::Client;

use indicatif::{DecimalBytes, MultiProgress};

mod disk;
mod download;
mod interact;
mod psa;

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();

    let mut map_info = "Sets the map to check for update. Supported maps:".to_string();
    for map in psa::MAPS {
        map_info = format!("{}\n - {}: {}", map_info, map.get_code(), map.get_name());
    }

    let matches = Command::new("PSA firmware update.")
        .version(crate_version!())
        .about("CLI alternative to Peugeot/Citroën/Open update for NAC/RCC firmware updates, hopefully more robust. Supports for resume of downloads.")
        .arg(Arg::new("VIN")
            .help("Vehicle Identification Number (VIN) to check for update")
            .required(false)
            .index(1))
        .arg(Arg::new("map")
            .help(&*map_info)
            .required(false)
            .long("map")
            .takes_value(true))
        .arg(Arg::new("silent")
            .help("Sets silent (non-interactive) mode")
            .required(false)
            .long("silent")
            .takes_value(false))
        .arg(Arg::new("extract")
            .help("Location where yo extract update files. Should be the root of an empty FAT32 USB drive.")
            .required(false)
            .long("extract")
            .takes_value(true))
        .get_matches();

    let interactive = !matches.contains_id("silent");
    let vin = matches.value_of("VIN").map(|s| s.to_uppercase());
    let vin_provided_as_arg = vin.is_some();
    let map = matches.value_of("map");

    // Vin not provided on command line, asking interactively
    let vin = if !vin_provided_as_arg && interactive {
        interact::prompt("Please enter VIN").ok()
    } else {
        vin.map(|v| v.to_string())
    };
    if vin.is_none() {
        return Err(anyhow!("No VIN provided"));
    }
    let vin = vin.unwrap();

    let client = Client::builder()
        .build()
        .with_context(|| format!("Failed to create HTTP client"))?;
    let device_info = psa::request_device_information(&client, &vin).await?;
    let is_nac: bool = device_info
        .devices
        .map(|l| l.iter().any(|d| d.ecu_type.contains("NAC")))
        == Some(true);

    // Maps not provided on command line, asking interactively for NAC
    let map = if map.is_none() && is_nac && interactive {
        interact::select_map()?
    } else {
        map
    };

    let extract_location = matches.value_of("extract");

    // TODO investigate compression such as gzip for faster download
    let update_response = psa::request_available_updates(&client, &vin, map).await?;

    if update_response.software.is_none() {
        println!("No update found");
        return Ok(());
    }

    let mut selected_updates: Vec<psa::SoftwareUpdate> = Vec::new();
    let mut total_update_size = 0_u64;

    for software in update_response.software.unwrap() {
        for update in &software.update {
            // An empty update can be sent by the server when there is no available update
            if !update.update_id.is_empty() {
                psa::print(&software, update);
                if !interactive || interact::confirm("Download update?")? {
                    selected_updates.push(update.clone());
                    let update_size = match update.update_size.parse() {
                        Ok(size) => size,
                        Err(_) => {
                            debug!("Failed to parse update size: {}", update.update_size);
                            0
                        }
                    };
                    total_update_size += update_size;
                }
            }
        }
    }

    if selected_updates.is_empty() {
        println!("No update to download");
        return Ok(());
    }

    // Check available disk size
    let disk_space = disk::get_current_dir_available_space();
    if let Some(space) = disk_space {
        if space < total_update_size {
            interact::warn(&format!("Not enough space on disk to proceed with download. Available disk space in current directory: {}",
                                   DecimalBytes(space)));
            if interactive && !(interact::confirm("Continue anyway?")?) {
                return Ok(());
            }
        }
    }

    let multi_progress = MultiProgress::new();

    // Download concurrently
    let downloads = selected_updates
        .iter()
        .map(|update| psa::download_update(&client, update, &multi_progress));

    let downloaded_updates: Vec<psa::DownloadedUpdate> = try_join_all(downloads).await?;

    let mut extract_location = extract_location.map(str::to_string);
    if interactive && extract_location.is_none() {
        if !interact::confirm(
            "To proceed to extraction of update(s), please insert an empty USB disk formatted as FAT32. Continue?",
        )? {
            return Ok(());
        }

        // Listing available disks for extraction
        // TODO check destination available space.
        disk::print_disks();
        let location = interact::prompt("Location where to extract the update files (IMPORTANT: Should be the root of an EMPTY USB device formatted as FAT32)")?;
        if !location.is_empty() {
            extract_location = Some(location);
        }
    }

    match extract_location {
        Some(location) => {
            let destination_path = Path::new(&location);
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
        None => {
            println!("No location, skipping extraction");
        }
    }

    Ok(())
}
