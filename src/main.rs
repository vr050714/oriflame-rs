//! Oriflamme: one human player against computer players.

// Release builds on Windows start without a console window. Debug builds keep
// it, so log output stays visible during development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod game;
mod i18n;
#[cfg(not(target_arch = "wasm32"))]
mod icon;
mod screens;

use bevy::{
    prelude::*,
    render::{
        RenderPlugin,
        settings::{Backends, WgpuSettings},
    },
};

/// Top-level screens of the application.
#[derive(States, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Screen {
    #[default]
    Menu,
    Playing,
    GameOver,
}

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(primary_window()),
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
    .init_resource::<screens::LanguageMenu>()
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
            screens::fit_ui_scale,
            screens::rebuild_ui,
            screens::hover_style,
            screens::animate,
        ),
    )
    .add_observer(screens::on_click);
    #[cfg(not(target_arch = "wasm32"))]
    app.add_systems(Update, icon::set_window_icon);
    install_font(&mut app);
    app.run()
}

/// Makes Fira Mono with Latin, Latin Extended and Cyrillic letters the
/// default font, which every text uses. Bevy's own default font has ASCII
/// only, so it cannot show Russian or Serbian. The font is compiled into the
/// executable; its licence is `assets/fonts/OFL.txt`.
fn install_font(app: &mut App) {
    let font = Font::from_bytes(include_bytes!("../assets/fonts/FiraMono-Regular.ttf").to_vec());
    let mut fonts = app.world_mut().resource_mut::<Assets<Font>>();
    if let Err(error) = fonts.insert(AssetId::default(), font) {
        error!("cannot install the font: {error}");
    }
}

/// The main window. In the browser it draws into the page's `#bevy` canvas
/// and follows the size of the canvas's parent element.
fn primary_window() -> Window {
    Window {
        title: "Oriflamme".into(),
        #[cfg(target_arch = "wasm32")]
        canvas: Some("#bevy".into()),
        #[cfg(target_arch = "wasm32")]
        fit_canvas_to_parent: true,
        ..default()
    }
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

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
