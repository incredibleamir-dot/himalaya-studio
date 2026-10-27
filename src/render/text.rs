//! Text, seal and font handling.
//!
//! The renderer looks for a Devanagari-capable face in the system font
//! directories (Nirmala UI or Mangal on Windows, Noto Serif Devanagari on
//! Linux, Devanagari Sangam MN on macOS). HarfRust performs OpenType shaping,
//! so matras are reordered, conjuncts are formed and the i-matra sits on the
//! correct side of its consonant; `ab_glyph` rasterises the shaped outlines.
//! A last-resort Latin face keeps the interface legible when no Devanagari
//! font is installed.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use ab_glyph::{Font, PxScale, ScaleFont};
use unicode_segmentation::UnicodeSegmentation;

use crate::color::Color;
use crate::render::raster::Bitmap;
use crate::shapes::{Seal, Text};

/// Font bytes plus the face index inside a TrueType collection.
struct FontResource {
    bytes: Arc<Vec<u8>>,
    index: u32,
}

/// A glyph placed by the shaper, in font units.
#[derive(Clone, Copy)]
struct PlacedGlyph {
    id: u16,
    x_offset: f32,
    y_offset: f32,
    x_advance: f32,
}

fn font_cache() -> &'static Mutex<HashMap<String, Option<Arc<FontResource>>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<Arc<FontResource>>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// System font directories searched for inscription and seal faces.
pub fn font_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if let Some(windir) = std::env::var_os("WINDIR") {
        dirs.push(std::path::PathBuf::from(windir).join("Fonts"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(
            std::path::PathBuf::from(local)
                .join("Microsoft")
                .join("Windows")
                .join("Fonts"),
        );
    }
    for dir in [
        "/System/Library/Fonts",
        "/System/Library/Fonts/Supplemental",
        "/Library/Fonts",
        "/usr/share/fonts",
        "/usr/share/fonts/opentype/noto",
        "/usr/share/fonts/truetype/noto",
        "/usr/share/fonts/truetype/dejavu",
        "/usr/share/fonts/TTF",
        "/usr/share/fonts/OTF",
        "/usr/local/share/fonts",
    ] {
        dirs.push(std::path::PathBuf::from(dir));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = std::path::PathBuf::from(home);
        dirs.push(home.join("Library").join("Fonts"));
        dirs.push(home.join(".local").join("share").join("fonts"));
        dirs.push(home.join(".fonts"));
    }
    if let Some(data) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(std::path::PathBuf::from(data).join("fonts"));
    }
    dirs
}

fn candidate_names(family: &str) -> &'static [&'static str] {
    if family.to_lowercase().contains("verdana") {
        &[
            "verdana.ttf",
            "Verdana.ttf",
            "arial.ttf",
            "Arial.ttf",
            "segoeui.ttf",
            "Helvetica.ttc",
            "DejaVuSans.ttf",
        ]
    } else {
        &[
            "Nirmala.ttc",
            "Nirmala.ttf",
            "NirmalaB.ttc",
            "NirmalaB.ttf",
            "NotoSerifDevanagari-Regular.ttf",
            "NotoSansDevanagari-Regular.ttf",
            "Mangal.ttf",
            "mangal.ttf",
            "KohinoorDevanagari-Regular.otf",
            "DevanagariSangamMN.ttc",
            "arialuni.ttf",
            "Arial Unicode.ttf",
            "DejaVuSans.ttf",
        ]
    }
}

fn load_font_file(path: &std::path::Path) -> Option<FontResource> {
    let bytes = std::fs::read(path).ok()?;
    // A collection's first face is the regular UI face for every candidate
    // here. Validate the bytes with both engines before caching them.
    ab_glyph::FontRef::try_from_slice_and_index(&bytes, 0).ok()?;
    harfrust::FontRef::from_index(&bytes, 0).ok()?;
    Some(FontResource {
        bytes: Arc::new(bytes),
        index: 0,
    })
}

fn load_font(family: &str) -> Option<Arc<FontResource>> {
    let key = if family.to_lowercase().contains("verdana") {
        "verdana".to_string()
    } else {
        "serif".to_string()
    };
    let mut cache = font_cache().lock().ok()?;
    if let Some(entry) = cache.get(&key) {
        return entry.clone();
    }
    let dirs = font_dirs();
    let mut found: Option<Arc<FontResource>> = None;
    'search: for name in candidate_names(family) {
        for dir in &dirs {
            if let Some(font) = load_font_file(&dir.join(name)) {
                found = Some(Arc::new(font));
                break 'search;
            }
        }
    }
    cache.insert(key, found.clone());
    found
}

/// Pixels per font unit for the rasteriser's em-height scale.
fn px_per_unit(font: &ab_glyph::FontRef<'_>, size: f32) -> f64 {
    let height = font.ascent_unscaled() - font.descent_unscaled();
    if height.abs() < f32::EPSILON {
        size as f64
    } else {
        (size as f64 / height as f64).abs()
    }
}

/// Shape a run with HarfRust. Positions and advances stay in font units.
fn shape(font: &FontResource, text: &str) -> Vec<PlacedGlyph> {
    let Some(face) = harfrust::FontRef::from_index(&font.bytes, font.index).ok() else {
        return fallback_layout(font, text);
    };
    let data = harfrust::ShaperData::new(&face);
    let shaper = data.shaper(&face).build();
    let mut buffer = harfrust::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    let output = shaper.shape(buffer, harfrust::ShapeOptions::new());

    let mut glyphs = Vec::with_capacity(output.len());
    for (info, pos) in output.glyph_infos().iter().zip(output.glyph_positions()) {
        let Ok(id) = u16::try_from(info.glyph_id) else {
            continue;
        };
        glyphs.push(PlacedGlyph {
            id,
            x_offset: pos.x_offset as f32,
            y_offset: pos.y_offset as f32,
            x_advance: pos.x_advance as f32,
        });
    }

    if glyphs.is_empty() && !text.is_empty() {
        fallback_layout(font, text)
    } else {
        glyphs
    }
}

/// Character-by-character layout used only if shaping cannot run.
fn fallback_layout(font: &FontResource, text: &str) -> Vec<PlacedGlyph> {
    let Ok(face) = ab_glyph::FontRef::try_from_slice_and_index(&font.bytes, font.index) else {
        return Vec::new();
    };
    text.chars()
        .map(|ch| {
            let id = face.glyph_id(ch);
            PlacedGlyph {
                id: id.0,
                x_offset: 0.0,
                y_offset: 0.0,
                x_advance: face.h_advance_unscaled(id),
            }
        })
        .collect()
}

fn run_width(glyphs: &[PlacedGlyph], unit: f64) -> f32 {
    glyphs.iter().map(|glyph| glyph.x_advance).sum::<f32>() * unit as f32
}

const SEAL_TEXT_FACTOR: f64 = 0.46;
const SEAL_PADDING_FACTOR: f64 = 0.18;
const SEAL_ADVANCE_FACTOR: f64 = 0.62;

/// A seal inscription measured for a single row.
pub struct SealLayout {
    /// Width of the seal rectangle in scene units. A square seal grows to the
    /// right when the inscription needs more room.
    pub width: f64,
    /// Font size in the same units as the seal height.
    pub font_size: f64,
    /// Width of the shaped inscription in scene units, excluding padding.
    pub text_width: f64,
}

/// Measure a seal inscription for a rectangle `height` units tall.
///
/// The width comes from the same shaped glyph run that the rasteriser draws,
/// so wide clusters such as `कां` keep their padding instead of touching the
/// seal edge. The inscription is never wrapped: the rectangle grows to fit up
/// to ten grapheme clusters on one line.
pub fn seal_layout(text: &str, height: f64) -> SealLayout {
    let text = text.trim();
    let font_size = (height * SEAL_TEXT_FACTOR).max(1.0);
    if text.is_empty() {
        return SealLayout {
            width: height,
            font_size,
            text_width: 0.0,
        };
    }

    let text_width = measure_line(text) * font_size;
    let width = (text_width + height * SEAL_PADDING_FACTOR).max(height);
    SealLayout {
        width,
        font_size,
        text_width,
    }
}

/// Width of a shaped line in font-size units.
fn measure_line(text: &str) -> f64 {
    let Some(font) = load_font("serif") else {
        return fallback_width(text);
    };
    let Ok(face) = ab_glyph::FontRef::try_from_slice_and_index(&font.bytes, font.index) else {
        return fallback_width(text);
    };
    let height = face.ascent_unscaled() - face.descent_unscaled();
    if height.abs() < f32::EPSILON {
        return fallback_width(text);
    }
    let glyphs = shape(&font, text);
    let units: f32 = glyphs.iter().map(|glyph| glyph.x_advance).sum();
    (units as f64 / height as f64).abs().max(0.01)
}

fn fallback_width(text: &str) -> f64 {
    (text.graphemes(true).count() as f64 * SEAL_ADVANCE_FACTOR).max(0.01)
}

/// Width, ascent and descent (both positive magnitudes) at a given pixel size.
pub fn text_metrics(family: &str, text: &str, size: f32) -> Option<(f32, f32, f32)> {
    let font = load_font(family)?;
    let face = ab_glyph::FontRef::try_from_slice_and_index(&font.bytes, font.index).ok()?;
    let scaled = face.as_scaled(PxScale::from(size.max(1.0)));
    let unit = px_per_unit(&face, size.max(1.0));
    let glyphs = shape(&font, text);
    Some((
        run_width(&glyphs, unit),
        scaled.ascent(),
        scaled.descent().abs(),
    ))
}

pub fn font_family_label(family: &str) -> &'static str {
    if family.to_lowercase().contains("verdana") {
        "Verdana"
    } else {
        "Devanagari"
    }
}

fn draw_glyphs_into_bitmap(
    bitmap: &mut Bitmap,
    font: &FontResource,
    glyphs: &[PlacedGlyph],
    left: f64,
    baseline: f64,
    size: f32,
    color: Color,
) {
    let Ok(face) = ab_glyph::FontRef::try_from_slice_and_index(&font.bytes, font.index) else {
        return;
    };
    let unit = px_per_unit(&face, size);
    let mut pen = left;
    for placed in glyphs {
        let glyph = ab_glyph::GlyphId(placed.id).with_scale_and_position(
            PxScale::from(size),
            ab_glyph::point(
                (pen + placed.x_offset as f64 * unit) as f32,
                (baseline - placed.y_offset as f64 * unit) as f32,
            ),
        );
        if let Some(outlined) = face.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, coverage| {
                let x = bounds.min.x.round() as i32 + gx as i32;
                let y = bounds.min.y.round() as i32 + gy as i32;
                if coverage > 0.0 {
                    bitmap.blend_pixel(x, y, color, coverage);
                }
            });
        }
        pen += placed.x_advance as f64 * unit;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_glyphs_into_layer(
    layer: &mut [f32],
    width: usize,
    height: usize,
    font: &FontResource,
    glyphs: &[PlacedGlyph],
    left: f64,
    baseline: f64,
    size: f32,
) {
    let Ok(face) = ab_glyph::FontRef::try_from_slice_and_index(&font.bytes, font.index) else {
        return;
    };
    let unit = px_per_unit(&face, size);
    let mut pen = left;
    for placed in glyphs {
        let glyph = ab_glyph::GlyphId(placed.id).with_scale_and_position(
            PxScale::from(size),
            ab_glyph::point(
                (pen + placed.x_offset as f64 * unit) as f32,
                (baseline - placed.y_offset as f64 * unit) as f32,
            ),
        );
        if let Some(outlined) = face.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, coverage| {
                let x = bounds.min.x.round() as i32 + gx as i32;
                let y = bounds.min.y.round() as i32 + gy as i32;
                if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                    return;
                }
                let idx = y as usize * width + x as usize;
                layer[idx] = (layer[idx] + coverage).min(1.0);
            });
        }
        pen += placed.x_advance as f64 * unit;
    }
}

fn rotated_composite(
    bitmap: &mut Bitmap,
    layer: &[f32],
    lw: usize,
    lh: usize,
    cx: f64,
    cy: f64,
    angle_deg: f64,
    color: Color,
) {
    let angle = angle_deg.to_radians();
    let (sin, cos) = angle.sin_cos();
    let half_w = lw as f64 / 2.0;
    let half_h = lh as f64 / 2.0;
    let radius = (half_w * half_w + half_h * half_h).sqrt() + 1.0;
    let x0 = (cx - radius).floor() as i32;
    let x1 = (cx + radius).ceil() as i32;
    let y0 = (cy - radius).floor() as i32;
    let y1 = (cy + radius).ceil() as i32;
    for py in y0..=y1 {
        for px in x0..=x1 {
            let dx = px as f64 + 0.5 - cx;
            let dy = py as f64 + 0.5 - cy;
            let lx = cos * dx + sin * dy + half_w;
            let ly = -sin * dx + cos * dy + half_h;
            if lx < 0.0 || ly < 0.0 || lx >= lw as f64 || ly >= lh as f64 {
                continue;
            }
            // Bilinear sample of the coverage layer.
            let sx = lx - 0.5;
            let sy = ly - 0.5;
            let sx0 = sx.floor();
            let sy0 = sy.floor();
            let fx = sx - sx0;
            let fy = sy - sy0;
            let sample = |ix: i32, iy: i32| -> f64 {
                if ix < 0 || iy < 0 || ix >= lw as i32 || iy >= lh as i32 {
                    0.0
                } else {
                    layer[iy as usize * lw + ix as usize] as f64
                }
            };
            let ix = sx0 as i32;
            let iy = sy0 as i32;
            let c00 = sample(ix, iy);
            let c10 = sample(ix + 1, iy);
            let c01 = sample(ix, iy + 1);
            let c11 = sample(ix + 1, iy + 1);
            let coverage =
                (c00 * (1.0 - fx) + c10 * fx) * (1.0 - fy) + (c01 * (1.0 - fx) + c11 * fx) * fy;
            if coverage > 0.001 {
                bitmap.blend_pixel(px, py, color, coverage as f32);
            }
        }
    }
}

pub fn draw_text(bitmap: &mut Bitmap, shape: &Text, xmin: f64, draw_scale: f64) {
    let Some(font) = load_font(&shape.family) else {
        return;
    };
    let Ok(face) = ab_glyph::FontRef::try_from_slice_and_index(&font.bytes, font.index) else {
        return;
    };
    let size = ((shape.size * draw_scale).max(1.0)) as f32;
    let scaled = face.as_scaled(PxScale::from(size));
    let ascent = scaled.ascent();
    let descent = scaled.descent();
    let unit = px_per_unit(&face, size);
    let glyphs = crate::render::text::shape(&font, &shape.text);
    let width = run_width(&glyphs, unit);
    let line_h = ascent - descent;
    let x = (shape.x - xmin) * draw_scale;
    let y = shape.y * draw_scale;

    if shape.angle.abs() < 0.001 {
        let left = match shape.anchor.as_str() {
            "start" => x,
            "end" => x - width as f64,
            _ => x - width as f64 / 2.0,
        };
        let baseline = y + (ascent + descent) as f64 / 2.0;
        draw_glyphs_into_bitmap(bitmap, &font, &glyphs, left, baseline, size, shape.color);
        return;
    }

    let pad = 2.0f64;
    let lw = (width as f64 + pad * 2.0).ceil().max(1.0) as usize;
    let lh = (line_h as f64 + pad * 2.0).ceil().max(1.0) as usize;
    let mut layer = vec![0.0f32; lw * lh];
    draw_glyphs_into_layer(
        &mut layer,
        lw,
        lh,
        &font,
        &glyphs,
        pad,
        pad + ascent as f64,
        size,
    );
    rotated_composite(bitmap, &layer, lw, lh, x, y, shape.angle, shape.color);
}

pub fn draw_seal(bitmap: &mut Bitmap, seal: &Seal, xmin: f64, draw_scale: f64) {
    let s = seal.size * draw_scale;
    let x = (seal.x - xmin) * draw_scale;
    let y = seal.y * draw_scale;
    let layout = seal_layout(&seal.text, seal.size);
    let w = layout.width * draw_scale;
    bitmap.fill_rounded_rect(x, y, w, s, s * 0.12, seal.color);

    if layout.text_width <= 0.0 {
        return;
    }
    let Some(font) = load_font("serif") else {
        return;
    };
    let Ok(face) = ab_glyph::FontRef::try_from_slice_and_index(&font.bytes, font.index) else {
        return;
    };
    let size = ((layout.font_size * draw_scale).max(1.0)) as f32;
    let scaled = face.as_scaled(PxScale::from(size));
    let ascent = scaled.ascent();
    let descent = scaled.descent();
    let unit = px_per_unit(&face, size);
    let glyphs = shape(&font, seal.text.trim());
    let text_width = run_width(&glyphs, unit);
    let left = x + (w - text_width as f64) / 2.0;
    let baseline = y + s / 2.0 + s * 0.03 + (ascent + descent) as f64 / 2.0;
    draw_glyphs_into_bitmap(
        bitmap,
        &font,
        &glyphs,
        left,
        baseline,
        size,
        seal.text_color,
    );
}
