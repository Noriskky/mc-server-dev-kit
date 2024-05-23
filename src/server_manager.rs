use std::{env, fs};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::exit;

use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use rand::distributions::Alphanumeric;
use rand::Rng;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use tempdir::TempDir;

use crate::send_info;
use crate::server::Software;

#[derive(Debug, Deserialize)]

  /***************************************************/
 /*                    Vanilla                      */
/***************************************************/
pub struct VanillaApiResponse {
    latest: ApiVanillaLatestVersions,
    pub(crate) versions: Vec<ApiVanillaVersionEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ApiVanillaLatestVersions {
    release: String,
    snapshot: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiVanillaVersionEntry {
    id: String,
    #[serde(rename = "type")]
    version_type: String,
}

  /***************************************************/
 /*                    Paper                        */
/***************************************************/

#[derive(Debug, Deserialize)]
pub struct PaperApiResponse {
    latest: String,
    versions: HashMap<String, String>,
}

pub fn generate_random_uuid() -> String {
    let random_string: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    random_string
}

pub fn get_temp_folder() -> Result<PathBuf, std::io::Error> {
    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let mut user_writable_dirs = vec!["/var/tmp"];

        user_writable_dirs.retain(|dir| {
            let metadata = fs::metadata(dir).ok();
            let permissions = metadata.expect("REASON").permissions();
            permissions.mode() & 0o200 != 0
        });

        if let Some(user_writable_dir) = user_writable_dirs.first() {
            return Ok(PathBuf::from(user_writable_dir));
        }
    }

    let temp_dir = match env::temp_dir().to_str() {
        Some(path) => path.to_string(),
        None => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid temp directory path")),
    };

    let temp_folder = TempDir::new_in(temp_dir, "mcsdk-tmp")?;
    Ok(temp_folder.into_path())
}

pub fn createdir(dir: PathBuf) {
    if !dir.exists() {
        if let Err(err) = fs::create_dir(dir.clone()) {
            eprintln!("Error creating directory: {}", err);
            exit(1)
        }
    }
}

pub async fn download_server_software(software: Software, version: String, wd: PathBuf) {
    let mut downloadurl = String::new();

    if software == Software::Paper {
        match paper_get_download_link(Some(&version)).await {
            Ok(download_link) => {
                downloadurl = download_link;
            },
            Err(e) => {
                eprintln!("Error: {}", e);
                exit(1);
            },
        }
        // } else if software == Software::Spigot {
    }

    if let Err(err) = download_file(&downloadurl, &wd, "server.jar").await {
        eprintln!("Error: {}", err);
    }
}

pub async fn paper_get_download_link(version: Option<&str>) -> Result<String, String> {
    let url = "https://qing762.is-a.dev/api/papermc";
    let response = match reqwest::get(url).await {
        Ok(resp) => resp,
        Err(e) => return Err(format!("Failed to fetch API response: {}", e)),
    };

    if !response.status().is_success() {
        return Err(format!("Failed to fetch API response: Status code {}", response.status()));
    }

    let json_response: PaperApiResponse = match response.json().await {
        Ok(resp) => resp,
        Err(e) => return Err(format!("Failed to parse JSON response: {}", e)),
    };

    let version = match version {
        Some(version) => version,
        None => &json_response.latest,
    };

    match json_response.versions.get(version) {
        Some(download_link) => Ok(download_link.clone()),
        None => Err(format!("Version {} not found in API response.", version)),
    }
}

pub fn copy_file_to_folder(file_path: PathBuf, folder_path: PathBuf) -> std::io::Result<()> {
    if !folder_path.is_dir() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Destination folder does not exist"));
    }

    let file_name = match file_path.file_name() {
        Some(name) => name,
        None => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid file path")),
    };

    let mut destination_path = folder_path.clone();
    destination_path.push(file_name);

    fs::copy(&file_path, &destination_path)?;

    Ok(())
}

pub fn copy_plugins(plugins: Vec<PathBuf>, plugins_folder: PathBuf) {
    for plugin in plugins {
        if !plugin.exists() {
            eprintln!("{:?} does not exist. Skipping...", plugin.file_name().unwrap());
            return;
        }
        if plugin.is_file() && plugin.is_absolute() && !plugin.is_symlink() {
            match copy_file_to_folder(plugin.clone(), plugins_folder.clone()) {
                Ok(()) => send_info(format!("{} moved to plugins Folder.", plugin.file_name().unwrap().to_str().unwrap())),
                Err(e) => eprintln!("Failed to copy file: {}", e),
            }
        }
    }
}

async fn download_file(url: &str, save_dir: &PathBuf, file_name: &str) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let response = client.get(url).send().await?;
    let content_length = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(content_length);
    pb.set_style(ProgressStyle::default_bar()
        .template("{bar:40.green/green} {bytes}/{total_bytes} ({eta})")?
        .progress_chars("#>-"));

    if !save_dir.exists() {
        tokio::fs::create_dir_all(save_dir).await?;
    }

    let mut file_path = save_dir.clone();
    file_path.push(file_name);

    let mut file = File::create(&file_path)?;
    let mut downloaded = 0;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
        file.write_all(&chunk)?;
    }
    pb.finish_with_message("Download complete");

    Ok(())
}

pub async fn check_valid_version(version_to_check: &str) -> bool {
    let version_regex_pattern = r"^1\.\d{1,2}\.\d{1,2}$";
    let version_regex = Regex::new(version_regex_pattern).unwrap();

    if !version_regex.is_match(version_to_check) {
        eprintln!("Error: '{}' is not a valid version number.", version_to_check);
        return false;
    }

    let url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
    let response = match reqwest::get(url).await {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Error: Failed to fetch version manifest - {}", e);
            return false;
        }
    };

    if !response.status().is_success() {
        eprintln!("Error: Failed to fetch version manifest - Status code {}", response.status());
        return false;
    }

    let json_response: VanillaApiResponse = match response.json().await {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Error: Failed to parse JSON response - {}", e);
            return false;
        }
    };

    let available_versions: HashSet<String> = json_response.versions.into_iter().map(|entry| entry.id).collect();

    if !available_versions.contains(version_to_check) {
        eprintln!("Error: Version {} not found in version manifest.", version_to_check);
        return false;
    }
    
    true
}
    