use std::io::{self, BufRead};
use std::time::Duration;

use anyhow::{Context, ensure};
use once_cell::sync::Lazy;
use reqwest::{
    blocking::{ClientBuilder},
    header::{self, HeaderMap, HeaderValue},
    Result,
};
use url::Url;

use crate::{
    parse::*,
    urls::*,
};
use crate::extract::read_html_gz;

pub struct RemoteArchiveFile {
    pub name: String,
    pub url: Url,
    pub expected_size: Option<usize>,
}

pub fn list_archives(list_url: &Url) -> Result<impl Iterator<Item=RemoteArchiveFile>> {
    static CLIENT: Lazy<reqwest::blocking::Client> = Lazy::new(|| {
        ClientBuilder::new()
            .timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build().unwrap()
    });

    log::debug!("Downloading list of archives: {}", list_url);
    CLIENT.get(list_url.clone()).send().map(|response|
        io::BufReader::new(response)
            .lines()
            .flatten()
            .filter_map(|line| parse_html_gz_url_list_line(&line)
                .and_then(|(rel, expected_size)| {
                    let url = TENHOU_ARCHIVE_ROOT.join(rel).ok()?;
                    let name = url.path_segments()?.last().unwrap_or("").to_string();
                    Some(RemoteArchiveFile { name, url, expected_size })
                })))
}

pub fn list_recent_archives() -> Result<impl Iterator<Item=RemoteArchiveFile>> {
    list_archives(&TENHOU_LIST_RECENT)
}

pub fn list_year_to_date_archives() -> Result<impl Iterator<Item=RemoteArchiveFile>> {
    list_archives(&TENHOU_LIST_YEAR_TO_DATE)
}

pub fn download_archive(remote_file: &RemoteArchiveFile) -> anyhow::Result<String> {
    static CLIENT: Lazy<reqwest::blocking::Client> = Lazy::new(|| {
        ClientBuilder::new()
            .timeout(Duration::from_secs(20))
            .no_gzip()  // make sure we have the crate feature enabled
            .redirect(reqwest::redirect::Policy::limited(5))
            .build().unwrap()
    });

    log::debug!("Downloading archive: {}", remote_file.url);
    let response = CLIENT.get(remote_file.url.clone()).send()?;
    ensure!(response.status().is_success(),
        "Failed to download archive: status={}", response.status());
    let archive_str = read_html_gz(response)?;
    // TODO(summivox): find out a way to validate the gzipped length (reqwest removed it!)
    /*
    if let Some(expected_size) = remote_file.expected_size {
        // NOTE: string "len" is in bytes, so this comparison is apples to apples.
        ensure!(archive_str.len() == expected_size,
            "Expected size {}, actual size {}", expected_size, archive_str.len());
    }
     */
    Ok(archive_str)
}

pub fn download_game_json(id: &str) -> anyhow::Result<String> {
    static CLIENT: Lazy<reqwest::blocking::Client> = Lazy::new(|| {
        let mut headers = HeaderMap::new();
        headers.insert(header::REFERER,
                       HeaderValue::from_str(TENHOU_REFERER.as_str()).unwrap());

        ClientBuilder::new()
            .timeout(Duration::from_secs(20))
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build().unwrap()
    });

    log::debug!("Downloading json: {}", id);
    CLIENT.get(tenhou_download_url(id)).send()?.text()
        .context("Failed to download game JSON")
        .and_then(|text| {
            ensure!(text.starts_with("{"), "Response does not look like JSON: {}", text);
            Ok(text)
        })
}
