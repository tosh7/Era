# IDB Integration

## Overview

Era uses [idb (iOS Development Bridge)](https://fbidb.io/) for advanced UI automation features. IDB is optional — basic simulator management works without it.

## Installation

```bash
brew install idb-companion
```

## Commands Requiring IDB

| Command | Description |
|---------|-------------|
| `tap` | Tap on screen coordinates or ref |
| `swipe` | Swipe gesture |
| `longpress` | Long press gesture |
| `text` | Type text into focused field |
| `key` | Send raw key events |
| `describe` | Get full accessibility tree (JSON) |
| `snapshot` | Get ref-numbered UI element list |
| `fill` | Fill text field by ref |

## Commands Without IDB

| Command | Description |
|---------|-------------|
| `list` | List simulators |
| `boot` | Boot simulator |
| `shutdown` | Shutdown simulator |
| `install` | Install app |
| `launch` | Launch app |
| `screenshot` | Take screenshot |
| `input` | Keyboard input (home, lock, etc.) |
| `openurl` | Open URL |
| `enumerate` | List input devices |

## Troubleshooting

### idb_companion not found
```bash
# verify installation
which idb_companion

# reinstall if needed
brew reinstall idb-companion
```

### Tap not working
- Use `era describe -d booted` to verify the accessibility tree is accessible
- Check that the coordinates are within the screen bounds
- Use `era snapshot -d booted --show-frames` to verify element positions

### Connection issues
- Ensure the simulator is booted: `era list --booted`
- idb_companion connects automatically to booted simulators
- If issues persist, try rebooting the simulator
