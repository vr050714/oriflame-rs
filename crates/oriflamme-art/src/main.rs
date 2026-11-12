//! Rasterises the art sources to the files the game uses:
//!
//! - every `art/cards/*.svg` and `art/ui/*.svg` to a PNG of the same name
//!   in `assets/cards` and `assets/ui`;
//! - `art/icon.svg` to `assets/icon.png` (window icon) and `assets/icon.ico`
//!   (executable icon).
//!
//! Run from anywhere in the workspace: `cargo run -p oriflamme-art`.
//! The output files are committed, so the game builds without this step.

use resvg::{tiny_skia, usvg};
use std::{error::Error, fs, path::Path};

/// Edge length in pixels of the window icon.
const WINDOW_ICON_SIZE: u32 = 64;
/// Edge lengths in pixels of the images inside the `.ico` file.
const ICO_SIZES: [u32; 4] = [16, 32, 48, 256];

fn main() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let art = root.join("art");
    let assets = root.join("assets");

    let mut count: u32 = 0;
    for folder in ["cards", "ui"] {
        let target = assets.join(folder);
        fs::create_dir_all(&target)?;
        for entry in fs::read_dir(art.join(folder))? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("svg") {
                continue;
            }
            let Some(stem) = path.file_stem() else {
                continue;
            };
            let tree = load(&path)?;
            let out = target.join(stem).with_extension("png");
            fs::write(&out, render(&tree, None)?)?;
            println!("{}", out.display());
            count = count.saturating_add(1);
        }
    }
    println!("{count} images written");

    let icon = load(&art.join("icon.svg"))?;
    fs::write(
        assets.join("icon.png"),
        render(&icon, Some(WINDOW_ICON_SIZE))?,
    )?;
    let images = ICO_SIZES
        .iter()
        .map(|&size| Ok((size, render(&icon, Some(size))?)))
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    fs::write(assets.join("icon.ico"), ico(&images)?)?;
    println!("icon.png and icon.ico written");
    Ok(())
}

fn load(path: &Path) -> Result<usvg::Tree, Box<dyn Error>> {
    Ok(usvg::Tree::from_data(
        &fs::read(path)?,
        &usvg::Options::default(),
    )?)
}

/// Renders an SVG to PNG bytes. With `width`, the image is scaled to that
/// width in pixels; otherwise one SVG unit is one pixel.
fn render(tree: &usvg::Tree, width: Option<u32>) -> Result<Vec<u8>, Box<dyn Error>> {
    let natural = tree.size().to_int_size();
    let size = match width {
        Some(width) => natural
            .scale_to_width(width)
            .ok_or("image size out of range")?,
        None => natural,
    };
    #[allow(clippy::cast_precision_loss, reason = "image sizes are far below 2^24")]
    let scale = size.width() as f32 / tree.size().width();
    let mut pixmap =
        tiny_skia::Pixmap::new(size.width(), size.height()).ok_or("image has no area")?;
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap.encode_png()?)
}

/// Builds a Windows `.ico` file from square PNG images given as
/// `(edge length, PNG bytes)`.
fn ico(images: &[(u32, Vec<u8>)]) -> Result<Vec<u8>, Box<dyn Error>> {
    const HEADER_LEN: usize = 6;
    const ENTRY_LEN: usize = 16;

    let count = u16::try_from(images.len())?;
    let mut out = Vec::new();
    out.extend_from_slice(&0_u16.to_le_bytes()); // reserved
    out.extend_from_slice(&1_u16.to_le_bytes()); // type: icon
    out.extend_from_slice(&count.to_le_bytes());

    let mut offset = images
        .len()
        .checked_mul(ENTRY_LEN)
        .and_then(|n| n.checked_add(HEADER_LEN))
        .ok_or("too many images")?;
    for (size, png) in images {
        // A size byte of 0 means 256 pixels.
        let edge = u8::try_from(*size).unwrap_or(0);
        out.extend_from_slice(&[edge, edge, 0, 0]); // width, height, palette, reserved
        out.extend_from_slice(&1_u16.to_le_bytes()); // colour planes
        out.extend_from_slice(&32_u16.to_le_bytes()); // bits per pixel
        out.extend_from_slice(&u32::try_from(png.len())?.to_le_bytes());
        out.extend_from_slice(&u32::try_from(offset)?.to_le_bytes());
        offset = offset.checked_add(png.len()).ok_or("icon file too large")?;
    }
    for (_, png) in images {
        out.extend_from_slice(png);
    }
    Ok(out)
}
