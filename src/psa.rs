use std::fs;
use std::fs::File;
use std::path::Path;

use serde::{Deserialize, Serialize};

use log::debug;

use anyhow::{anyhow, Context, Error, Result};

use indicatif::MultiProgress;

use tar::Archive;

use crate::download;

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

pub async fn request_available_updates(
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

pub async fn download_update(
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

// Extract firmware update to specified location
pub fn extract_update(update: &DownloadedUpdate, destination_path: &Path) -> Result<(), Error> {
    println!(
        "Extracting update to {}",
        destination_path.to_string_lossy()
    );

    if update.license_filename != None {
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
