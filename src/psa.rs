use std::fs;
use std::fs::File;
use std::path::Path;
use std::str;

use serde::{Deserialize, Serialize};

use log::debug;

use anyhow::{Context, Error, Result, anyhow};

use console::Style;

use indicatif::{DecimalBytes, MultiProgress};

use tar::Archive;

use crate::download;

// URL to query vehicle device: NAC or RCC
const DEVICE_URL: &str = "https://api.groupe-psa.com/applications/majesticf/v1/devices/{VIN}?client_id=20a4cf7c-f5fb-41d5-9175-a6e23b9880e5";

// URL to query for firmware/map updates. The client_id below was extracted from the official Peugeot Update software
const UPDATE_URL: &str = "https://api.groupe-psa.com/applications/majesticf/v1/getAvailableUpdate?client_id=20a4cf7c-f5fb-41d5-9175-a6e23b9880e5";

pub struct Map<'a> {
    code: &'a str,
    name: &'a str,
}

impl<'a> Map<'a> {
    pub fn get_name(&self) -> &str {
        self.name
    }
    pub fn get_code(&self) -> &str {
        self.code
    }
}

// List of maps sorted by name
pub const MAPS: &[Map] = &[
    Map {
        code: "afr",
        name: "Africa",
    },
    Map {
        code: "alg",
        name: "Algeria",
    },
    Map {
        code: "asia",
        name: "Asia",
    },
    Map {
        code: "eur",
        name: "Europe",
    },
    Map {
        code: "isr",
        name: "Israel",
    },
    Map {
        code: "latam",
        name: "Latin America",
    },
    Map {
        code: "latam-chile",
        name: "Latin America Chile",
    },
    Map {
        code: "mea",
        name: "Middle East",
    },
    Map {
        code: "oce",
        name: "Oceania",
    },
    Map {
        code: "russia",
        name: "Russia",
    },
    Map {
        code: "taiwan",
        name: "Taiwan",
    },
];

/*
Sample device response: {"vin":"xxx","requestDate":"2022-09-24T14:05:03+0200","devices":[{"ecuType":"NAC_EUR_WAVE2"}]}
 */
#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceResponse {
    pub vin: String,
    #[serde(rename = "requestDate")]
    pub request_date: Option<String>,
    pub devices: Option<Vec<Device>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    #[serde(rename = "ecuType")]
    pub ecu_type: String,
}

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
pub struct UpdateResponse {
    #[serde(rename = "requestResult")]
    pub request_result: String,
    #[serde(rename = "installerURL")]
    pub installer_url: Option<String>,
    pub vin: String,
    pub software: Option<Vec<Software>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Software {
    #[serde(rename = "softwareType")]
    pub software_type: String,
    #[serde(rename = "updateRequestResult")]
    pub update_request_result: String,
    #[serde(rename = "currentSoftwareVersion")]
    pub current_software_version: String,
    pub update: Vec<SoftwareUpdate>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SoftwareUpdate {
    #[serde(rename = "updateId")]
    pub update_id: String,
    #[serde(rename = "updateSize")]
    pub update_size: String,
    #[serde(rename = "updateVersion")]
    pub update_version: String,
    #[serde(rename = "updateDate")]
    pub update_date: String,
    #[serde(rename = "updateURL")]
    pub update_url: String,
    #[serde(rename = "licenseURL")]
    pub license_url: String,
}

#[derive(Debug)]
pub struct DownloadedUpdate {
    pub license_filename: Option<String>,
    pub update_filename: String,
}

pub fn print(software: &Software, update: &SoftwareUpdate) {
    let cyan = Style::new().cyan();
    let software_type = if software.software_type.starts_with("map") {
        "Map "
    } else {
        "Firmware"
    };
    println!(
        "Update available - Type: {:<9} Version: {}",
        &cyan.apply_to(software_type),
        cyan.apply_to(&update.update_version)
    );

    let update_size = match update.update_size.parse() {
        Ok(size) => Some(size),
        Err(_) => {
            debug!("Failed to parse update size: {}", update.update_size);
            None
        }
    };
    let update_size_formatted = update_size
        .map(|s| DecimalBytes(s).to_string())
        .unwrap_or_else(|| "?".to_string());
    println!(
        "                   Size: {:<9} Release date: {}",
        cyan.apply_to(update_size_formatted),
        cyan.apply_to(&update.update_date)
    );

    println!(
        "                   URL: {}",
        cyan.apply_to(&update.update_url)
    );
    if !update.license_url.is_empty() {
        println!(
            "                   License URL: {}",
            cyan.apply_to(&update.license_url)
        );
    }
}

pub async fn request_device_information(
    client: &reqwest::Client,
    vin: &str,
) -> Result<DeviceResponse, Error> {
    let request = client
        .get(DEVICE_URL.replace("{VIN}", vin))
        .header("Content-type", "application/json")
        .build()
        .context("Failed to build update request")?;

    debug!("Sending request {request:?}");
    let response = client.execute(request).await?;
    if response.status() == 400 {
        return Err(anyhow!(
            "Failed to retrieve device information, make sure the VIN {} is correct.",
            vin
        ));
    }
    if response.status() == 404 {
        return Err(anyhow!(
            "Device not found, make sure the VIN {} is correct.",
            vin
        ));
    }
    if response.status().is_server_error() {
        return Err(anyhow!(
            "Received error from server when requesting device information: {}",
            response.status()
        ));
    }

    debug!("Received response {response:?}");

    let response_text = response.text().await?;
    debug!("Received response body {response_text}");

    let device_response: DeviceResponse = serde_json::from_str(&response_text)
        .with_context(|| format!("Failed to parse device information: {response_text}"))?;

    Ok(device_response)
}

pub async fn request_available_updates(
    client: &reqwest::Client,
    vin: &str,
    map: Option<&str>,
) -> Result<UpdateResponse, Error> {
    // Body for firmware update request. Available software types are
    // - rcc-firmware: Firmware update for Bosch RCC (Radio Couleur Connectée)
    // - ovip-int-firmware-version: Firmware update for Continental NAC (Navigation Audio Connectée)
    // - aio-firmware: Firmware update for Continental NAC AIO (All In One) - Used outside Europe (South America)
    // - for NAC maps: map-afr, map-alg, map-asia, map-eur, map-isr, map-latam, map-latam-chile, map-mea, map-oce, map-russia, map-taiwan
    let body = if map.is_none() {
        serde_json::json!({
            "vin": vin,
            "softwareTypes": [
                { "softwareType": "ovip-int-firmware-version" },
                { "softwareType": "rcc-firmware" },
                { "softwareType": "aio-firmware" },
            ]
        })
    } else {
        serde_json::json!({
            "vin": vin,
            "softwareTypes": [
                { "softwareType": "ovip-int-firmware-version" },
                { "softwareType": "rcc-firmware" },
                { "softwareType": "aio-firmware" },
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
        .context("Failed to build update request")?;

    debug!("Sending request {request:?} with body {body:?}");
    let response = client.execute(request).await?;

    debug!("Received response {response:?}");

    if response.status().is_server_error() {
        return Err(anyhow!(
            "Received error from server when requesting updates: {}",
            response.status()
        ));
    }

    let response_text = response.text().await?;
    debug!("Received response body {response_text}");

    let update_response: UpdateResponse = serde_json::from_str(&response_text)
        .with_context(|| format!("Failed to parse response: {response_text}"))?;

    if update_response.request_result != "OK" {
        Err(anyhow!(
            "Failed to retrieve available updates, received an error from server: {}",
            update_response.request_result
        ))
    } else {
        Ok(update_response)
    }
}

pub async fn download_update(
    client: &reqwest::Client,
    software_update: &SoftwareUpdate,
    multi_progress: &MultiProgress,
) -> Result<DownloadedUpdate, Error> {
    debug!("Downloading update {software_update:?}");
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

// Extract firmware update to specified location
pub fn extract_update(update: &DownloadedUpdate, destination_path: &Path) -> Result<(), Error> {
    println!(
        "Extracting update to {}",
        destination_path.to_string_lossy()
    );

    if update.license_filename.is_some() {
        debug!("Copying licence file");
        let licence_destination_path = destination_path.join("license");
        fs::create_dir(&licence_destination_path).with_context(|| {
            format!(
                "Failed to create directory {}",
                licence_destination_path.to_string_lossy()
            )
        })?;
        let licence_destination =
            licence_destination_path.join(update.license_filename.as_ref().unwrap());
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
            update.update_filename,
            destination_path.to_string_lossy()
        )
    })?;
    Ok(())
}
