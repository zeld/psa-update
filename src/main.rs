use std::fs;
use std::fs::File;
use std::vec::Vec;

use futures::future::try_join_all;

use log::debug;

use anyhow::{anyhow, Context, Error, Result};

use clap::{App, Arg};

use reqwest::Client;

use serde::{Deserialize, Serialize};

use console::Style;
use dialoguer::{Confirm, Input};
use indicatif::{HumanBytes, MultiProgress};

use tar::Archive;

mod download;

//type Error = Box<dyn std::error::Error>;
//type Error = anyhow::Error; <- currently in use

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();

    let matches = App::new("PSA firmware update.")
        .version("0.0.2")
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

    let update_response = request_available_updates(&client, vin, map).await?;

    if update_response.software.is_none() {
        println!("No update found");
        return Ok(());
    }

    let mut selected_updates: Vec<SoftwareUpdate> = Vec::new();

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

    // TODO check available disk space

    let multi_progress = MultiProgress::new();

    // Download concurrently
    let downloads = selected_updates
        .iter()
        .map(|update| download_update(&client, update, &multi_progress));

    let downloaded_updates: Vec<DownloadedUpdate> = try_join_all(downloads).await?;

    let destination_path = prompt("Location where to extract the update files (IMPORTANT: Should be the root of an EMPTY USB device formatted as FAT32): ")?;
    if destination_path.is_empty() {
        println!("No location, skipping extraction");
    } else {
        for update in downloaded_updates {
            extract_update(&update, &destination_path)
                .with_context(|| format!("Failed to extract update"))?;
        }
    }

    Ok(())
}

async fn download_update(
    client: &reqwest::Client,
    software_update: &SoftwareUpdate,
    multi_progress: &MultiProgress,
) -> Result<DownloadedUpdate, Error> {
    debug!("Downloading update {:?}", software_update);
    let license_filename = if software_update.license_url.is_empty() {
        None
    } else {
        Some(
            download::download_file(client, &software_update.license_url, multi_progress, false)
                .await?,
        )
    };
    let update_filename =
        download::download_file(client, &software_update.update_url, multi_progress, true).await?;
    Ok(DownloadedUpdate {
        license_filename,
        update_filename,
    })
}

fn confirm(message: &str) -> Result<bool, Error> {
    Ok(Confirm::new().with_prompt(message).interact()?)
}

fn prompt(message: &str) -> Result<String, Error> {
    Ok(Input::new().with_prompt(message).interact_text()?)
}

const UPDATE_URL: &str = "https://api.groupe-psa.com/applications/majesticf/v1/getAvailableUpdate?client_id=1eeecd7f-6c2b-486a-b59c-8e08fca81f54";

/*
Sample response:
{
    "requestResult": "OK",
    "installerURL": "https://majestic.mpsa.com/mjf00-web/rest/UpdateDownload?updateId\u003d000000001570806588\u0026uin\u003d00000000000000000000\u0026type\u003dfw",
    "vin": "xxx",
    "software": [{
        "softwareType": "map-eur",
        "updateRequestResult": "OK",
        "currentSoftwareVersion": "14.0.0-r0",
        "update": [{
            "updateId": "002315011610132966",
            "updateSize": "9875589120",
            "updateVersion": "20.0.0-r0",
            "updateDate": "2021-02-07 11:47:22.0",
            "updateURL": "http://download.tomtom.com/OEM/PSA/MAP/PSA_map-eur_20.0.0-r0-NAC_EUR_WAVE2.tar",
            "licenseURL": ""
        }]
    }, {
        "softwareType": "ovip-int-firmware-version",
        "updateRequestResult": "OK",
        "currentSoftwareVersion": "21.07.67.32_NAC-r0",
        "update": [{
            "updateId": "001315031613548831",
            "updateSize": "2730659840",
            "updateVersion": "21.08.87.32_NAC-r1",
            "updateDate": "2021-04-19 17:38:57.0",
            "updateURL": "https://majestic-web.mpsa.com/mjf00-web/rest/UpdateDownload?updateId\u003d001315031613548831\u0026uin\u003d0D011C0939D4EE8027F4\u0026type\u003dfw",
            "licenseURL": "https://majestic-web.mpsa.com/mjf00-web/rest/LicenseDownload?mediaVersion\u003d001315031613548831\u0026uin\u003d0D011C0939D4EE8027F4"
        }]
    }]
}
*/

#[derive(Debug, Serialize, Deserialize)]
struct UpdateResponse {
    #[serde(rename = "requestResult")]
    request_result: String,
    #[serde(rename = "installerURL")]
    installer_url: Option<String>,
    vin: String,
    software: Option<Vec<Software>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Software {
    #[serde(rename = "softwareType")]
    software_type: String,
    #[serde(rename = "updateRequestResult")]
    update_request_result: String,
    #[serde(rename = "currentSoftwareVersion")]
    current_software_version: String,
    update: Vec<SoftwareUpdate>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SoftwareUpdate {
    #[serde(rename = "updateId")]
    update_id: String,
    #[serde(rename = "updateSize")]
    update_size: String,
    #[serde(rename = "updateVersion")]
    update_version: String,
    #[serde(rename = "updateDate")]
    update_date: String,
    #[serde(rename = "updateURL")]
    update_url: String,
    #[serde(rename = "licenseURL")]
    license_url: String,
}

#[derive(Debug)]
struct DownloadedUpdate {
    license_filename: Option<String>,
    update_filename: String,
}

async fn request_available_updates(
    client: &reqwest::Client,
    vin: &str,
    map: Option<&str>,
) -> Result<UpdateResponse, Error> {
    // Body for firmware update request
    // - ovip-int-firmware-version: Firmware update for Bosch NAC (Navigation Audio Connectée)
    // - rcc-firmware: Firmware update for Continental RCC (Radio Couleur Connectée)
    // Note: other software types exist for NAC maps: map-afr, map-alg, map-asia, map-eur, map-isr, map-latam, map-latam-chile, map-mea, map-oce, map-russia, map-taiwan
    let body = if map == None {
        serde_json::json!({
            "vin": vin,
            "softwareTypes": [
                { "softwareType": "ovip-int-firmware-version" },
                { "softwareType": "rcc-firmware" }
            ]
        })
    } else {
        serde_json::json!({
            "vin": vin,
            "softwareTypes": [
                { "softwareType": "ovip-int-firmware-version" },
                { "softwareType": "rcc-firmware" },
                { "softwareType": format!("map-{}", map.unwrap()) }
            ]
        })
    };

    let body_as_text = body.to_string();

    let request = client
        .post(UPDATE_URL)
        .header("Content-type", "application/json")
        .body(body_as_text)
        .build()
        .with_context(|| format!("Failed to build update request"))?;

    debug!("Sending request {:?} with body {:?}", request, body);
    let response = client.execute(request).await?;

    debug!("Received response {:?}", response);

    let response_text = response.text().await?;
    debug!("Received response body {}", response_text);

    let update_response: UpdateResponse = serde_json::from_str(&response_text)
        .with_context(|| format!("Failed to parse response"))?;

    if update_response.request_result != "OK" {
        Err(anyhow!(
            "Failed to retrieve available updates, received an error from server: {}",
            update_response.request_result
        ))
    } else {
        Ok(update_response)
    }
}

// Extract firmware update to specified location
fn extract_update(update: &DownloadedUpdate, destination_path: &str) -> Result<(), Error> {
    println!("Extracting update to {}", destination_path);
    // TODO check destination available space. Warn if not USB root folder, not empty, not FAT32

    if update.license_filename != None {
        debug!("Copying licence file");
        let licence_destination_path = format!("{}/license", destination_path);
        fs::create_dir(&licence_destination_path)
            .with_context(|| format!("Failed to create directory {}", licence_destination_path))?;
        let licence_destination = format!(
            "{}/{}",
            licence_destination_path,
            update.license_filename.as_ref().unwrap()
        );
        fs::copy(
            update.license_filename.as_ref().unwrap(),
            licence_destination,
        )?;
    }

    debug!("Extracting update");
    let mut ar = Archive::new(
        File::open(&update.update_filename)
            .with_context(|| format!("Failed to open firmware {}", update.update_filename))?,
    );
    ar.unpack(destination_path).with_context(|| {
        format!(
            "Failed to extract update {} to {} ",
            update.update_filename, destination_path
        )
    })?;
    Ok(())
}
