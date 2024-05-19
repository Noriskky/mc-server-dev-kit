mod ServerManagment;

use std::{fs, io};
use std::path::{Path, PathBuf};
use std::process::exit;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum, ValueHint};
use libtermcolor::colors;
use regex::Regex;
use crate::ServerManagment::{Server, Software};

#[derive(Parser, Debug)]
#[command(about, long_about, name = "mcsdk")]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug, PartialEq)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Start {
        #[arg(required = true, value_enum)]
        software: Software,

        #[arg(required = true)]
        version: String,

        #[arg(require_equals = true)]
        plugins: Vec<PathBuf>,

        #[arg(short, long, default_value = "none")]
        working_directory: PathBuf,

        #[arg(short, long)]
        args: Vec<String>,

        #[arg(short, long, default_value = "2048")]
        mem: u32
    },

    #[command()]
    List {}
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.command {
        Some(Commands::Start { software, version, plugins, working_directory, args, mem }) => {
            if !ServerManagment::check_valid_version(&*version).await {
                exit(1)
            }

            if working_directory != PathBuf::from("none") {
                if !working_directory.exists() {
                    match fs::create_dir(working_directory.clone()) {
                        Err(err) => eprintln!("Error creating directory: {}", err),
                        Ok(_) => {}
                    }
                }

                if !working_directory.is_dir() {
                    eprintln!("Error: You need to specify a Directory not a file");
                    exit(1)
                }
            }

            let mut server = Server {
                wd: working_directory,
                software: software,
                version: version,
                plugins: plugins,
                args: args,
                mem: mem,
            };

            server.init_server().await;
        }
        Some(Commands::List {}) => {}
        _ => {}
    }

    exit(1)
}