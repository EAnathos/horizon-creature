//! A published module's icon (its `module.toml`'s `[registry] icon`): a square PNG of `SIDE` pixels, for its registry
//! page (`registry/icons/<id>.png`, 64 to 1024 pixels and at most 256 KiB) and its package (`icon.png`, at most 512
//! pixels and 256 KiB: the game's Mods screen).
//!
//! Made from one of the module's models, its picture in `docs/models/` (`cargo creature preview`), cropped to the model
//! and centred; or from a PNG by its path from the repository's root (an item's 16-pixel texture, scaled up in steps).
use std::path::Path;

/// The icon's side, in pixels.
pub const SIDE: usize = 256;
/// The empty margin around the model, as a share of the side.
const MARGIN: f64 = 0.06;
/// The biggest file the registry and the game take.
pub const MAX_BYTES: u64 = 256 * 1024;

/// An RGBA image.
struct Image {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

/// Writes the icon `source` names for module `id` to `out`.
pub fn make(id: &str, source: &str, out: &Path) -> crate::Result {
    let root = crate::root();
    let path = if source.ends_with(".png") {
        root.join(source)
    } else {
        root.join(format!("docs/models/{id}-{source}-front.png"))
    };
    let image = read(&path)?;
    let icon = square(&crop(&image), source.ends_with(".png") && image.width <= 64);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    write(out, &icon)?;
    let size = std::fs::metadata(out).map_err(|e| e.to_string())?.len();
    if size > MAX_BYTES {
        return Err(format!("{}: {size} bytes, over 256 KiB", out.display()));
    }
    Ok(())
}

/// The smallest part of `image` holding every pixel that isn't transparent.
fn crop(image: &Image) -> Image {
    let opaque = |x: usize, y: usize| image.rgba[(y * image.width + x) * 4 + 3] > 0;
    let (mut x0, mut y0, mut x1, mut y1) = (image.width, image.height, 0, 0);
    for y in 0..image.height {
        for x in 0..image.width {
            if opaque(x, y) {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
            }
        }
    }
    if x0 >= x1 {
        return Image {
            width: image.width,
            height: image.height,
            rgba: image.rgba.clone(),
        };
    }
    let (width, height) = (x1 - x0, y1 - y0);
    let mut rgba = Vec::with_capacity(width * height * 4);
    for y in y0..y1 {
        let row = (y * image.width + x0) * 4;
        rgba.extend_from_slice(&image.rgba[row..row + width * 4]);
    }
    Image { width, height, rgba }
}

/// `image` scaled to fit `SIDE` less the margins and centred on a transparent square: by nearest pixel for a small
/// pixel-art texture (`pixel_art`, its pixels kept square), else by averaging.
fn square(image: &Image, pixel_art: bool) -> Image {
    let room = SIDE as f64 * (1.0 - 2.0 * MARGIN);
    let scale = if pixel_art {
        (room / image.width.max(image.height) as f64).floor().max(1.0)
    } else {
        room / image.width.max(image.height) as f64
    };
    let (w, h) = (
        ((image.width as f64 * scale).round() as usize).clamp(1, SIDE),
        ((image.height as f64 * scale).round() as usize).clamp(1, SIDE),
    );
    let (ox, oy) = ((SIDE - w) / 2, (SIDE - h) / 2);
    let mut rgba = vec![0u8; SIDE * SIDE * 4];
    for y in 0..h {
        for x in 0..w {
            let px = if pixel_art {
                let (sx, sy) = ((x as f64 / scale) as usize, (y as f64 / scale) as usize);
                let i = (sy.min(image.height - 1) * image.width + sx.min(image.width - 1)) * 4;
                [image.rgba[i], image.rgba[i + 1], image.rgba[i + 2], image.rgba[i + 3]]
            } else {
                average(image, x as f64 / scale, y as f64 / scale, 1.0 / scale)
            };
            let o = ((oy + y) * SIDE + ox + x) * 4;
            rgba[o..o + 4].copy_from_slice(&px);
        }
    }
    Image {
        width: SIDE,
        height: SIDE,
        rgba,
    }
}

/// The average of `image`'s pixels in the square of side `size` at `(x, y)`, colours weighted by their opacity.
fn average(image: &Image, x: f64, y: f64, size: f64) -> [u8; 4] {
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = (
        ((x + size).ceil() as usize).clamp(x0 + 1, image.width),
        ((y + size).ceil() as usize).clamp(y0 + 1, image.height),
    );
    let mut sum = [0.0f64; 4];
    let mut count = 0.0;
    for sy in y0.min(image.height - 1)..y1 {
        for sx in x0.min(image.width - 1)..x1 {
            let i = (sy * image.width + sx) * 4;
            let a = f64::from(image.rgba[i + 3]);
            for (k, total) in sum.iter_mut().take(3).enumerate() {
                *total += f64::from(image.rgba[i + k]) * a;
            }
            sum[3] += a;
            count += 1.0;
        }
    }
    if sum[3] == 0.0 {
        return [0; 4];
    }
    let colour = |k: usize| (sum[k] / sum[3]).round() as u8;
    [colour(0), colour(1), colour(2), (sum[3] / count).round() as u8]
}

fn read(path: &Path) -> crate::Result<Image> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().map_err(|e| format!("{}: {e}", path.display()))?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("image too big")?];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let (width, height) = (info.width as usize, info.height as usize);
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf[..width * height * 4].to_vec(),
        png::ColorType::GrayscaleAlpha => buf[..width * height * 2]
            .chunks(2)
            .flat_map(|g| [g[0], g[0], g[0], g[1]])
            .collect(),
        other => return Err(format!("{}: colours {other:?}", path.display())),
    };
    Ok(Image { width, height, rgba })
}

fn write(path: &Path, image: &Image) -> crate::Result {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), image.width as u32, image.height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| format!("{}: {e}", path.display()))?;
    writer
        .write_image_data(&image.rgba)
        .map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: usize, height: usize, opaque: &[(usize, usize)]) -> Image {
        let mut rgba = vec![0; width * height * 4];
        for &(x, y) in opaque {
            rgba[(y * width + x) * 4..][..4].copy_from_slice(&[200, 100, 50, 255]);
        }
        Image { width, height, rgba }
    }

    #[test]
    fn the_icon_is_the_model_cropped_and_centred_on_a_square() {
        let cropped = crop(&image(10, 8, &[(2, 3), (5, 6)]));
        assert_eq!((cropped.width, cropped.height), (4, 4));
        let icon = square(&cropped, true);
        assert_eq!((icon.width, icon.height), (SIDE, SIDE));
        // The middle is the model, the corner the transparent margin.
        assert_eq!(
            icon.rgba[((SIDE / 2) * SIDE + SIDE / 2) * 4 + 3],
            0,
            "between the two pixels"
        );
        assert_eq!(icon.rgba[3], 0);
        let opaque = icon.rgba.chunks(4).filter(|p| p[3] > 0).count();
        assert!(opaque > 0);
    }
}
