use tokio::fs;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};

use log::debug;

use anyhow::{Context, Error, Result, anyhow};

use reqwest::header::{ACCEPT_RANGES, RANGE};
use reqwest::{Client, Response};

use futures_util::StreamExt;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct FileDownloadInfo {
    pub filename: String,
    pub filesize: u64,
    pub supports_resume: bool,
}

// Issue a head request to retrieve info on file to download
pub async fn request_file_download_info(
    client: &Client,
    url: &str,
) -> Result<FileDownloadInfo, Error> {
    // Issuing a HEAD request to retrieve download name and size
    debug!("Sending request HEAD {url}");
    let head_response = client.get(url).send().await?;
    debug!("Received response {head_response:?}");

    if head_response.status() != 200 {
        return Err(anyhow!(
            "Failed to fetch file information, got status {}.",
            head_response.status()
        ));
    }

    // Parse target filename from response
    let filename = String::from(parse_filename(&head_response)?);
    let filesize = head_response.content_length().unwrap_or(0);
    let supports_resume = head_response.headers().contains_key(ACCEPT_RANGES);

    Ok(FileDownloadInfo {
        filename,
        filesize,
        supports_resume,
    })
}

// Could not find a suitable crate to download a file that supports for resume
pub async fn download_file(
    client: &Client,
    url: &str,
    multi_progress: &MultiProgress,
    try_to_resume: bool,
) -> Result<String, Error> {
    let mut resume_position: u64 = 0; // Greater than zero means we will resume download
    let mut head_content_length: u64 = 0;

    if try_to_resume {
        // Issuing a HEAD request to retrieve download name and size
        let file_info = request_file_download_info(client, url).await?;

        if !file_info.supports_resume {
            debug!("Server does support range header");
        } else {
            let file_metadata = fs::metadata(&file_info.filename).await;
            if file_metadata.is_ok() {
                resume_position = file_metadata.ok().unwrap().len();
                debug!(
                    "File {} exists with size: {}",
                    &file_info.filename, resume_position
                );

                head_content_length = file_info.filesize;
                if head_content_length == resume_position {
                    println!(
                        "Skipping download of file {}, already completed",
                        file_info.filename
                    );
                    return Ok(file_info.filename);
                }
            }
        }
    }

    let mut request = client.get(url);
    if resume_position > 0 {
        debug!("Adding range header to resume download: bytes={resume_position}-");
        request = request.header(RANGE, format!("bytes={resume_position}-"));
    }

    debug!("Sending request GET {url}");
    let response = request.send().await?;
    debug!("Received response {response:?}");

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to download file, got status {}.",
            response.status()
        ));
    }

    // Parse target filename from response
    let filename = String::from(parse_filename(&response)?);

    let remaining_content_length = response.content_length().unwrap_or(0);
    let total_content_length = if resume_position > 0 {
        head_content_length // content length retrieved on HEAD request in case of download resume
    } else {
        remaining_content_length
    };

    let progress_bar = multi_progress.add(ProgressBar::new(total_content_length));
    progress_bar.set_style(
        ProgressStyle::with_template(
            "{percent:>3}% [{bar}] {bytes_per_sec:<12} ETA={eta:<3} {wide_msg:.cyan}",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    progress_bar.set_message(filename.to_string()); // Triggers first draw
    progress_bar.set_position(resume_position);
    // Need to reset ETA in case of resume, otherwise estimations are biased
    progress_bar.reset_eta();

    let file = if resume_position == 0 {
        debug!("Opening {filename} in create mode");
        File::create(filename.clone())
            .await
            .with_context(|| format!("Failed to create file {filename}"))?
    } else {
        debug!("Opening {filename} in append mode for resume");
        OpenOptions::new()
            .append(true)
            .open(filename.clone())
            .await
            .with_context(|| format!("Failed to open file {filename} in append mode"))?
    };

    let mut stream = response.bytes_stream();

    let mut file_writer = BufWriter::new(file);

    while let Some(item) = stream.next().await {
        let chunk =
            item.with_context(|| format!("Failed to download file {filename} from {url}"))?;
        progress_bar.inc(chunk.len() as u64);
        file_writer
            .write_all(&chunk)
            .await
            .with_context(|| format!("Error writing to file {filename}"))?;
    }
    file_writer
        .flush()
        .await
        .with_context(|| format!("Error flushing file {filename}"))?;

    progress_bar.finish();
    Ok(filename)
}

// Parse the name of the file to download from the response
fn parse_filename(response: &Response) -> Result<&str, Error> {
    // Try to parse content-disposition header for filename
    let filename_from_header = parse_filename_from_content_disposition(response)?;
    if let Some(filename) = filename_from_header {
        return Ok(filename);
    }

    // Deduce filename from last path segment of url
    debug!("Parsing filename from url: {}", response.url());
    let filename_from_url: Option<&str> = response
        .url()
        .path_segments()
        .and_then(|mut s| s.next_back());
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
    if content_disposition.is_none() {
        return Ok(None); // No content-disposition header
    }

    // We have a content-disposition header, we should be able to find a filename
    let content_disposition_str = content_disposition.unwrap().to_str().with_context(|| {
        format!(
            "Failed to fetch content-disposition header: {:?}",
            content_disposition.unwrap()
        )
    })?;
    debug!("Parsing content-disposition header: {content_disposition_str}");

    // Parse filename parameter, handling both quoted and unquoted values
    // Example headers:
    // - content-disposition: attachment; filename="example.txt"
    // - content-disposition: attachment; filename=example.txt
    // Note: filename* (RFC 5987 encoded) is not handled here
    for part in content_disposition_str.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix("filename=") {
            // Remove surrounding quotes if present
            let filename = value.trim_matches('"').trim();
            if !filename.is_empty() {
                return Ok(Some(filename));
            }
        }
    }

    Err(anyhow!(
        "No filename found in content-disposition header: {}",
        content_disposition_str
    ))
}
