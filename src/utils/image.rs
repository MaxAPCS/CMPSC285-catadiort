//! image.rs: writing the rendered image (HW2 handout: the interface) and
//! reading the images textures come from (read_image, at the end). Complete
//! in both kits; nothing to change; the twin of the C++ kit's image.cpp. The
//! PNG is what you look
//! at and what the benchmark compares: linear radiance times 2^exposure,
//! through the scene's tone map (rtTonemap: a clamp to [0, 1] by default,
//! or a curve with a shoulder that keeps the bright parts of an image whose
//! sun is a thousand times its shade), sRGB encoded, 8 bits per channel,
//! no alpha, dithered. The PFM (--pfm only) keeps the floats as rendered,
//! for compare.py and the HW3 studies.

use std::io::Write;
use std::sync::OnceLock;

use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use ultraviolet::Vec3;

#[inline]
pub fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// Its inverse, for 8-bit textures loaded with srgb = 1.
#[inline]
pub fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Tables for the sRGB encode, built once: the code at the start of each of
/// 4096 equal bins of [0, 1] (the one just below the bin's start), and per
/// code its exact linear value. The curve is steepest at 0, where a bin
/// spans 0.8 codes, so a bin never holds more than two codes: one comparison.
struct EncodeTable {
    code: Vec<u8>,
    level: Vec<f32>,
}

fn table() -> &'static EncodeTable {
    static TABLE: OnceLock<EncodeTable> = OnceLock::new();
    TABLE.get_or_init(|| EncodeTable {
        code: (0..4096)
            .map(|i| (linear_to_srgb(i as f32 / 4096.0) * 255.0).floor() as u8)
            .collect(),
        level: (0..=256)
            .map(|c| srgb_to_linear(c as f32 / 255.0))
            .collect(),
    })
}

/// A linear value clamped to [0, 1], sRGB encoded to 8 bits with a dither:
/// the code just below it, plus one when u in [0, 1) falls under the
/// value's position between the two codes, so the code's expectation is the
/// value and a slow gradient in a dark region breaks into fine noise instead
/// of bands (u = 0.5 is plain rounding).
fn to8(v: f32, u: f32) -> u8 {
    let t = table();
    if !(v > 0.0) {
        return 0; /* also NaN */
    }
    if v >= 1.0 {
        return 255;
    }
    let mut c = t.code[(v * 4096.0) as usize] as usize;
    if c < 255 && v >= t.level[c + 1] {
        c += 1;
    }
    if c < 255 && u < (v - t.level[c]) / (t.level[c + 1] - t.level[c]) {
        c += 1;
    }
    c as u8
}

/* The tone maps, per channel on linear radiance x >= 0, each 0 at 0 and
 * rising to 1 with a shoulder instead of the clamp's hard edge. */
#[inline]
fn tone_none(x: f32) -> f32 {
    x
}
#[inline]
fn tone_reinhard(x: f32) -> f32 {
    x / (1.0 + x) /* Reinhard et al. 2002, the simple form */
}
#[inline]
fn tone_aces(x: f32) -> f32 {
    /* Narkowicz 2015, a fit of the ACES filmic curve */
    (x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)
}
#[inline]
fn tone_filmic(x: f32) -> f32 {
    /* Hable 2010 ("Uncharted 2"), white at 11.2 */
    const A: f32 = 0.15;
    const B: f32 = 0.50;
    const C: f32 = 0.10;
    const D: f32 = 0.20;
    const E: f32 = 0.02;
    const F: f32 = 0.30;
    const W: f32 = 11.2;
    let num = (x * (A * x + C * B) + D * E) / (x * (A * x + B) + D * F) - E / F;
    let white = (W * (A * W + C * B) + D * E) / (W * (A * W + B) + D * F) - E / F;
    num / white
}

fn tone_map(name: &str) -> fn(f32) -> f32 {
    match name {
        "reinhard" => tone_reinhard,
        "aces" => tone_aces,
        "filmic" => tone_filmic,
        "none" | "" => tone_none,
        other => {
            eprintln!(
                "rtTonemap: none, reinhard, aces or filmic, not '{}'; clamping",
                other
            );
            tone_none
        }
    }
}

/// A number in [0, 1) from a pixel's index (a 32-bit integer hash), the
/// same for its three channels: the dither above.
fn pixel_noise(pixel: usize) -> f32 {
    let mut h = (pixel as u32).wrapping_mul(2654435761);
    h ^= h >> 15;
    h = h.wrapping_mul(2246822519);
    h ^= h >> 13;
    h = h.wrapping_mul(3266489917);
    h ^= h >> 16;
    (h >> 8) as f32 * (1.0 / 16777216.0)
}

/// The 8-bit image, PNG or JPEG by format: exposure, tone map, sRGB and
/// the dither below.
fn write_8bit(
    path: &str,
    w: u32,
    h: u32,
    rgb: &Box<[Vec3]>,
    tonemap: &str,
    exposure: f32,
    format: &str,
) -> bool {
    let tone = tone_map(tonemap);
    let scale = 2.0f32.powf(exposure);
    /* Dithered: a value between two 8-bit levels goes to the upper one with
     * the probability of its position between them, decided by a hash of
     * the pixel, so a slow gradient in a dark region breaks into fine noise
     * instead of bands (a deterministic hash: the same image every run).
     * The PFM carries the exact values. */
    let bytes: Vec<u8> = rgb
        .par_iter()
        .enumerate()
        .flat_map_iter(|(i, &v)| {
            v.as_array()
                .map(|v| to8(tone(if v > 0.0 { v * scale } else { 0.0 }), pixel_noise(i)))
        })
        .collect();
    /* The fastest deflate and no row filtering: a slow PNG write would show
     * in the TIME line's total (the benchmark ranks by build + render). A
     * JPEG at quality 92, as the C++ kit's stb_image_write writes it. */
    use image::ImageEncoder;
    use image::codecs::jpeg::JpegEncoder;
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    let file = match std::fs::File::create(path) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("could not write {} (run gauchoRT from the kit root)", path);
            return false;
        }
    };
    let out = std::io::BufWriter::new(file);
    let color = image::ExtendedColorType::Rgb8;
    let written = if format == "jpg" {
        JpegEncoder::new_with_quality(out, 92).write_image(&bytes, w, h, color)
    } else {
        PngEncoder::new_with_quality(out, CompressionType::Fast, FilterType::NoFilter)
            .write_image(&bytes, w, h, color)
    };
    if written.is_err() {
        eprintln!("could not write {}", path);
        return false;
    }
    println!("wrote {} ({} x {})", path, w, h);
    true
}

/// Portable float map: "PF", the size, a negative scale (meaning
/// little-endian), then the rows bottom to top, three floats per pixel.
#[cfg(not(target_endian = "little"))]
fn write_pfm(
    path: &str,
    w: usize,
    h: usize,
    rgb: &Box<[Vec3]>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(path)?;
    let mut out = std::io::BufWriter::new(file);
    write!(out, "PF\n{} {}\n-1.0\n", w, h)?;
    for y in (0..h).rev() {
        out.write_all(
            &rgb[y * w..(y + 1) * w]
                .iter()
                .flat_map(|v| v.as_array().iter().flat_map(|v| v.to_le_bytes()))
                .collect::<Vec<_>>(),
        )?;
    }
    out.flush()?;
    Ok(())
}

#[cfg(target_endian = "little")]
fn write_pfm(
    path: &str,
    w: usize,
    h: usize,
    rgb: &Box<[Vec3]>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(path)?;
    let mut out = std::io::BufWriter::new(file);
    write!(out, "PF\n{} {}\n-1.0\n", w, h)?;
    for y in (0..h).rev() {
        out.write_all(bytemuck::cast_slice(&rgb[y * w..(y + 1) * w]))?;
    }
    out.flush()?;
    Ok(())
}

/// Write w x h pixels of linear radiance (three floats per pixel, top row
/// first) as <path>.png, or <path>.jpg with format "jpg" (--format):
/// scaled by 2^exposure (stops), through the tone map ("none": clamped to
/// [0, 1]; "reinhard", "aces" or "filmic": a curve with a shoulder, as
/// rt.h's rtTonemap says), then sRGB encoded; and with pfm as <path>.pfm
/// with the floats as they are, linear, before exposure and tone map. The
/// folder must exist (cmdline.rs's image_path makes it). True if every file
/// was written, else stderr says.
#[allow(clippy::too_many_arguments)] // the C++ kit's writeImage, argument for argument
pub fn write_image(
    path: &str,
    w: i32,
    h: i32,
    rgb: &Box<[Vec3]>,
    pfm: bool,
    tonemap: &str,
    exposure: f32,
    format: &str,
) -> bool {
    let mut ok = write_8bit(
        &format!("{}.{}", path, format),
        w as _,
        h as _,
        rgb,
        tonemap,
        exposure,
        format,
    );
    if pfm {
        ok &= write_pfm(&format!("{}.pfm", path), w as _, h as _, rgb).is_ok();
    }
    ok
}

/* reading ---------------------------------------------------------------- */

/// An image read from a file: linear floats, top row first, the channels
/// interleaved (1 grey, 2 grey and alpha, 3 RGB, 4 RGBA).
pub struct Image {
    pub width: i32,
    pub height: i32,
    pub channels: i32,
    pub texels: Vec<f32>,
}

/// Lowercase extension of a file name, without the dot ("" if none).
fn extension_of(name: &str) -> String {
    match name.rfind('.') {
        Some(dot) => name[dot + 1..].to_ascii_lowercase(),
        None => String::new(),
    }
}

/// Portable float map: "PF" (RGB) or "Pf" (grey), width height, a scale
/// whose sign gives the byte order (negative: little-endian), then the rows
/// bottom to top.
fn read_pfm(path: &str) -> Option<Image> {
    let bytes = std::fs::read(path).ok()?;
    /* the header: four words separated by whitespace, then one whitespace byte */
    let mut pos = 0;
    let mut words: Vec<String> = Vec::new();
    while words.len() < 4 {
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let start = pos;
        while pos < bytes.len() && !bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if start == pos {
            return None;
        }
        words.push(String::from_utf8_lossy(&bytes[start..pos]).into_owned());
    }
    pos += 1; /* the single whitespace byte before the data */
    let channels = match words[0].as_str() {
        "PF" => 3,
        "Pf" => 1,
        _ => return None,
    };
    let w: usize = words[1].parse().ok()?;
    let h: usize = words[2].parse().ok()?;
    let scale: f32 = words[3].parse().ok()?;
    let row = w * channels;
    if w == 0 || h == 0 || scale == 0.0 || bytes.len() < pos + w * h * channels * 4 {
        return None;
    }
    let mut texels = vec![0.0f32; w * h * channels];
    for y in 0..h {
        /* the file's first row is the bottom one */
        let src = &bytes[pos + (h - 1 - y) * row * 4..pos + (h - y) * row * 4];
        for (k, b) in src.chunks_exact(4).enumerate() {
            let b = [b[0], b[1], b[2], b[3]];
            texels[y * row + k] = if scale < 0.0 {
                f32::from_le_bytes(b)
            } else {
                f32::from_be_bytes(b)
            };
        }
    }
    Some(Image {
        width: w as i32,
        height: h as i32,
        channels: channels as i32,
        texels,
    })
}

/// Read an image file into linear floats: PNG, JPEG, BMP, TGA and Radiance
/// HDR through the image crate, PFM ("PF" RGB or "Pf" grey) by hand. The
/// color channels of an 8-bit image are decoded from sRGB when srgb is set,
/// its alpha never; HDR and PFM are linear as stored (a PFM's scale gives
/// only the byte order). None, with a message on stderr, when the file
/// cannot be read. rtMakeTexture is built on this.
pub fn read_image(path: &str, srgb: bool) -> Option<Image> {
    let ext = extension_of(path);
    let image = if ext == "pfm" {
        read_pfm(path).ok_or_else(|| "bad PFM header or short file".to_string())
    } else {
        image::open(path).map_err(|e| e.to_string()).map(|img| {
            let (w, h) = (img.width() as i32, img.height() as i32);
            if ext == "hdr" {
                /* linear floats, three channels */
                return Image {
                    width: w,
                    height: h,
                    channels: 3,
                    texels: img.into_rgb32f().into_raw(),
                };
            }
            /* 8 bits per channel, as many channels as the file has */
            let n = (img.color().channel_count() as usize).clamp(1, 4);
            let raw = match n {
                1 => img.into_luma8().into_raw(),
                2 => img.into_luma_alpha8().into_raw(),
                3 => img.into_rgb8().into_raw(),
                _ => img.into_rgba8().into_raw(),
            };
            let alpha = if n == 2 || n == 4 { n - 1 } else { usize::MAX };
            let texels = raw
                .iter()
                .enumerate()
                .map(|(i, &b)| {
                    let v = b as f32 / 255.0;
                    if srgb && i % n != alpha {
                        srgb_to_linear(v)
                    } else {
                        v
                    }
                })
                .collect();
            Image {
                width: w,
                height: h,
                channels: n as i32,
                texels,
            }
        })
    };
    match image {
        Ok(img) => Some(img),
        Err(why) => {
            eprintln!("readImage: cannot read {} ({})", path, why);
            None
        }
    }
}
