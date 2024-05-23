use std::fs;
use std::path::PathBuf;
use std::process::exit;
use clap::{CommandFactory, Parser, Subcommand};
use libtermcolor::colors;
use crate::server::Software;
use crate::server_manager::check_valid_version;

mod server;
mod server_manager;

/// A Local Minecraft Server Plugin Testing Solution
#[derive(Parser, Debug)]
#[command(about, long_about, name = "mcsdk", version)]
#[command(author = "Noriskky")]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,   
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start a Local Test Server
    #[command(arg_required_else_help = true)]
    Start {
        /// Define what Server Software should be used
        #[arg(required = true, value_enum)]
        software: Software,

        /// Which Minecraft Version should be started
        #[arg(required = true)]
        version: String,
        
        /// Path to Plugin jars to put into the plugins Folder
        #[arg()]
        plugins: Vec<PathBuf>,

        /// Where the server should be stored
        #[arg(short, long, default_value = "none")]
        working_directory: PathBuf,
        
        /// Arguments to give the server
        #[arg(short, long)]
        args: Vec<String>,
        
        /// How much Ram is the server allowed to use
        #[arg(short, long, default_value = "2048")]
        mem: u32,
        
        /// If used the server Gui will start too
        #[arg(short, long)]
        gui: bool
    }
}

pub fn send_info(msg: String) {
    println!("{}[{}MC-SDK{}]{} {}{}", colors::bright_black().regular, colors::bright_green().regular, colors::bright_black().regular, colors::bright_green().regular, msg, colors::reset())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if let Some(Commands::Start { software, version, plugins, working_directory, mut args, mem, gui }) = args.command {
        if !check_valid_version(&version).await {
            exit(1)
        }
    
        if !gui { 
            args.push("--nogui".to_string())
        }
        
        if working_directory != PathBuf::from("none") {
            if !working_directory.exists() {
                if let Err(err) = fs::create_dir(working_directory.clone()) { eprintln!("Error creating directory: {}", err) }
            }

            if !working_directory.is_dir() {
                eprintln!("Error: You need to specify a Directory not a file");
                exit(1)
            }
        }

        let mut server = server::Server {
            wd: working_directory,
            software,
            version,
            plugins,
            args,
            mem,
        };

        server.init_server().await;
        if let Err(err) = server.start_server().await {
            eprintln!("Error starting server: {}", err);
            exit(1);
        }
        
        println!("\n");
        send_info("Server Stopped.".to_string())
    }

    // If no arguments are provided, show the help message
    if std::env::args().len() == 1 {
        Args::command().print_help().unwrap();
        exit(0);
    }
    exit(0)
}