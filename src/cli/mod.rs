// CLI module - Sub1担当

pub mod commands;

use clap::Parser;
use commands::{Cli, Commands};

pub fn run() {
    let cli = Cli::parse();

    match cli.command {
        Commands::List { booted } => {
            if booted {
                println!("Listing booted simulators...");
            } else {
                println!("Listing all simulators...");
            }
            // TODO: Call simulator::operations::list()
        }
        Commands::Boot { device } => {
            println!("Booting simulator: {}", device);
            // TODO: Call simulator::operations::boot()
        }
        Commands::Shutdown { device } => {
            println!("Shutting down simulator: {}", device);
            // TODO: Call simulator::operations::shutdown()
        }
        Commands::Install { device, app_path } => {
            println!("Installing {} on {}", app_path, device);
            // TODO: Call simulator::operations::install()
        }
        Commands::Launch { device, bundle_id } => {
            println!("Launching {} on {}", bundle_id, device);
            // TODO: Call simulator::operations::launch()
        }
        Commands::Screenshot { device, output } => {
            println!("Taking screenshot of {} to {}", device, output);
            // TODO: Call simulator::operations::screenshot()
        }
        Commands::Input { device, key } => {
            println!("Sending {:?} key to {}", key, device);
            // TODO: Call simulator::operations::input()
        }
        Commands::Openurl { device, url } => {
            println!("Opening URL {} on {}", url, device);
            // TODO: Call simulator::operations::openurl()
        }
        Commands::Tap { device, x, y } => {
            println!("Tapping at ({}, {}) on {}", x, y, device);
            // TODO: Call simulator::operations::tap()
        }
        Commands::Swipe { device, start_x, start_y, end_x, end_y } => {
            println!("Swiping from ({}, {}) to ({}, {}) on {}", start_x, start_y, end_x, end_y, device);
            // TODO: Call simulator::operations::swipe()
        }
        Commands::Enumerate { device } => {
            println!("Enumerating input devices on {}", device);
            // TODO: Call simulator::operations::enumerate()
        }
    }
}
