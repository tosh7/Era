# Coordinate Conversion

## Overview

iOS Simulators use logical point coordinates, but screenshots and UI tools often report pixel coordinates. Era's `--scale` option handles the conversion automatically.

## Scale Factors

| Device | Scale Factor | Screen Size (pixels) | Screen Size (points) |
|--------|-------------|---------------------|---------------------|
| iPhone SE | 2x | 750 x 1334 | 375 x 667 |
| iPhone 16 | 3x | 1179 x 2556 | 393 x 852 |
| iPhone 16 Pro | 3x | 1206 x 2622 | 402 x 874 |
| iPhone 16 Pro Max | 3x | 1320 x 2868 | 440 x 956 |
| iPhone Air | 3x | 1260 x 2736 | 420 x 912 |
| iPad Air | 2x | 1640 x 2360 | 820 x 1180 |

## Usage

```bash
# Without --scale: coordinates are treated as points
era tap -d booted -x 200 -y 400

# With --scale: coordinates are treated as pixels and auto-converted
era tap -d booted -x 600 -y 1200 --scale 3
# equivalent to: era tap -d booted -x 200 -y 400

# Same for swipe
era swipe -d booted --start-x 300 --start-y 1500 --end-x 300 --end-y 600 --scale 3
```

## Auto-Detection

When `--scale` is not specified but the device is known, Era auto-detects the scale factor. This works for most common devices.

## Formula

```
point = pixel / scale_factor
```

Example: Tap at pixel (1260, 2736) on a 3x device:
- point_x = 1260 / 3 = 420
- point_y = 2736 / 3 = 912
