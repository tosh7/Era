# Era

A fast and ergonomic iOS Simulator CLI tool written in Rust.

## Overview

Era provides a command-line interface for managing iOS Simulators, offering a more streamlined alternative to `xcrun simctl`. It supports common simulator operations like booting, shutting down, installing apps, and taking screenshots, as well as advanced input automation features.

## Installation

```bash
cargo install era
```

### Requirements

- macOS with Xcode and Simulator installed
- Rust toolchain (for building from source)

## Commands

| Command | Description |
|---------|-------------|
| `list` | List available simulators |
| `boot` | Boot a simulator |
| `shutdown` | Shutdown a simulator |
| `install` | Install an app on a simulator |
| `launch` | Launch an app on a simulator |
| `screenshot` | Take a screenshot of a simulator |
| `input` | Send keyboard input to a simulator |
| `openurl` | Open a URL in the simulator |
| `tap` | Tap on the simulator screen |
| `swipe` | Swipe on the simulator screen |
| `enumerate` | Enumerate available input devices |

## Usage Examples

### List Simulators

```bash
# List all simulators
era list

# List only booted simulators
era list --booted
```

### Boot and Shutdown

```bash
# Boot a simulator by name or UDID
era boot "iPhone 16 Pro"
era boot 12345678-ABCD-1234-ABCD-123456789ABC

# Shutdown a simulator
era shutdown "iPhone 16 Pro"

# Shutdown all simulators
era shutdown all
```

### App Management

```bash
# Install an app
era install -d "iPhone 16 Pro" /path/to/MyApp.app

# Launch an app by bundle ID
era launch -d "iPhone 16 Pro" com.example.myapp
```

### Screenshot

```bash
# Take a screenshot
era screenshot -d "iPhone 16 Pro" screenshot.png
```

### Input Simulation

```bash
# Send keyboard input
era input -d "iPhone 16 Pro" -k home
era input -d "iPhone 16 Pro" -k lock
era input -d "iPhone 16 Pro" -k return
era input -d "iPhone 16 Pro" -k volume-up
era input -d "iPhone 16 Pro" -k volume-down
era input -d "iPhone 16 Pro" -k shake
```

### URL Handling

```bash
# Open a URL in the simulator
era openurl -d "iPhone 16 Pro" -u "https://example.com"

# Open a deep link
era openurl -d "iPhone 16 Pro" -u "myapp://settings"
```

### Touch Automation

```bash
# Tap at coordinates
era tap -d "iPhone 16 Pro" -x 200 -y 400

# Swipe gesture
era swipe -d "iPhone 16 Pro" --start-x 100 --start-y 500 --end-x 100 --end-y 200
```

### Device Enumeration

```bash
# List input devices
era enumerate -d "iPhone 16 Pro"
```

## IDB Integration

Era optionally integrates with [idb (iOS Development Bridge)](https://fbidb.io/) for advanced UI automation features. While basic functionality works with `simctl` alone, idb provides more reliable touch input simulation.

### Installing IDB

```bash
brew install idb-companion
```

When idb is not installed, Era gracefully falls back to simctl-based operations where possible.

## Building from Source

```bash
git clone https://github.com/tosh7/Era.git
cd Era
cargo build --release
```

## License

MIT License
