use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::{exit, Stdio};
use clap::ValueEnum;
use regex::Regex;
use tokio::process::Command;
use tokio::signal::unix::{signal, SignalKind};
use crate::send_info;
use crate::ServerManager::{copy_plugins, createdir, download_server_software, generate_random_uuid, get_temp_folder, Vanilla_VersionManifest};

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum Software {
    Paper,
    Spigot
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

        send_info("Downloading Server Software.".to_string());
        download_server_software(self.software, self.version.clone(), self.wd.clone()).await;

        send_info("Creating Eula.txt.".to_string());
        let mut path = self.wd.clone();
        path.push("eula.txt");

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
        let mut command = Command::new("java");
        command.args(&["-Xms256M", &format!("-Xmx{}M", self.mem), "-jar", "server.jar"]);

        for arg in &self.args {
            command.arg(arg);
        }

        command.current_dir(&self.wd);

        command.stdout(Stdio::inherit())
            .stdin(Stdio::inherit())
            .stderr(Stdio::inherit());

        let mut child = command.spawn()?;

        let mut signal = signal(SignalKind::interrupt())?;

        tokio::select! {
            _ = child.wait() => {
            }
            _ = signal.recv() => {
                child.kill().await;
            }
        }

        Ok(())
    }

}