use std::fs;
use std::path::PathBuf;
use std::process::exit;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use libtermcolor::colors;
use crate::Server::Software;
use crate::ServerManager::check_valid_version;

mod Server;
mod ServerManager;

/// A Local Minecraft Server Plugin Testing Solution
#[derive(Parser, Debug)]
#[command(about, long_about, name = "mcsdk")]
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
        #[arg(require_equals = true)]
        plugins: Vec<PathBuf>,

        /// Where the server should be stored (default="/var/tmp/mcsdk/<server>)
        #[arg(short, long, default_value = "none")]
        working_directory: PathBuf,
        
        /// Arguments to give the server
        #[arg(short, long)]
        args: Vec<String>,
        
        /// How much Ram is the server allowed to use
        #[arg(short, long, default_value = "2048")]
        mem: u32
    },
    
    // Servers currently running
    //#[command()]
    //List {}
}

pub fn send_info(msg: String) {
    println!("{}[{}MC-SDK{}]{} {}{}", colors::bright_black().regular, colors::bright_green().regular, colors::bright_black().regular, colors::bright_green().regular, msg, colors::reset())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.command {
        Some(Commands::Start { software, version, plugins, working_directory, args, mem }) => {
            if !check_valid_version(&*version).await {
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

            let mut server = Server::Server {
                wd: working_directory,
                software: software,
                version: version,
                plugins: plugins,
                args: args,
                mem: mem,
            };

            server.init_server().await;
            if let Err(err) = server.start_server().await {
                eprintln!("Error starting server: {}", err);
                exit(1);
            }
        }
        //Some(Commands::List {}) => {}
        _ => {}
    }

    exit(1)
}