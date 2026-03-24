// CLI commands definition

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

    /// Manage simulator sessions
    #[command(subcommand)]
    Session(SessionCommand),

    /// Show a ref-numbered UI element tree (requires IDB)
    ///
    /// Outputs a compact, Playwright-style snapshot of the current screen.
    /// Each element gets a [ref] number that can be used with `tap --ref` or `fill --ref`.
    Snapshot {
        /// Simulator device ID or name
        #[arg(short, long, conflicts_with = "session")]
        device: Option<String>,

        /// Session name (use instead of --device)
        #[arg(long, conflicts_with = "device")]
        session: Option<String>,

        /// Include frame coordinates in output
        #[arg(long = "show-frames")]
        show_frames: bool,

        /// Only show interactive (tappable/fillable) elements
        #[arg(long)]
        interactive: bool,

        /// Filter by element type (e.g. "Button", "TextField")
        #[arg(long)]
        filter: Option<String>,
    },

    /// Tap on the simulator screen (requires IDB)
    ///
    /// Coordinates are in logical points by default.
    /// Use --scale to convert pixel coordinates from screenshots.
    /// Use --ref to tap an element from the latest snapshot.
    /// Example: --scale 3 for iPhone Pro models (3x Retina)
    Tap {
        /// Simulator device ID or name
        #[arg(short, long, conflicts_with = "session")]
        device: Option<String>,

        /// Session name (use instead of --device)
        #[arg(long, conflicts_with = "device")]
        session: Option<String>,

        /// X coordinate (pixels if --scale is set, otherwise logical points)
        #[arg(short = 'x', long, required_unless_present = "ref_id", conflicts_with = "ref_id")]
        x: Option<u32>,

        /// Y coordinate (pixels if --scale is set, otherwise logical points)
        #[arg(short = 'y', long, required_unless_present = "ref_id", conflicts_with = "ref_id")]
        y: Option<u32>,

        /// Ref number from `era snapshot` output. Taps the center of the referenced element.
        #[arg(long = "ref", conflicts_with_all = ["x", "y", "scale"])]
        ref_id: Option<u32>,

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

    /// Fill text into a UI element by ref number (requires IDB)
    ///
    /// Taps the referenced element to focus it, then inputs the specified text.
    /// Use `era snapshot` first to get ref numbers.
    Fill {
        /// Simulator device ID or name
        #[arg(short, long, conflicts_with = "session")]
        device: Option<String>,

        /// Session name (use instead of --device)
        #[arg(long, conflicts_with = "device")]
        session: Option<String>,

        /// Ref number from `era snapshot` output
        #[arg(long = "ref", required = true)]
        ref_id: u32,

        /// Text to input
        #[arg(required = true)]
        text: String,

        /// Clear existing text before input (triple-tap to select all, then replace)
        #[arg(long)]
        clear: bool,

        /// Disable automatic retry for the initial tap.
        #[arg(long)]
        no_retry: bool,

        /// Screenshot observation policy for retry diagnostics.
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
        #[arg(short, long, conflicts_with = "session")]
        device: Option<String>,

        /// Session name (use instead of --device)
        #[arg(long, conflicts_with = "device")]
        session: Option<String>,

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
        #[arg(short, long, conflicts_with = "session")]
        device: Option<String>,

        /// Session name (use instead of --device)
        #[arg(long, conflicts_with = "device")]
        session: Option<String>,

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

/// Session management subcommands
#[derive(Subcommand)]
pub enum SessionCommand {
    /// Connect to a simulator device and create a session
    Connect {
        /// Session name
        #[arg(long, default_value = "default")]
        name: String,

        /// Simulator device ID or name
        #[arg(short, long, required = true)]
        device: String,
    },

    /// List all active sessions
    List,

    /// Disconnect a session
    Disconnect {
        /// Session name to disconnect
        #[arg(long, required = true)]
        name: String,
    },

    /// Disconnect all sessions
    #[command(name = "disconnect-all")]
    DisconnectAll,
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
