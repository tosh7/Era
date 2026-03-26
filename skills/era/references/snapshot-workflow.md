# Snapshot and Ref-Based Interaction

## Overview

Era's snapshot system provides a numbered reference (ref) for each UI element, eliminating the need for manual coordinate calculation.

## Basic Workflow

```bash
# Step 1: Take a snapshot
era snapshot -d booted
```

Output example:
```
[ref=1] StaticText "Welcome"
[ref=2] TextField "Email"
[ref=3] SecureTextField "Password"
[ref=4] Button "Sign In"
[ref=5] Button "Forgot Password?"
```

```bash
# Step 2: Interact using ref numbers
era fill -d booted --ref 2 "user@example.com"
era fill -d booted --ref 3 "mypassword"
era tap -d booted --ref 4
```

## Snapshot Options

```bash
# show only interactive elements (buttons, text fields, etc.)
era snapshot -d booted --interactive

# filter by element type
era snapshot -d booted --filter "Button"

# show frame coordinates alongside refs
era snapshot -d booted --show-frames
```

## Ref vs Coordinate

| Method | Pros | Cons |
|--------|------|------|
| `--ref 42` | No calculation, layout-independent | Requires snapshot first |
| `-x 200 -y 400` | Direct, no snapshot needed | Fragile to layout changes |
| `-x 600 -y 1200 --scale 3` | Works with pixel values | Requires scale knowledge |

## Tips

- Refs are regenerated on each `snapshot` call. Always take a fresh snapshot before interacting.
- Use `--interactive` to reduce noise when looking for tappable elements.
- Use `--filter` to narrow down specific element types.
- If a ref-based tap doesn't work, use `--show-frames` to verify the element's position.
