use std::fs;
use std::fs::{File, OpenOptions};
use std::io::Write;

use log::debug;

use anyhow::{anyhow, Context, Error, Result};

use regex::{Match, Regex};

use reqwest::header::{ACCEPT_RANGES, RANGE};
use reqwest::{Client, Response};

use futures_util::StreamExt;

use indicatif::{ProgressBar, ProgressStyle};

// Could not find a suitable crate to download a file that supports for resume
pub async fn download_file(
    client: &Client,
    url: &str,
    try_to_resume: bool,
) -> Result<String, Error> {
    let mut resume_position: u64 = 0; // Greater than zero means we will resume download
    let mut head_content_length: u64 = 0;

    if try_to_resume {
        // Issuing a HEAD request to retrieve download name and size
        debug!("Sending request HEAD {}", url);
        let head_response = client.get(url).send().await?;
        debug!("Received response {:?}", head_response);

        // Parse target filename from response
        let head_filename = String::from(parse_filename(&head_response)?);

        if !head_response.headers().contains_key(ACCEPT_RANGES) {
            debug!("Server does support range header");
        } else {
            let file_metadata = fs::metadata(&head_filename);
            if file_metadata.is_ok() {
                resume_position = file_metadata.ok().unwrap().len();
                debug!(
                    "File {} exists with size: {}",
                    head_filename, resume_position
                );

                head_content_length = head_response.content_length().unwrap_or(0);
                if head_content_length == resume_position {
                    println!(
                        "Skipping download of file {}, already completed",
                        head_filename
                    );
                    return Ok(head_filename);
                }
            }
        }
    }

    let mut request = client.get(url);
    if resume_position > 0 {
        debug!(
            "Adding range header to resume download: bytes={}-",
            resume_position
        );
        request = request.header(RANGE, format!("bytes={}-", resume_position));
    }

    debug!("Sending request GET {}", url);
    let response = request.send().await?;
    debug!("Received response {:?}", response);

    // Parse target filename from response
    let filename = String::from(parse_filename(&response)?);

    let remaining_content_length = response.content_length().unwrap_or(0);
    let total_content_length = if resume_position > 0 {
        head_content_length // content length retrieved on HEAD request in case of download resume
    } else {
        remaining_content_length
    };

    // TODO check available disk space

    // TODO Investigate parallel download
    let mut file = if resume_position == 0 {
        println!("Downloading file {}...", filename);
        File::create(filename.clone())
            .with_context(|| format!("Failed to create file {}", filename))?
    } else {
        println!("Resuming download of file {}...", filename);
        debug!("Opening {} in append mode", filename);
        OpenOptions::new()
            .append(true)
            .open(filename.clone())
            .with_context(|| format!("Failed to open file {} in append mode", filename))?
    };

    let progress_bar = ProgressBar::new(total_content_length);
    progress_bar.set_style(ProgressStyle::default_bar().template(
        "[{elapsed_precise}] {wide_bar} {bytes:7}/{total_bytes:7}@{bytes_per_sec} ({eta})",
    ));
    progress_bar.set_position(resume_position);

    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk =
            item.with_context(|| format!("Failed to download file {} from {}", filename, url))?;
        progress_bar.inc(chunk.len() as u64);
        file.write_all(&chunk)?; // TODO Investigate buffered / async write
    }

    progress_bar.finish();
    Ok(filename)
}

// Parse the name of the file to download from the response
fn parse_filename(response: &Response) -> Result<&str, Error> {
    // Try to parse content-disposition header for filename
    let filename_from_header = parse_filename_from_content_disposition(response)?;
    if filename_from_header != None {
        return Ok(filename_from_header.unwrap());
    }

    // Deduce filename from last path segment of url
    debug!("Parsing filename from url: {}", response.url());
    let filename_from_url: Option<&str> =
        response.url().path_segments().map(|s| s.last()).flatten();
    match filename_from_url {
        Some(f) => Ok(f),
        None => Err(anyhow!(
            "Failed to parse the filename from the url {}",
            response.url()
        )),
    }
}

// Parse the name of the file to download from the content-disposition header of the response
fn parse_filename_from_content_disposition(response: &Response) -> Result<Option<&str>, Error> {
    let content_disposition = response.headers().get("content-disposition");
    if content_disposition == None {
        return Ok(None); // No content-disposition header
    }

    // We have a content-disposition header, we should be able to find a filename
    let content_disposition_str = content_disposition
        .unwrap()
        .to_str()
        .with_context(|| format!("Failed to fetch content-disposition header"))?;
    debug!(
        "Parsing content-disposition header: {}",
        content_disposition_str
    );

    // TODO Could not find a nice way to parse content-disposition header in reqwest
    // Workaround: use an ugly regexp
    let re = Regex::new(r"attachment; filename=(\S+)")
        .with_context(|| format!("Failed to compile content-disposition regexp"))?;

    let re_match: Option<Match> = re
        .captures(content_disposition_str)
        .map(|c| c.get(1))
        .flatten();

    let filename: Option<&str> = re_match.map(|m| m.as_str());

    match filename {
        Some(x) => Ok(Some(x)),
        None => Err(anyhow!("Failed to parse content-disposition header")),
    }
}