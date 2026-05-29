---
name: era
description: Automates iOS Simulator interactions for app testing, UI automation, screenshots, and input simulation. Use when the user needs to manage simulators, tap/swipe on screens, input text, take screenshots, or test iOS apps on simulators.
allowed-tools: Bash(era:*)
---

# iOS Simulator Automation with Era

## Quick start

```bash
# list available simulators
era list
# boot a simulator
era boot "iPhone 16 Pro"
# take a screenshot
era screenshot -d booted screenshot.png
# tap at coordinates
era tap -d booted -x 200 -y 400
# shutdown
era shutdown "iPhone 16 Pro"
```

## Commands

### Simulator Management

```bash
era list
era list --booted
era boot "iPhone 16 Pro"
era boot 12345678-ABCD-1234-ABCD-123456789ABC
era shutdown "iPhone 16 Pro"
era shutdown all
```

### App Management

```bash
era install -d "iPhone 16 Pro" /path/to/MyApp.app
era launch -d "iPhone 16 Pro" com.example.myapp
```

### Screenshot

```bash
era screenshot -d "iPhone 16 Pro" screenshot.png
era screenshot -d booted output.png
```

### Touch Automation

```bash
# tap at point coordinates
era tap -d booted -x 200 -y 400

# tap at pixel coordinates (auto-converted with --scale)
era tap -d booted -x 1260 -y 2736 --scale 3

# long press
era longpress -d booted -x 200 -y 400 --duration 2.0
era longpress -d booted -x 600 -y 1200 --scale 3 --duration 1.5

# swipe (point coordinates)
era swipe -d booted --start-x 100 --start-y 500 --end-x 100 --end-y 200

# swipe (pixel coordinates)
era swipe -d booted --start-x 300 --start-y 1500 --end-x 300 --end-y 600 --scale 3
```

### Text Input

```bash
# type text into focused field (requires IDB)
era text -d booted "Hello, World!"

# send keyboard input
era input -d booted -k home
era input -d booted -k lock
era input -d booted -k return
era input -d booted -k volume-up
era input -d booted -k volume-down
era input -d booted -k shake

# send raw key event (requires IDB)
era key -d booted 42
```

### UI Inspection

```bash
# get full accessibility tree as JSON (requires IDB)
era describe -d booted

# get ref-numbered UI snapshot (requires IDB)
era snapshot -d booted
era snapshot -d booted --interactive
era snapshot -d booted --filter "Button"
era snapshot -d booted --show-frames
```

### Ref-Based Interaction

After taking a snapshot, interact with elements using ref numbers:

```bash
era snapshot -d booted
# use ref numbers from snapshot output
era tap -d booted --ref 42
era fill -d booted --ref 5 "user@example.com"
```

### URL Handling

```bash
era openurl -d booted -u "https://example.com"
era openurl -d booted -u "myapp://settings"
```

### Device Enumeration

```bash
era enumerate -d booted
```

### Image / Visual Diff

Image utilities for before/after visual regression (no ImageMagick needed):

```bash
# compare two images; prints AE = differing pixel count, exits 2 if > --threshold
era compare before.png after.png

# compare only a region (WxH+X+Y, pixels) and write a highlighted diff image
era compare before.png after.png --region 1206x250+0+330 --out diff.png

# allow minor anti-aliasing noise
era compare before.png after.png --fuzz 2 --threshold 50

# combine images into a grid (--tile 2x1 = side-by-side, 1x3 = stacked)
era montage before.png after.png --tile 2x1 -o montage.png

# crop a rectangular region
era crop screenshot.png 1206x150+0+478 -o cropped.png
```

## Coordinate Conversion

The `--scale` option converts pixel coordinates to logical points automatically. Use this when working with coordinates from screenshots.

| Device | Scale Factor |
|--------|--------------|
| Standard displays (iPhone SE, etc.) | 2 |
| Super Retina displays (iPhone 16 Pro, etc.) | 3 |
| iPhone Air | 3 |

Formula: `point = pixel / scale`

## Snapshot Workflow

The recommended workflow for UI automation:

```bash
# 1. take a snapshot to see available UI elements with ref numbers
era snapshot -d booted

# 2. interact using ref numbers (no coordinate calculation needed)
era tap -d booted --ref 3
era fill -d booted --ref 7 "test@example.com"
era tap -d booted --ref 12

# 3. verify the result
era snapshot -d booted
```

This eliminates coordinate calculation errors entirely.

## Example: App Login Flow

```bash
era boot "iPhone 16 Pro"
era launch -d booted com.example.myapp
era screenshot -d booted before-login.png

era snapshot -d booted
era fill -d booted --ref 3 "user@example.com"
era fill -d booted --ref 5 "password123"
era tap -d booted --ref 8

era screenshot -d booted after-login.png
era shutdown "iPhone 16 Pro"
```

## Example: Scroll and Tap

```bash
era boot "iPhone 16 Pro"
era launch -d booted com.example.myapp

# scroll down
era swipe -d booted --start-x 200 --start-y 600 --end-x 200 --end-y 200

# take snapshot and tap target element
era snapshot -d booted
era tap -d booted --ref 15

era shutdown "iPhone 16 Pro"
```

## Example: Deep Link Testing

```bash
era boot "iPhone 16 Pro"
era openurl -d booted -u "myapp://product/12345"
era screenshot -d booted deeplink-result.png
era shutdown "iPhone 16 Pro"
```

## Example: Before/After Visual Regression

Verify a refactor renders identically by driving the same flow on two builds and
comparing screenshots. Build once per version; capturing different states needs no
rebuild — just operate the installed app:

```bash
# 1. install + capture the BEFORE build (drive a FIXED tap recipe, screenshot each step)
era install -d booted /path/to/Before.app
era launch  -d booted com.example.myapp
era screenshot -d booted before_step1.png

# 2. install + capture the AFTER build with the SAME recipe
era install -d booted /path/to/After.app
era launch  -d booted com.example.myapp
era screenshot -d booted after_step1.png

# 3. compare the component region (ignores status-bar clock / dynamic content) + montage
era compare before_step1.png after_step1.png --region 1206x250+0+330 --out diff_step1.png
era montage before_step1.png after_step1.png --tile 2x1 -o montage_step1.png
```

Keep the tap sequence identical for both runs (a fixed "recipe"); crop to the
component with `--region` so unrelated areas don't add noise; `AE=0` means
pixel-identical; use `--fuzz` to tolerate anti-aliasing.

## Requirements

- macOS with Xcode and Simulator installed
- [idb-companion](https://fbidb.io/) for advanced features (tap, swipe, text, describe, snapshot)

```bash
brew install idb-companion
```

## Specific tasks

* **Snapshot and ref-based interaction** [references/snapshot-workflow.md](references/snapshot-workflow.md)
* **Coordinate conversion** [references/coordinate-conversion.md](references/coordinate-conversion.md)
* **IDB integration** [references/idb-integration.md](references/idb-integration.md)
