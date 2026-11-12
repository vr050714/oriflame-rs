//! Oriflamme: one human player against computer players.

// Release builds on Windows start without a console window. Debug builds keep
// it, so log output stays visible during development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod game;
mod screens;

use bevy::{
    asset::RenderAssetUsages,
    ecs::system::NonSendMarker,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::*,
    render::{
        RenderPlugin,
        settings::{Backends, WgpuSettings},
    },
    window::PrimaryWindow,
    winit::WINIT_WINDOWS,
};
use winit::window::Icon;

/// Top-level screens of the application.
#[derive(States, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Screen {
    #[default]
    Menu,
    Playing,
    GameOver,
}

fn main() -> AppExit {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Oriflamme".into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: wgpu_settings().into(),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(screens::FELT))
        .init_state::<Screen>()
        .init_resource::<game::Settings>()
        .init_resource::<game::BotTurn>()
        .init_resource::<screens::CardArt>()
        .insert_resource(screens::UiDirty(true))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (game::play_scenes, game::bot_turns)
                .chain()
                .run_if(in_state(Screen::Playing)),
        )
        .add_systems(
            Update,
            (
                screens::rebuild_ui,
                screens::hover_style,
                screens::animate,
                set_window_icon,
            ),
        )
        .add_observer(screens::on_click)
        .run()
}

/// Graphics settings. On Windows the default backend is DirectX 12, because
/// the Vulkan driver of some integrated GPUs crashes at start-up. Setting the
/// `WGPU_BACKEND` environment variable overrides this.
fn wgpu_settings() -> WgpuSettings {
    let mut settings = WgpuSettings::default();
    if cfg!(windows) && std::env::var_os("WGPU_BACKEND").is_none() {
        settings.backends = Some(Backends::DX12);
    }
    settings
}

/// Sets the icon in the title bar. Runs until the window exists, then once.
/// The executable's own icon (Explorer, taskbar) is embedded by `build.rs`.
fn set_window_icon(
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

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
