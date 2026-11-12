//! Title-bar icon for native builds. The web build uses the page's favicon.

use bevy::{
    asset::RenderAssetUsages,
    ecs::system::NonSendMarker,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::*,
    window::PrimaryWindow,
    winit::WINIT_WINDOWS,
};
use winit::window::Icon;

/// Sets the icon in the title bar. Runs until the window exists, then once.
/// The executable's own icon (Explorer, taskbar) is embedded by `build.rs`.
pub fn set_window_icon(
    mut done: Local<bool>,
    windows: Query<Entity, With<PrimaryWindow>>,
    // Window calls must be made on the main thread.
    _main_thread: NonSendMarker,
) {
    if *done {
        return;
    }
    let Ok(entity) = windows.single() else {
        return;
    };
    WINIT_WINDOWS.with_borrow(|winit_windows| {
        let Some(window) = winit_windows.get_window(entity) else {
            return;
        };
        *done = true;
        match decode_icon() {
            Ok(icon) => window.set_window_icon(Some(icon)),
            Err(error) => error!("cannot set the window icon: {error}"),
        }
    });
}

fn decode_icon() -> Result<Icon, Box<dyn std::error::Error>> {
    let image = Image::from_buffer(
        include_bytes!("../assets/icon.png"),
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::Default,
        RenderAssetUsages::MAIN_WORLD,
    )?;
    let (width, height) = (image.width(), image.height());
    let pixels = image.data.ok_or("icon image has no pixel data")?;
    Ok(Icon::from_rgba(pixels, width, height)?)
}
