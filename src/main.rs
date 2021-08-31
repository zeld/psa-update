use std::fs;
use std::fs::File;
use std::io::Write;

use log::debug;

use anyhow::{anyhow, Context, Error, Result};

use clap::{App, Arg};

use reqwest::Client;

use serde::{Deserialize, Serialize};

use indicatif::HumanBytes;

use tar::Archive;

mod download;

//type Error = Box<dyn std::error::Error>;
//type Error = anyhow::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();

    let matches = App::new("PSA firmware update.")
        .version("0.0.1")
        .about("CLI alternative to Peugeot/Citroën/Open update for NAC/RCC firmware updates, hopefully more robust. Supports for resume of downloads.")
        .arg(
            Arg::with_name("VIN")
                .help("Sets the VIN to check for update")
                .required(true)
                .index(1),
        )
        .get_matches();

    let vin = matches.value_of("VIN").unwrap_or("");

    let client = Client::new();

    let update_response = request_available_updates(&client, vin).await?;

    if update_response.software.is_none() {
        println!("No update found");
        return Ok(());
    }

    for software in update_response.software.unwrap() {
        for update in software.update {
            // A empty update can be sent by the server when there are no available update
            if !update.update_id.is_empty() {
                println!("Firmware update available: {}", update.update_version);
                println!("\tRelease date: {}", update.update_date);
                let update_size: u64 = update.update_size.parse()?;
                println!("\tSize: {}", HumanBytes(update_size));
                if confirm_choice("Proceed with update")? {
                    let licence_filename =
                        download::download_file(&client, &update.license_url, true).await?;
                    let update_filename =
                        download::download_file(&client, &update.update_url, true).await?;

                    let destination_path = prompt("Location where to extract the update files (IMPORTANT: Should be the root of an EMPTY USB device formatted as FAT32): ")?;
                    if destination_path.is_empty() {
                        println!("No location, skipping extraction");
                    } else {
                        extract_firmware(&licence_filename, &update_filename, &destination_path)?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn confirm_choice(message: &str) -> Result<bool, Error> {
    let confirm_message = format!("{} ([Y]es/[N]o)? ", message);
    let input = prompt(&confirm_message)?;
    Ok(input.to_lowercase() == "y")
}

fn prompt(message: &str) -> Result<String, Error> {
    println!("{}", message);
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    // TODO use a crate to properly read from stdin and strip ending newline
    if let Some('\n') = input.chars().next_back() {
        input.pop();
    }
    if let Some('\r') = input.chars().next_back() {
        input.pop();
    }
    Ok(input)
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

#[derive(Debug, Serialize, Deserialize)]
struct Software {
    #[serde(rename = "softwareType")]
    software_type: String,
    #[serde(rename = "updateRequestResult")]
    update_request_result: String,
    #[serde(rename = "currentSoftwareVersion")]
    current_software_version: String,
    update: Vec<SoftwareUpdate>,
}

#[derive(Debug, Serialize, Deserialize)]
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

async fn request_available_updates(
    client: &reqwest::Client,
    vin: &str,
) -> Result<UpdateResponse, Error> {
    // Body for firmware update request
    // - ovip-int-firmware-version: Firmware update for Bosch NAC (Navigation Audio Connectée)
    // - rcc-firmware: Firmware update for Continental RCC (Radio Couleur Connectée)
    // Note: other software types exist for NAC maps: map-afr, map-alg, map-asia, map-eur, map-isr, map-latam, map-latam-chile, map-mea, map-oce, map-russia, map-taiwan
    let body = serde_json::json!({
        "vin": vin,
        "softwareTypes": [
            { "softwareType": "ovip-int-firmware-version" },
            { "softwareType": "rcc-firmware" }
        ]
    });

    let body_as_text = body.to_string();

    let request = client
        .post(UPDATE_URL)
        .header("Content-type", "application/json")
        .body(body_as_text)
        .build()?;

    debug!(
        "Sending request {:?} with body {:?}",
        request,
        request.body()
    );
    let response = client.execute(request).await?;

    debug!("Received response {:?}", response);

    let response_text = response.text().await?;
    debug!("Received reponse body {}", response_text);

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
fn extract_firmware(
    licence_filename: &str,
    firmware_filename: &str,
    destination_path: &str,
) -> Result<(), Error> {
    println!("Extracting update to {}", destination_path);
    // TODO check destination available space. Warn if not USB root folder, not empty, not FAT32

    debug!("Copying licence file");
    let licence_destination_path = format!("{}/license", destination_path);
    fs::create_dir(&licence_destination_path)
        .with_context(|| format!("Failed to create directory {}", licence_destination_path))?;
    let licence_destination = format!("{}/{}", licence_destination_path, licence_filename);
    fs::copy(licence_filename, licence_destination)?;

    debug!("Extracting firmware");
    let mut ar = Archive::new(
        File::open(firmware_filename)
            .with_context(|| format!("Failed to open firmware {}", firmware_filename))?,
    );
    ar.unpack(destination_path).with_context(|| {
        format!(
            "Failed to extract firware {} to {} ",
            firmware_filename, destination_path
        )
    })?;
    Ok(())
}
