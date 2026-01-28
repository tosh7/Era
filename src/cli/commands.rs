// CLI commands definition - Sub1担当

use clap::{Parser, Subcommand, ValueEnum};

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

    /// Send keyboard input to a simulator
    Input {
        /// Simulator device ID or name
        #[arg(short, long, required = true)]
        device: String,

        /// Key to send
        #[arg(short, long, required = true, value_enum)]
        key: KeyType,
    },

    /// Open a URL in the simulator
    Openurl {
        /// Simulator device ID or name
        #[arg(short, long, required = true)]
        device: String,

        /// URL to open
        #[arg(short, long, required = true)]
        url: String,
    },

    /// Tap on the simulator screen (requires IDB)
    ///
    /// Coordinates are in logical points by default.
    /// Use --scale to convert pixel coordinates from screenshots.
    /// Example: --scale 3 for iPhone Pro models (3x Retina)
    Tap {
        /// Simulator device ID or name
        #[arg(short, long, required = true)]
        device: String,

        /// X coordinate (pixels if --scale is set, otherwise logical points)
        #[arg(short = 'x', long, required = true)]
        x: u32,

        /// Y coordinate (pixels if --scale is set, otherwise logical points)
        #[arg(short = 'y', long, required = true)]
        y: u32,

        /// Scale factor to convert pixel coordinates to logical points.
        /// Use 2 for 2x Retina (iPhone SE), 3 for 3x Retina (iPhone Pro).
        /// When set, x and y are treated as pixel coordinates from screenshots.
        #[arg(short, long)]
        scale: Option<u32>,
    },

    /// Swipe on the simulator screen (requires IDB)
    ///
    /// Coordinates are in logical points by default.
    /// Use --scale to convert pixel coordinates from screenshots.
    /// Example: --scale 3 for iPhone Pro models (3x Retina)
    Swipe {
        /// Simulator device ID or name
        #[arg(short, long, required = true)]
        device: String,

        /// Start X coordinate (pixels if --scale is set, otherwise logical points)
        #[arg(long, required = true)]
        start_x: u32,

        /// Start Y coordinate (pixels if --scale is set, otherwise logical points)
        #[arg(long, required = true)]
        start_y: u32,

        /// End X coordinate (pixels if --scale is set, otherwise logical points)
        #[arg(long, required = true)]
        end_x: u32,

        /// End Y coordinate (pixels if --scale is set, otherwise logical points)
        #[arg(long, required = true)]
        end_y: u32,

        /// Scale factor to convert pixel coordinates to logical points.
        /// Use 2 for 2x Retina (iPhone SE), 3 for 3x Retina (iPhone Pro).
        /// When set, coordinates are treated as pixels from screenshots.
        #[arg(short, long)]
        scale: Option<u32>,
    },

    /// Enumerate available input devices
    Enumerate {
        /// Simulator device ID or name
        #[arg(short, long, required = true)]
        device: String,
    },
}

/// Keyboard key types for input command
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum KeyType {
    /// Home button
    Home,
    /// Lock button
    Lock,
    /// Return/Enter key
    Return,
    /// Volume up
    VolumeUp,
    /// Volume down
    VolumeDown,
    /// Shake gesture
    Shake,
}
