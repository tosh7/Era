//! Image utilities for visual regression: compare (AE diff), montage (grid
//! composite) and crop. Operates on PNG files produced by `era screenshot`.
//!
//! These let `era` do before/after visual checks on its own, without depending
//! on external tools such as ImageMagick.

use std::error::Error;

use image::{Rgba, RgbaImage};

/// Result of an image comparison.
pub struct CompareResult {
    /// Number of differing pixels.
    pub diff: u64,
    /// Total number of compared pixels.
    pub total: u64,
}

/// Parse a region string `WxH+X+Y` (all in pixels).
fn parse_region(s: &str) -> Result<(u32, u32, u32, u32), Box<dyn Error>> {
    let err = || -> Box<dyn Error> { format!("invalid region '{s}', expected WxH+X+Y").into() };
    let (wh, rest) = s.split_once('+').ok_or_else(err)?;
    let (x, y) = rest.split_once('+').ok_or_else(err)?;
    let (w, h) = wh.split_once('x').ok_or_else(err)?;
    Ok((w.parse()?, h.parse()?, x.parse()?, y.parse()?))
}

/// Parse a hex color `rrggbb` or `rrggbbaa` (an optional leading `#` is allowed).
fn parse_hex(s: &str) -> Result<Rgba<u8>, Box<dyn Error>> {
    let s = s.trim_start_matches('#');
    let byte = |i: usize| u8::from_str_radix(&s[i..i + 2], 16);
    match s.len() {
        6 => Ok(Rgba([byte(0)?, byte(2)?, byte(4)?, 255])),
        8 => Ok(Rgba([byte(0)?, byte(2)?, byte(4)?, byte(6)?])),
        _ => Err(format!("invalid hex color '{s}', expected rrggbb or rrggbbaa").into()),
    }
}

/// Parse a tile string `COLSxROWS`.
fn parse_tile(s: &str) -> Result<(u32, u32), Box<dyn Error>> {
    let err = || -> Box<dyn Error> { format!("invalid tile '{s}', expected COLSxROWS").into() };
    let (c, r) = s.split_once('x').ok_or_else(err)?;
    Ok((c.parse()?, r.parse()?))
}

/// Compare two images, counting pixels that differ beyond `fuzz` (per-channel
/// tolerance, 0-255). When `region` (`WxH+X+Y`) is given, only that rectangle is
/// compared on both images. When `out` is given, a diff image (changed pixels in
/// red over a faded original) is written there.
pub fn compare(
    before: &str,
    after: &str,
    region: Option<&str>,
    out: Option<&str>,
    fuzz: u8,
) -> Result<CompareResult, Box<dyn Error>> {
    let mut a = image::open(before)?.to_rgba8();
    let mut b = image::open(after)?.to_rgba8();

    if let Some(r) = region {
        let (w, h, x, y) = parse_region(r)?;
        a = image::imageops::crop_imm(&a, x, y, w, h).to_image();
        b = image::imageops::crop_imm(&b, x, y, w, h).to_image();
    }

    if a.dimensions() != b.dimensions() {
        return Err(format!(
            "image size mismatch: {:?} vs {:?} (use a common --region to compare)",
            a.dimensions(),
            b.dimensions()
        )
        .into());
    }

    let (w, h) = a.dimensions();
    let total = w as u64 * h as u64;
    let fuzz = fuzz as i16;
    let mut diff = 0u64;
    let mut diff_img = out.map(|_| RgbaImage::new(w, h));

    for y in 0..h {
        for x in 0..w {
            let pa = a.get_pixel(x, y).0;
            let pb = b.get_pixel(x, y).0;
            let differs = (0..4).any(|i| (pa[i] as i16 - pb[i] as i16).abs() > fuzz);
            if differs {
                diff += 1;
            }
            if let Some(img) = diff_img.as_mut() {
                let px = if differs {
                    Rgba([255, 32, 32, 255])
                } else {
                    // faded original for context
                    Rgba([
                        (pa[0] / 2).saturating_add(128),
                        (pa[1] / 2).saturating_add(128),
                        (pa[2] / 2).saturating_add(128),
                        255,
                    ])
                };
                img.put_pixel(x, y, px);
            }
        }
    }

    if let (Some(img), Some(path)) = (diff_img, out) {
        img.save(path)?;
    }

    Ok(CompareResult { diff, total })
}

/// Composite images into a grid. Tiles are placed left-to-right, top-to-bottom,
/// centered in equal-sized cells. `tile` is `COLSxROWS` (defaults to a single
/// row). `spacing` pixels are added between and around cells; `background` is a
/// hex color.
pub fn montage(
    inputs: &[String],
    out: &str,
    tile: Option<&str>,
    spacing: u32,
    background: &str,
) -> Result<(), Box<dyn Error>> {
    if inputs.is_empty() {
        return Err("montage requires at least one input image".into());
    }

    let images: Vec<RgbaImage> = inputs
        .iter()
        .map(|p| image::open(p).map(|i| i.to_rgba8()))
        .collect::<Result<_, _>>()?;

    let (cols, rows) = match tile {
        Some(t) => parse_tile(t)?,
        None => (images.len() as u32, 1),
    };
    if cols == 0 || rows == 0 {
        return Err("tile cols/rows must be >= 1".into());
    }

    let cell_w = images.iter().map(|i| i.width()).max().unwrap_or(0);
    let cell_h = images.iter().map(|i| i.height()).max().unwrap_or(0);
    let bg = parse_hex(background)?;

    let canvas_w = cols * cell_w + spacing * (cols + 1);
    let canvas_h = rows * cell_h + spacing * (rows + 1);
    let mut canvas = RgbaImage::from_pixel(canvas_w, canvas_h, bg);

    for (idx, img) in images.iter().enumerate() {
        let idx = idx as u32;
        let (col, row) = (idx % cols, idx / cols);
        if row >= rows {
            break; // ignore extras that do not fit the requested grid
        }
        let cell_x = spacing + col * (cell_w + spacing);
        let cell_y = spacing + row * (cell_h + spacing);
        // center the image within its cell
        let ox = cell_x + (cell_w - img.width()) / 2;
        let oy = cell_y + (cell_h - img.height()) / 2;
        image::imageops::overlay(&mut canvas, img, ox as i64, oy as i64);
    }

    canvas.save(out)?;
    Ok(())
}

/// Crop a rectangular `WxH+X+Y` region (pixels) out of `input` into `out`.
pub fn crop(input: &str, region: &str, out: &str) -> Result<(), Box<dyn Error>> {
    let (w, h, x, y) = parse_region(region)?;
    let img = image::open(input)?.to_rgba8();
    let cropped = image::imageops::crop_imm(&img, x, y, w, h).to_image();
    cropped.save(out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_parses() {
        assert_eq!(parse_region("1206x250+0+330").unwrap(), (1206, 250, 0, 330));
        assert!(parse_region("bad").is_err());
        assert!(parse_region("10x10+5").is_err());
    }

    #[test]
    fn hex_parses() {
        assert_eq!(parse_hex("cccccc").unwrap(), Rgba([0xcc, 0xcc, 0xcc, 255]));
        assert_eq!(parse_hex("#2ecc71").unwrap(), Rgba([0x2e, 0xcc, 0x71, 255]));
        assert_eq!(parse_hex("00112233").unwrap(), Rgba([0, 0x11, 0x22, 0x33]));
        assert!(parse_hex("xyz").is_err());
        assert!(parse_hex("12345").is_err());
    }

    #[test]
    fn tile_parses() {
        assert_eq!(parse_tile("2x1").unwrap(), (2, 1));
        assert_eq!(parse_tile("1x3").unwrap(), (1, 3));
        assert!(parse_tile("2").is_err());
    }
}
