use std::{env, fs};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Debug, format};
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, exit, Stdio};

use clap::ValueEnum;
use futures_util::{StreamExt, TryStreamExt};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use rand::distributions::Alphanumeric;
use rand::Rng;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use tempdir::TempDir;
use tokio::io::{AsyncWriteExt};
use tokio::process::Child;
use crate::send_info;
use tokio::process::Command as TokioCommand;
use tokio::signal::unix::{signal, SignalKind};

#[derive(Debug, Deserialize)]
struct Vanilla_VersionManifest {
    latest: Vanilla_LatestVersions,
    versions: Vec<Vanilla_VersionEntry>,
}

#[derive(Debug, Deserialize)]
struct Vanilla_LatestVersions {
    release: String,
    snapshot: String,
}

#[derive(Debug, Deserialize)]
struct Vanilla_VersionEntry {
    id: String,
    #[serde(rename = "type")]
    version_type: String,
}

#[derive(Debug, Deserialize)]
struct Paper_ApiResponse {
    latest: String,
    versions: HashMap<String, String>,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum Software {
    Paper,
    Spigot
}

fn generate_random_uuid() -> String {
    let random_string: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    random_string
}

fn get_temp_folder() -> Result<PathBuf, std::io::Error> {
    // On Unix-like systems, check for user-writable directories
    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let mut user_writable_dirs = vec!["/var/tmp"];
        // Add any additional directories you want to check for user-writability

        // Filter directories that are user-writable
        user_writable_dirs.retain(|dir| {
            let metadata = fs::metadata(dir).ok();
            let permissions = metadata.expect("REASON").permissions();
            permissions.mode() & 0o200 != 0
        });

        // If there are user-writable directories, use the first one found
        if let Some(user_writable_dir) = user_writable_dirs.first() {
            return Ok(PathBuf::from(user_writable_dir));
        }
    }

    // Get the temporary directory path for the current platform
    let temp_dir = match env::temp_dir().to_str() {
        Some(path) => path.to_string(),
        None => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid temp directory path")),
    };

    // If no user-writable directories found or not on Unix, fallback to system temp directory
    let temp_folder = TempDir::new_in(&temp_dir, "mcsdk-tmp")?;
    Ok(temp_folder.into_path())
}

pub fn createdir(dir: PathBuf) {
    if !dir.exists() {
        match fs::create_dir(dir.clone()) {
            Err(err) => {
                eprintln!("Error creating directory: {}", err);
                exit(1)
            },
            Ok(_) => {}
        }
    }
}

pub struct Server {
    pub wd: PathBuf,
    pub software: Software,
    pub version: String,
    pub plugins: Vec<PathBuf>,
    pub args: Vec<String>,
    pub mem: u32
}

impl Server {
    pub async fn init_server(&mut self) {
        // Create Working Directory
        send_info("Creating Working Directory.".to_string());
        if self.wd == PathBuf::from("none") {
            let dir_name = format!("{:?}:{}-{}", self.software, self.version, generate_random_uuid());
            self.wd = get_temp_folder().unwrap();
            self.wd.push("mcsdk");
            createdir(self.wd.clone());
            self.wd.push(dir_name);
            createdir(self.wd.clone());
        } else {
            if let Ok(full_path) = self.wd.canonicalize() {
                self.wd = full_path
            } else {
                eprintln!("Error: Failed to get the full path.");
                exit(1)
            }
        }

        // Download Server Software
        send_info("Downloading Server Software.".to_string());
        download_server_software(self.software, self.version.clone(), self.wd.clone()).await;

        // Create Eula txt
        send_info("Creating Eula.txt.".to_string());
        let mut path = self.wd.clone();  // Use the provided directory
        path.push("eula.txt");

        // Create and open the file at the specified path
        match File::create(&path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(b"eula=true") {
                    eprintln!("Error writing to eula.txt: {}", e);
                    exit(1);
                }
            }
            Err(e) => {
                eprintln!("Error creating eula.txt: {}", e);
                exit(1);
            }
        }
        
        let mut plugins_folder = self.wd.clone();
        plugins_folder.push("plugins");
        createdir(plugins_folder.clone());
        copy_plugins(self.plugins.clone(), plugins_folder);
    }

    pub async fn start_server(&self) -> Result<(), Box<dyn Error>> {
        let mut command = TokioCommand::new("java");
        command.args(&["-Xms256M", &format!("-Xmx{}M", self.mem), "-jar", "server.jar"]); // Assuming the server jar is named "server.jar"

        // Adding extra server arguments if provided
        for arg in &self.args {
            command.arg(arg);
        }

        // Set working directory
        command.current_dir(&self.wd);

        // Redirect stdout, stdin, and stderr to inherit from the parent process
        command.stdout(std::process::Stdio::inherit())
            .stdin(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());

        // Spawn the child process
        let mut child = command.spawn()?;

        // Set up a signal handler for SIGINT (Ctrl+C)
        let mut signal = signal(SignalKind::interrupt())?;

        // Wait for either the child process to exit or the Ctrl+C signal
        tokio::select! {
            _ = child.wait() => {
                // Child process exited, no action needed
            }
            _ = signal.recv() => {
                // Ctrl+C signal received, terminate the child process
                child.kill().await;
            }
        }

        Ok(())
    }

}

pub async fn download_server_software(software: Software, version: String, wd: PathBuf) {
    let mut downloadurl = String::new();

    if software == Software::Paper {
        match Paper_get_Download_link(Some(&version)).await {
            Ok(download_link) => {
                downloadurl = download_link;
            },
            Err(e) => {
                eprintln!("Error: {}", e);
                exit(1);
            },
        }
    } else if software == Software::Spigot {
    }

    if let Err(err) = download_file(&*downloadurl, &wd, "server.jar").await {
        eprintln!("Error: {}", err);
    }
}

pub async fn Paper_get_Download_link(version: Option<&str>) -> Result<String, String> {
    let url = "https://qing762.is-a.dev/api/papermc";
    let response = match reqwest::get(url).await {
        Ok(resp) => resp,
        Err(e) => { return Err(format!("Failed to fetch API response: {}", e)); exit(1) },
    };

    if !response.status().is_success() {
        return Err(format!("Failed to fetch API response: Status code {}", response.status()));
    }

    let json_response: Paper_ApiResponse = match response.json().await {
        Ok(resp) => resp,
        Err(e) => { return Err(format!("Failed to parse JSON response: {}", e)); exit(1) },
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

fn copy_file_to_folder(file_path: PathBuf, folder_path: PathBuf) -> std::io::Result<()> {
    // Ensure the folder path exists
    if !folder_path.is_dir() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Destination folder does not exist"));
    }

    // Get the file name from the file path
    let file_name = match file_path.file_name() {
        Some(name) => name,
        None => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid file path")),
    };

    // Construct the destination path
    let mut destination_path = folder_path.clone();
    destination_path.push(file_name);

    // Copy the file to the destination path
    fs::copy(&file_path, &destination_path)?;

    Ok(())
}

fn copy_plugins(plugins: Vec<PathBuf>, plugins_folder: PathBuf) {
    for plugin in plugins {
        if !plugin.exists() {
            eprintln!("{:?} does not exist. Skipping...", plugin.file_name());
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

    let json_response: Vanilla_VersionManifest = match response.json().await {
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