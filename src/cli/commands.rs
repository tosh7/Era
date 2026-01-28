// CLI commands definition - Sub1担当

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "era")]
#[command(about = "iOS Simulator CLI tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List available simulators
    List {
        /// Show only booted simulators
        #[arg(short, long)]
        booted: bool,
    },

    /// Boot a simulator
    Boot {
        /// Simulator device ID or name
        #[arg(required = true)]
        device: String,
    },

    /// Shutdown a simulator
    Shutdown {
        /// Simulator device ID or name (use "all" to shutdown all)
        #[arg(required = true)]
        device: String,
    },

    /// Install an app on a simulator
    Install {
        /// Simulator device ID or name
        #[arg(short, long, required = true)]
        device: String,

        /// Path to the .app bundle
        #[arg(required = true)]
        app_path: String,
    },

    /// Launch an app on a simulator
    Launch {
        /// Simulator device ID or name
        #[arg(short, long, required = true)]
        device: String,

        /// Bundle identifier of the app
        #[arg(required = true)]
        bundle_id: String,
    },

    /// Take a screenshot of a simulator
    Screenshot {
        /// Simulator device ID or name
        #[arg(short, long, required = true)]
        device: String,

        /// Output file path
        #[arg(required = true)]
        output: String,
    },
}
