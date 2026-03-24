// CLI commands definition - Sub1担当

use clap::{Parser, Subcommand, ValueEnum};

use crate::capture::ObservationPolicy;

#[derive(Parser)]
#[command(name = "era")]
#[command(about = "iOS Simulator CLI tool", long_about = None)]
pub struct Cli {
    /// Increase logging verbosity (-v for info, -vv for debug)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Save debug screenshots to disk during tap operations
    #[arg(long, global = true)]
    pub debug_capture: bool,

    /// Directory for debug screenshots
    #[arg(long, global = true, default_value = "/tmp/era-debug/")]
    pub debug_dir: String,

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

        /// Disable automatic retry with UI state verification.
        /// When set, performs a single tap without checking if the UI changed.
        #[arg(long)]
        no_retry: bool,

        /// Screenshot observation policy for retry diagnostics.
        /// Requires --debug-capture to save screenshots to disk.
        #[arg(long, value_enum, default_value = "on-failure")]
        observe: ObservationPolicy,
    },

    /// Tap within a rectangular region on the simulator screen (requires IDB)
    ///
    /// Taps near the center of the specified region with small jitter.
    /// Coordinates are in logical points by default.
    /// Use --scale to convert pixel coordinates from screenshots.
    #[command(name = "tap-region")]
    TapRegion {
        /// Simulator device ID or name
        #[arg(short, long, required = true)]
        device: String,

        /// Left edge X coordinate (pixels if --scale is set, otherwise logical points)
        #[arg(short = 'x', long, required = true)]
        x: u32,

        /// Top edge Y coordinate (pixels if --scale is set, otherwise logical points)
        #[arg(short = 'y', long, required = true)]
        y: u32,

        /// Region width (pixels if --scale is set, otherwise logical points)
        #[arg(short = 'W', long, required = true)]
        width: u32,

        /// Region height (pixels if --scale is set, otherwise logical points)
        #[arg(short = 'H', long, required = true)]
        height: u32,

        /// Scale factor to convert pixel coordinates to logical points.
        /// Use 2 for 2x Retina (iPhone SE), 3 for 3x Retina (iPhone Pro).
        /// When set, all coordinates are treated as pixel values from screenshots.
        #[arg(short, long)]
        scale: Option<u32>,

        /// Disable automatic retry with UI state verification.
        /// When set, performs a single tap without checking if the UI changed.
        #[arg(long)]
        no_retry: bool,

        /// Screenshot observation policy for retry diagnostics.
        /// Requires --debug-capture to save screenshots to disk.
        #[arg(long, value_enum, default_value = "on-failure")]
        observe: ObservationPolicy,
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

    /// Input text into the currently focused text field (requires IDB)
    ///
    /// Sends the specified text string to the simulator as if typed on a keyboard.
    /// A text field must be focused before running this command.
    Text {
        /// Simulator device ID or name
        #[arg(short, long, required = true)]
        device: String,

        /// Text string to input
        #[arg(required = true)]
        text: String,
    },

    /// Retrieve the UI element tree from the simulator (requires IDB)
    ///
    /// Returns the full accessibility tree as JSON via `idb ui describe-all`.
    /// Useful for inspecting UI state and finding element coordinates.
    Describe {
        /// Simulator device ID or name
        #[arg(short, long, required = true)]
        device: String,
    },

    /// Long press on the simulator screen (requires IDB)
    ///
    /// Performs a tap-and-hold gesture at the specified coordinates.
    /// Coordinates are in logical points by default.
    /// Use --scale to convert pixel coordinates from screenshots.
    Longpress {
        /// Simulator device ID or name
        #[arg(short, long, required = true)]
        device: String,

        /// X coordinate (pixels if --scale is set, otherwise logical points)
        #[arg(short = 'x', long, required = true)]
        x: u32,

        /// Y coordinate (pixels if --scale is set, otherwise logical points)
        #[arg(short = 'y', long, required = true)]
        y: u32,

        /// Press duration in seconds
        #[arg(long, default_value = "1.0")]
        duration: f64,

        /// Scale factor to convert pixel coordinates to logical points.
        /// Use 2 for 2x Retina (iPhone SE), 3 for 3x Retina (iPhone Pro).
        /// When set, x and y are treated as pixel coordinates from screenshots.
        #[arg(short, long)]
        scale: Option<u32>,
    },

    /// Send a raw key event to the simulator (requires IDB)
    ///
    /// Sends a key code or key name via `idb ui key`.
    /// For hardware buttons (HOME, LOCK), use the `input` command instead.
    Key {
        /// Simulator device ID or name
        #[arg(short, long, required = true)]
        device: String,

        /// Key code (integer) or key name to send
        #[arg(required = true)]
        key: String,
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
