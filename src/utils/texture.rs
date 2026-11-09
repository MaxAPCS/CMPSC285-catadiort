//! texture.rs: the texture table behind rtMakeTexture and rtTexture (HW2
//! handout: shaders and textures). Given complete in both kits; rt.rs's two
//! calls hand it the file and the lookup, and the environment map
//! (Workflow, step 13) reads it through lookup_lat_long. The twin of the
//! C++ kit's texture.cpp. An 8-bit image is kept as read, 4 bytes per
//! texel, and decoded to linear at lookup through a 256-entry table (sRGB
//! or plain), so Sponza's textures cost 200 MB rather than 800; PFM and HDR
//! are kept as linear floats. Rows run top first. Handles index one table,
//! no slot limit.
//!
//! Textures are loaded on the main thread while a scene runs, and looked up
//! by every worker while one renders. Each worker keeps its own snapshot of
//! the table, refreshed only when a load or a clear has changed it, so a
//! lookup never waits on a lock that other threads are taking too.

#![allow(dead_code)] // size and texel are HW4's (the environment as a light)

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use ultraviolet::Vec4;

use super::image::{read_image, srgb_to_linear};

struct Texture {
    w: i32,
    h: i32,
    rgba_f: Vec<f32>, /* float images: 4 floats per texel, top row first */
    rgba8: Vec<u8>,   /* 8-bit images: 4 bytes per texel */
    decode: &'static [f32; 256], /* 8-bit images: the color channels' byte-to-linear table */
}

/// The two byte-to-linear tables: sRGB decoded, and plain (alpha, masks,
/// images loaded with srgb = 0). Built once.
struct DecodeTables {
    srgb: [f32; 256],
    plain: [f32; 256],
}

fn tables() -> &'static DecodeTables {
    static T: OnceLock<DecodeTables> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = DecodeTables {
            srgb: [0.0; 256],
            plain: [0.0; 256],
        };
        for i in 0..256 {
            t.plain[i] = i as f32 / 255.0;
            t.srgb[i] = srgb_to_linear(t.plain[i]);
        }
        t
    })
}

type Table = Arc<Vec<Arc<Texture>>>;

/// The table, and a count of its changes, which tells a worker's snapshot
/// that it is out of date.
static TABLE: Mutex<Option<Table>> = Mutex::new(None);
static CHANGES: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static SNAPSHOT: RefCell<(u64, Table)> = RefCell::new((u64::MAX, Arc::new(Vec::new())));
}

/// Run f on the texture of a handle (None for an invalid one), through this
/// thread's snapshot of the table.
fn with_texture<R>(handle: i32, f: impl FnOnce(Option<&Texture>) -> R) -> R {
    SNAPSHOT.with(|cell| {
        let mut snap = cell.borrow_mut();
        let changes = CHANGES.load(Ordering::Acquire);
        if snap.0 != changes {
            let table = TABLE.lock().expect("textures").clone().unwrap_or_default();
            *snap = (changes, table);
        }
        let tex = if handle >= 0 {
            snap.1.get(handle as usize).map(|t| &**t)
        } else {
            None
        };
        f(tex.filter(|t| t.w > 0))
    })
}

/// PFM and HDR through the given read_image (image.rs), kept as four floats
/// per texel; a grey image repeats its channel.
fn load_float(path: &str) -> Option<Texture> {
    let img = read_image(path, false)?;
    let n = img.channels as usize;
    let mut rgba_f = vec![0.0f32; img.width as usize * img.height as usize * 4];
    for (i, o) in rgba_f.chunks_exact_mut(4).enumerate() {
        let p = &img.texels[i * n..i * n + n];
        if n <= 2 {
            o[0] = p[0];
            o[1] = p[0];
            o[2] = p[0];
            o[3] = if n == 2 { p[1] } else { 1.0 };
        } else {
            o[0] = p[0];
            o[1] = p[1];
            o[2] = p[2];
            o[3] = if n == 4 { p[3] } else { 1.0 };
        }
    }
    Some(Texture {
        w: img.width,
        h: img.height,
        rgba_f,
        rgba8: Vec::new(),
        decode: &tables().plain,
    })
}

/// An 8-bit image through the image crate: grey, grey + alpha, RGB or RGBA,
/// stored as 4 bytes per texel. The color channels are decoded at lookup,
/// from sRGB when asked; the alpha channel is coverage and never is.
fn load_8bit(path: &str, srgb: bool) -> Result<Texture, String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let (w, h) = (img.width() as i32, img.height() as i32);
    let rgba8 = match img.color().channel_count() {
        1 | 2 => {
            let la = img.into_luma_alpha8();
            la.pixels().flat_map(|p| [p[0], p[0], p[0], p[1]]).collect()
        }
        _ => img.into_rgba8().into_raw(),
    };
    let decode = if srgb {
        &tables().srgb
    } else {
        &tables().plain
    };
    Ok(Texture {
        w,
        h,
        rgba_f: Vec::new(),
        rgba8,
        decode,
    })
}

/// Lowercase extension of a file name, without the dot ("" if none).
fn extension(name: &str) -> String {
    match name.rfind('.') {
        Some(dot) => name[dot + 1..].to_ascii_lowercase(),
        None => String::new(),
    }
}

/// Load an image and return its handle (>= 0), or -1 with a message on
/// stderr. Tries the path as given, then under textures/. 8-bit formats
/// (PNG, JPEG, BMP, TGA) are decoded to linear when srgb is set, the alpha
/// channel never; PFM and Radiance HDR are linear floats already.
pub fn load(file: &str, srgb: bool) -> i32 {
    let mut path = file.to_string();
    if !std::path::Path::new(&path).is_file() {
        path = format!("assets/{}", file);
    }
    if !std::path::Path::new(&path).is_file() {
        path = format!("assets/textures/{}", file);
    }
    if !std::path::Path::new(&path).is_file() {
        eprintln!(
            "rtMakeTexture: cannot find {} (as given or under assets/textures/; run gauchoRT from the kit root)",
            file
        );
        return -1;
    }
    let ext = extension(&path);
    let tex = if ext == "pfm" || ext == "hdr" {
        load_float(&path) /* read_image says why when it fails */
    } else {
        load_8bit(&path, srgb)
            .map_err(|why| eprintln!("rtMakeTexture: cannot read {} ({})", path, why))
            .ok()
    };
    let Some(tex) = tex else { return -1 };
    let mut table = TABLE.lock().expect("textures");
    let list = Arc::make_mut(table.get_or_insert_with(Default::default));
    list.push(Arc::new(tex));
    let handle = list.len() as i32 - 1;
    CHANGES.fetch_add(1, Ordering::Release);
    handle
}

/// i wrapped into [0, n) for any sign.
fn wrap(i: i32, n: i32) -> i32 {
    i.rem_euclid(n)
}

/// The bilinear lookup behind both public ones; wrap_rows says whether t
/// repeats like s or clamps to the first and last rows.
#[inline]
fn sample(handle: i32, s: f32, t: f32, wrap_rows: bool) -> Vec4 {
    with_texture(handle, |tex| {
        let Some(tex) = tex else {
            return [0.0, 0.0, 0.0, 1.0];
        };
        /* Texel centers sit at (i + 0.5) / w, so the sample point in texel
         * units is s w - 0.5; blend the four texels around it, in linear
         * values (an 8-bit texel is decoded first, so the blend is linear). */
        let x = s * tex.w as f32 - 0.5;
        let y = t * tex.h as f32 - 0.5;
        let (fx0, fy0) = (x.floor(), y.floor());
        let (ax, ay) = (x - fx0, y - fy0);
        let x0 = wrap(fx0 as i32, tex.w);
        let x1 = wrap(fx0 as i32 + 1, tex.w);
        let (y0, y1) = if wrap_rows {
            (wrap(fy0 as i32, tex.h), wrap(fy0 as i32 + 1, tex.h))
        } else {
            (
                (fy0 as i32).clamp(0, tex.h - 1),
                (fy0 as i32 + 1).clamp(0, tex.h - 1),
            )
        };
        let at = |x: i32, y: i32| (y as usize * tex.w as usize + x as usize) * 4;
        let (i00, i10, i01, i11) = (at(x0, y0), at(x1, y0), at(x0, y1), at(x1, y1));
        let (w00, w10, w01, w11) = (
            (1.0 - ax) * (1.0 - ay),
            ax * (1.0 - ay),
            (1.0 - ax) * ay,
            ax * ay,
        );
        let mut rgba = [0.0f32; 4];
        if tex.rgba8.is_empty() {
            let p = &tex.rgba_f;
            for c in 0..4 {
                rgba[c] = p[i00 + c] * w00 + p[i10 + c] * w10 + p[i01 + c] * w01 + p[i11 + c] * w11;
            }
        } else {
            let (p, d, plain) = (&tex.rgba8, tex.decode, &tables().plain);
            for c in 0..3 {
                rgba[c] = d[p[i00 + c] as usize] * w00
                    + d[p[i10 + c] as usize] * w10
                    + d[p[i01 + c] as usize] * w01
                    + d[p[i11 + c] as usize] * w11;
            }
            rgba[3] = plain[p[i00 + 3] as usize] * w00
                + plain[p[i10 + 3] as usize] * w10
                + plain[p[i01 + 3] as usize] * w01
                + plain[p[i11 + 3] as usize] * w11;
        }
        rgba
    })
    .into()
}

/// Bilinear lookup with repeat wrap; s runs left to right, t top to
/// bottom. Alpha is 1 when the image has none. An invalid handle reads
/// opaque black.
pub fn lookup(handle: i32, s: f32, t: f32) -> Vec4 {
    sample(handle, s, t, true)
}

/// The same lookup for a latitude-longitude map (an environment): s wraps
/// around, t clamps at the top and bottom rows, which are the poles.
pub fn lookup_lat_long(handle: i32, s: f32, t: f32) -> Vec4 {
    sample(handle, s, t, false)
}

/// A texture's size in texels, None for an invalid handle; and one texel's
/// linear RGBA, column x of row y (top row first), for building the
/// environment's sampling distribution (HW4).
pub fn size(handle: i32) -> Option<(i32, i32)> {
    with_texture(handle, |tex| tex.map(|t| (t.w, t.h)))
}

pub fn texel(handle: i32, x: i32, y: i32) -> [f32; 4] {
    with_texture(handle, |tex| {
        let Some(tex) = tex else {
            return [0.0, 0.0, 0.0, 1.0];
        };
        let i = (y as usize * tex.w as usize + x as usize) * 4;
        if tex.rgba8.is_empty() {
            [
                tex.rgba_f[i],
                tex.rgba_f[i + 1],
                tex.rgba_f[i + 2],
                tex.rgba_f[i + 3],
            ]
        } else {
            let d = tex.decode;
            [
                d[tex.rgba8[i] as usize],
                d[tex.rgba8[i + 1] as usize],
                d[tex.rgba8[i + 2] as usize],
                tables().plain[tex.rgba8[i + 3] as usize],
            ]
        }
    })
}

/// Free every texture (rtBegin, rtEnd).
pub fn clear() {
    *TABLE.lock().expect("textures") = None;
    CHANGES.fetch_add(1, Ordering::Release);
}
