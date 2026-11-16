//! User interface: menu, board and game-over screens.
//!
//! The interface is rebuilt from the game state whenever [`UiDirty`] is set.
//! Every clickable node carries a [`Click`] that says what it does.

use crate::{
    Screen,
    game::{
        BotTurn, Entry, HUMAN, LogLine, Mark, Piece, Role, RoundLog, Seen, Sentence, Session,
        Settings, Speed, player_name, set_speed, speed,
    },
    i18n::{self, Language, card_description, card_name},
    t,
};
use bevy::{
    asset::RenderAssetUsages,
    ecs::hierarchy::ChildSpawnerCommands,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{Activate, Button},
    window::PrimaryWindow,
};
use oriflamme_ai::Difficulty;
use oriflamme_core::{
    Action, CardKind, Decision, MAX_PLAYERS, MIN_PLAYERS, Phase, PlayerId, ROUNDS, Slot,
};

pub const FELT: Color = Color::srgb(0.10, 0.22, 0.18);
const PANEL: Color = Color::srgb(0.16, 0.30, 0.25);
const INK: Color = Color::srgb(0.17, 0.13, 0.20);
const PAPER: Color = Color::srgb(0.96, 0.91, 0.76);
const GOLD: Color = Color::srgb(0.95, 0.76, 0.19);
const DIM: Color = Color::srgb(0.72, 0.78, 0.74);
const HIDDEN_TINT: Color = Color::srgb(0.62, 0.62, 0.70);
const RED: Color = Color::srgb(0.88, 0.22, 0.18);
/// Text colour of an influence loss; `RED` is too dark to read on the panels.
const LOSS: Color = Color::srgb(1.0, 0.52, 0.46);
/// The parchment of the story and its inks. The serif strokes are thin, so
/// the inks are darker than the interface colours.
const PAGE: Color = PAPER;
const PAGE_INK: Color = Color::srgb(0.08, 0.05, 0.05);
const CRIMSON: Color = Color::srgb(0.50, 0.08, 0.05);
const PAGE_DIM: Color = Color::srgb(0.34, 0.26, 0.20);

/// Size of the arrow that points at the card being resolved.
const ARROW_W: f32 = 26.0;
const ARROW_H: f32 = 22.0;
/// Size of the spear that shows the resolution direction.
const SPEAR_W: f32 = 330.0;
const SPEAR_H: f32 = 40.0;

const CARD_W: f32 = 96.0;
const CARD_H: f32 = 128.0;
/// Size of the art inside the 5-pixel card border.
const ART_W: f32 = 86.0;
const ART_H: f32 = 118.0;
/// Size of the panel under the hand that explains a card.
const INFO_W: f32 = 640.0;
const INFO_H: f32 = 96.0;
/// Size of the strip of the story on the board, and width of the page in the
/// log window.
const LOG_W: f32 = 820.0;
const LOG_PANEL_H: f32 = 136.0;
const LOG_WINDOW_W: f32 = 860.0;
/// Text size of the story on the board and in the log window.
const LOG_TEXT: f32 = 17.0;
const LOG_WINDOW_TEXT: f32 = 20.0;

/// Narrowest width, in UI pixels, at which the screens are laid out: the
/// top bar with its buttons and drop-downs and a score box fit in it. A
/// narrower window shrinks the interface instead.
const BOARD_MIN_W: f32 = 560.0;
const MENU_H: f32 = 640.0;
const MENU_MIN_W: f32 = 560.0;
/// From this width on, the round title sits in the top bar between the
/// buttons; on narrower screens it has a line of its own.
const WIDE_W: f32 = 900.0;
/// Width taken by one card in a row of cards, its gap included: the card
/// with its emphasis ring and a little room for a long caption.
const CARD_SLOT_W: f32 = 118.0;
/// Extra width of a Royal Decree "move here" slot before a queue card.
const DECREE_GAP_W: f32 = 52.0;
const MOVE_SLOT_W: f32 = 40.0;
/// Gap between cards in a row and between rows.
const CARD_GAP: f32 = 8.0;
/// Height of a queue card with the arrow space and two caption lines, and of
/// a hand card with one caption line.
const QUEUE_CARD_H: f32 = 201.0;
const HAND_CARD_H: f32 = 158.0;
/// Padding of the screen and of the queue panel.
const PAGE_PAD: f32 = 12.0;
const QUEUE_PAD: f32 = 10.0;
/// Gap between the parts of a screen.
const SECTION_GAP: f32 = 10.0;
/// Estimated width of one score box and height of a line of them.
const SCORE_W: f32 = 420.0;
const SCORE_H: f32 = 44.0;
/// Smallest scale; below it the text is too small to read.
const MIN_SCALE: f32 = 0.5;

/// Width of the window in UI pixels at the current scale. The board splits
/// rows of cards by it.
#[derive(Resource, Default)]
pub struct ViewWidth(pub f32);

/// How many of `count` items of width `item_w` (gap included) go in a row
/// `width` wide. When they do not fit in one row they are spread evenly over
/// as few rows as possible: 7 cards that need two rows are 4 and 3, not 6
/// and 1.
fn per_row(count: usize, item_w: f32, width: f32) -> usize {
    // The last item needs no gap after it.
    let fit = (((width + CARD_GAP) / item_w).floor() as usize).max(1);
    let rows = count.div_ceil(fit).max(1);
    count.div_ceil(rows).max(1)
}

/// Number of rows `count` items take; see [`per_row`].
fn rows_of(count: usize, item_w: f32, width: f32) -> usize {
    count.div_ceil(per_row(count, item_w, width)).max(1)
}

/// Height of `rows` rows of cards `card_h` tall.
fn rows_height(rows: usize, card_h: f32) -> f32 {
    let rows = rows.max(1) as f32;
    rows * card_h + (rows - 1.0) * CARD_GAP
}

/// What the queue row shows: how many card places (cards and "place here"
/// slots) and the width of each.
fn queue_items(session: &Session) -> (usize, f32) {
    let queue = session
        .scene
        .as_ref()
        .map_or(session.state.queue.len(), |s| s.queue.len());
    if session.move_from.is_some() {
        return (queue, CARD_SLOT_W + DECREE_GAP_W);
    }
    let slots = if session.scene.is_none()
        && session.selected.is_some()
        && matches!(session.state.phase, Phase::Placement { .. })
    {
        2
    } else {
        0
    };
    (queue.saturating_add(slots), CARD_SLOT_W)
}

/// Width available to the queue's cards on a screen `width` wide.
fn queue_width(width: f32) -> f32 {
    width - 2.0 * (PAGE_PAD + QUEUE_PAD)
}

/// Width available to the hand's cards on a screen `width` wide.
fn hand_width(width: f32) -> f32 {
    width - 2.0 * PAGE_PAD
}

/// Height the board needs on a screen `width` wide, estimated from its
/// parts. Parts that come and go during a turn (the Reveal buttons, a second
/// line of the headline) always count, so the scale does not jump.
fn board_height(session: &Session, width: f32) -> f32 {
    let narrow = width < WIDE_W;
    let players = session.state.players.len();
    let score_rows = rows_of(players, SCORE_W + SECTION_GAP, width - 2.0 * PAGE_PAD);
    let (items, item_w) = queue_items(session);
    let queue_rows = rows_of(items, item_w, queue_width(width));
    let hand = session
        .state
        .players
        .get(HUMAN)
        .map_or(0, |player| player.hand.len());
    let hand_rows = rows_of(hand, CARD_SLOT_W, hand_width(width));
    let parts = [
        // Top bar, and the round title under it on a narrow screen.
        48.0,
        if narrow { 34.0 } else { 0.0 },
        rows_height(score_rows, SCORE_H),
        // Headline.
        if narrow { 60.0 } else { 34.0 },
        // Queue panel: the spear and the rows of cards.
        2.0 * QUEUE_PAD + 44.0 + rows_height(queue_rows, QUEUE_CARD_H),
        // Reveal buttons.
        50.0,
        // "Your hand" and the hand.
        22.0,
        rows_height(hand_rows, HAND_CARD_H),
        // Card explanation, taller when its text wraps.
        if narrow { 112.0 } else { 96.0 },
        LOG_PANEL_H,
    ];
    let gaps = parts.iter().filter(|h| **h > 0.0).count().saturating_sub(1) as f32;
    parts.iter().sum::<f32>() + gaps * SECTION_GAP + 2.0 * PAGE_PAD
}

/// Shrinks the whole interface so that it fits the window: the largest scale
/// up to 1 at which the screen, with its rows of cards split as the width
/// forces, is as wide as its narrowest layout and not taller than the window.
/// Records the resulting width in [`ViewWidth`] and redraws when it changes.
pub fn fit_ui_scale(
    mut scale: ResMut<UiScale>,
    mut view: ResMut<ViewWidth>,
    mut dirty: ResMut<UiDirty>,
    windows: Query<&Window, With<PrimaryWindow>>,
    screen: Res<State<Screen>>,
    session: Option<Res<Session>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let (width, height) = (window.width(), window.height());
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let session = session.as_deref();
    let needs = |avail_w: f32| -> (f32, f32) {
        match (screen.get(), session) {
            (Screen::Playing, Some(session)) => (BOARD_MIN_W, board_height(session, avail_w)),
            (Screen::GameOver, _) => (MENU_MIN_W, MENU_H * 0.7),
            _ => (MENU_MIN_W, MENU_H),
        }
    };
    let mut best = MIN_SCALE;
    let mut candidate = 1.0_f32;
    while candidate >= MIN_SCALE {
        let avail_w = width / candidate;
        let (min_w, needed_h) = needs(avail_w);
        if avail_w >= min_w && needed_h <= height / candidate {
            best = candidate;
            break;
        }
        candidate -= 0.025;
    }
    if (scale.0 - best).abs() > 0.001 {
        scale.0 = best;
    }
    let view_w = width / scale.0;
    if (view.0 - view_w).abs() > 0.5 {
        view.0 = view_w;
        dirty.0 = true;
    }
}

/// Family colour of each seat.
fn family_color(player: PlayerId) -> Color {
    match player {
        0 => Color::srgb(0.85, 0.22, 0.20),
        1 => Color::srgb(0.24, 0.46, 0.80),
        2 => Color::srgb(0.25, 0.62, 0.32),
        3 => Color::srgb(0.58, 0.36, 0.78),
        _ => Color::srgb(0.92, 0.55, 0.18),
    }
}

/// Card images and the story font, decoded once and kept for the whole run.
///
/// The files are compiled into the executable, so the game runs from any
/// folder without an `assets` directory beside it.
#[derive(Resource)]
pub struct CardArt {
    faces: Vec<(CardKind, Handle<Image>)>,
    back: Handle<Image>,
    /// Spear pointing in the direction the queue is resolved.
    spear: Handle<Image>,
    /// Arrow pointing down at a card.
    arrow: Handle<Image>,
    /// Card miniatures for the game log.
    minis: Vec<(CardKind, Handle<Image>)>,
    /// Miniature of a face-down card.
    mini_back: Handle<Image>,
    /// Book font of the story in the log: PT Serif, with Cyrillic.
    story_font: Handle<Font>,
}

/// The PNG bytes of a card miniature.
fn mini_png(kind: CardKind) -> &'static [u8] {
    match kind {
        CardKind::Archer => include_bytes!("../assets/mini/archer.png"),
        CardKind::Soldier => include_bytes!("../assets/mini/soldier.png"),
        CardKind::Spy => include_bytes!("../assets/mini/spy.png"),
        CardKind::Heir => include_bytes!("../assets/mini/heir.png"),
        CardKind::Assassination => include_bytes!("../assets/mini/assassination.png"),
        CardKind::RoyalDecree => include_bytes!("../assets/mini/royal_decree.png"),
        CardKind::Lord => include_bytes!("../assets/mini/lord.png"),
        CardKind::Shapeshifter => include_bytes!("../assets/mini/shapeshifter.png"),
        CardKind::Ambush => include_bytes!("../assets/mini/ambush.png"),
        CardKind::Conspiracy => include_bytes!("../assets/mini/conspiracy.png"),
    }
}

/// The PNG bytes of a card face.
fn face_png(kind: CardKind) -> &'static [u8] {
    match kind {
        CardKind::Archer => include_bytes!("../assets/cards/archer.png"),
        CardKind::Soldier => include_bytes!("../assets/cards/soldier.png"),
        CardKind::Spy => include_bytes!("../assets/cards/spy.png"),
        CardKind::Heir => include_bytes!("../assets/cards/heir.png"),
        CardKind::Assassination => include_bytes!("../assets/cards/assassination.png"),
        CardKind::RoyalDecree => include_bytes!("../assets/cards/royal_decree.png"),
        CardKind::Lord => include_bytes!("../assets/cards/lord.png"),
        CardKind::Shapeshifter => include_bytes!("../assets/cards/shapeshifter.png"),
        CardKind::Ambush => include_bytes!("../assets/cards/ambush.png"),
        CardKind::Conspiracy => include_bytes!("../assets/cards/conspiracy.png"),
    }
}

impl FromWorld for CardArt {
    fn from_world(world: &mut World) -> Self {
        // PT Serif, unmodified; its licence is assets/fonts/PTSerif-LICENSE.txt.
        let story_font = world.resource_mut::<Assets<Font>>().add(Font::from_bytes(
            include_bytes!("../assets/fonts/PTSerif-Regular.ttf").to_vec(),
        ));
        let mut images = world.resource_mut::<Assets<Image>>();
        let mut decode = |name: &str, png: &[u8]| {
            let image = Image::from_buffer(
                png,
                ImageType::Extension("png"),
                CompressedImageFormats::NONE,
                true,
                ImageSampler::Default,
                RenderAssetUsages::RENDER_WORLD,
            );
            match image {
                Ok(image) => images.add(image),
                Err(error) => {
                    error!("cannot decode card image {name}: {error}");
                    Handle::default()
                }
            }
        };
        Self {
            faces: CardKind::ALL
                .into_iter()
                .map(|kind| (kind, decode(kind.slug(), face_png(kind))))
                .collect(),
            back: decode("back", include_bytes!("../assets/cards/back.png")),
            spear: decode("spear", include_bytes!("../assets/ui/spear.png")),
            arrow: decode("arrow", include_bytes!("../assets/ui/arrow.png")),
            minis: CardKind::ALL
                .into_iter()
                .map(|kind| (kind, decode(kind.slug(), mini_png(kind))))
                .collect(),
            mini_back: decode("mini back", include_bytes!("../assets/mini/back.png")),
            story_font,
        }
    }
}

impl CardArt {
    /// The face of `kind`, or the card back for `None`.
    fn image(&self, kind: Option<CardKind>) -> Handle<Image> {
        kind.and_then(|kind| self.faces.iter().find(|(k, _)| *k == kind))
            .map_or_else(|| self.back.clone(), |(_, handle)| handle.clone())
    }

    /// The miniature of `kind`, or of a face-down card for `None`.
    fn mini(&self, kind: Option<CardKind>) -> Handle<Image> {
        kind.and_then(|kind| self.minis.iter().find(|(k, _)| *k == kind))
            .map_or_else(|| self.mini_back.clone(), |(_, handle)| handle.clone())
    }
}

/// How a node moves. The motion starts when the node is spawned.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
#[require(MotionClock, UiTransform)]
pub enum Motion {
    /// Grows and shrinks a little: the card that acts or is affected.
    Pulse,
    /// Stays for a moment, then spins and shrinks to nothing: a card leaving.
    Vanish,
    /// Grows to full size: a card that was just put down.
    Arrive,
    /// Moves up and down: the arrow above a card.
    Bob,
}

/// Seconds since a moving node was spawned.
#[derive(Component, Default)]
pub struct MotionClock(f32);

/// Moves every node that has a [`Motion`].
pub fn animate(time: Res<Time>, mut nodes: Query<(&Motion, &mut MotionClock, &mut UiTransform)>) {
    // A slower speed stretches every animation; scene lengths stay the same.
    let step = time.delta_secs() / speed().factor();
    for (motion, mut clock, mut transform) in &mut nodes {
        clock.0 += step;
        let t = clock.0;
        match motion {
            Motion::Pulse => transform.scale = Vec2::splat(1.0 + 0.08 * (t * 7.0).sin().abs()),
            Motion::Vanish => {
                let gone = ((t - 0.5) / 0.6).clamp(0.0, 1.0);
                transform.scale = Vec2::splat(1.0 - gone);
                transform.rotation = Rot2::radians(gone * 1.2);
            }
            Motion::Arrive => {
                let there = (t / 0.35).clamp(0.0, 1.0);
                transform.scale = Vec2::splat(0.4 + 0.6 * there);
            }
            Motion::Bob => transform.translation = Val2::px(0.0, 4.0 * (t * 6.0).sin()),
        }
    }
}

/// Set to rebuild the interface on the next frame.
#[derive(Resource)]
pub struct UiDirty(pub bool);

/// True while the language list under the language button is open.
#[derive(Resource, Default)]
pub struct LanguageMenu(pub bool);

/// True while the speed list under the speed button is open.
#[derive(Resource, Default)]
pub struct SpeedMenu(pub bool);

/// Root node of the current screen.
#[derive(Component)]
pub struct UiRoot;

/// Border colour of a button when the pointer is not over it.
#[derive(Component)]
pub struct RestBorder(Color);

/// What a button does.
#[derive(Component, Clone, Copy)]
pub enum Click {
    Players(usize),
    Level(Difficulty),
    Start,
    Menu,
    /// Pick a hand card to place.
    Hand(CardKind),
    /// Place the picked hand card here.
    Slot(Slot),
    /// Play this action directly.
    Do(Action),
    /// Royal Decree: pick the card to move.
    MoveFrom(usize),
    CancelMove,
    /// Open or close the language list.
    LanguageMenu,
    Language(Language),
    /// Open or close the playback speed list.
    SpeedMenu,
    Speed(Speed),
    /// Open or close the log window.
    Log,
    /// Show the story of this round in the log window.
    LogRound(u8),
}

/// Handles every button press.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per resource"
)]
pub fn on_click(
    event: On<Activate>,
    clicks: Query<&Click>,
    mut commands: Commands,
    mut settings: ResMut<Settings>,
    mut session: Option<ResMut<Session>>,
    mut bot: ResMut<BotTurn>,
    mut dirty: ResMut<UiDirty>,
    mut next: ResMut<NextState<Screen>>,
    mut language_menu: ResMut<LanguageMenu>,
    mut speed_menu: ResMut<SpeedMenu>,
) {
    let Ok(&click) = clicks.get(event.entity) else {
        return;
    };
    dirty.0 = true;
    // Any click closes both lists; a list's own button toggles it.
    let language_was_open = std::mem::take(&mut language_menu.0);
    let speed_was_open = std::mem::take(&mut speed_menu.0);
    match click {
        Click::LanguageMenu => language_menu.0 = !language_was_open,
        Click::Language(language) => i18n::set_language(language),
        Click::SpeedMenu => speed_menu.0 = !speed_was_open,
        Click::Speed(new_speed) => set_speed(new_speed),
        Click::Players(count) => settings.players = count,
        Click::Level(difficulty) => settings.difficulty = difficulty,
        Click::Start => match Session::new(*settings) {
            Ok(new_session) => {
                commands.insert_resource(new_session);
                bot.reset();
                next.set(Screen::Playing);
            }
            Err(error) => error!("cannot start a game: {error}"),
        },
        Click::Menu => next.set(Screen::Menu),
        Click::Log | Click::LogRound(_) => {
            if let Some(session) = session.as_mut() {
                session.log_window = match click {
                    Click::LogRound(round) => Some(round),
                    _ if session.log_window.is_some() => None,
                    _ => session.log.last().map(|story| story.round),
                };
            }
        }
        Click::Hand(_) | Click::Slot(_) | Click::Do(_) | Click::MoveFrom(_) | Click::CancelMove => {
            let Some(session) = session.as_mut().filter(|s| s.human_turn()) else {
                return;
            };
            match click {
                Click::Hand(kind) => session.selected = Some(kind),
                Click::Slot(slot) => {
                    if let Some(kind) = session.selected {
                        session.apply(Action::Place { kind, slot });
                        bot.reset();
                    }
                }
                Click::Do(action) => {
                    session.apply(action);
                    bot.reset();
                }
                Click::MoveFrom(pos) => session.move_from = Some(pos),
                _ => session.move_from = None,
            }
        }
    }
}

/// Shows a white border on the button under the pointer.
pub fn hover_style(
    mut buttons: Query<(&Hovered, &RestBorder, &mut BorderColor), Changed<Hovered>>,
) {
    for (hovered, rest, mut border) in &mut buttons {
        *border = BorderColor::all(if hovered.get() { Color::WHITE } else { rest.0 });
    }
}

/// Rebuilds the screen after a change.
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy systems take one parameter per resource"
)]
pub fn rebuild_ui(
    mut commands: Commands,
    mut dirty: ResMut<UiDirty>,
    screen: Res<State<Screen>>,
    roots: Query<Entity, With<UiRoot>>,
    settings: Res<Settings>,
    session: Option<Res<Session>>,
    art: Res<CardArt>,
    language_menu: Res<LanguageMenu>,
    speed_menu: Res<SpeedMenu>,
    view: Res<ViewWidth>,
) {
    if !dirty.0 && !screen.is_changed() {
        return;
    }
    dirty.0 = false;
    for root in &roots {
        commands.entity(root).despawn();
    }
    let mut root = commands.spawn((
        UiRoot,
        Node {
            width: percent(100),
            height: percent(100),
            // The root is a grid item of Bevy's viewport node, whose least
            // width is otherwise its content's: the screen would grow wider
            // than the window instead of wrapping.
            min_width: px(0),
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(px(PAGE_PAD)),
            row_gap: px(SECTION_GAP),
            ..default()
        },
        BackgroundColor(FELT),
    ));
    let pickers = Pickers {
        language_open: language_menu.0,
        speed_open: speed_menu.0,
    };
    root.with_children(|ui| match (screen.get(), session.as_deref()) {
        (Screen::Playing, Some(session)) => board(ui, session, &art, pickers, view.0),
        (Screen::GameOver, Some(session)) => {
            top_bar(ui, pickers, |_| {}, None);
            game_over(ui, session, &art);
        }
        _ => {
            top_bar(ui, pickers, |_| {}, None);
            menu(ui, &settings, &art);
        }
    });
}

/// Which drop-downs of the top bar are open.
#[derive(Clone, Copy)]
struct Pickers {
    language_open: bool,
    speed_open: bool,
}

/// The bar at the top of every screen: the screen's own buttons on the left,
/// the playback speed and language drop-downs on the right, and `title`
/// between them.
fn top_bar(
    ui: &mut ChildSpawnerCommands,
    pickers: Pickers,
    left: impl FnOnce(&mut ChildSpawnerCommands),
    title: Option<String>,
) {
    ui.spawn((
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(8),
            row_gap: px(8),
            flex_shrink: 0.0,
            ..default()
        },
        // Drawn over the screen, so an open list covers what is under it.
        GlobalZIndex(10),
    ))
    .with_children(|ui| {
        // Each group of buttons stays on one line.
        let group = Node {
            flex_wrap: FlexWrap::NoWrap,
            flex_shrink: 0.0,
            align_items: AlignItems::FlexStart,
            ..row(8.0)
        };
        ui.spawn(group.clone()).with_children(left);
        if let Some(title) = title {
            ui.spawn(Node {
                flex_grow: 1.0,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|ui| {
                ui.spawn(text(title, 26.0, GOLD));
            });
        }
        ui.spawn(group).with_children(|ui| {
            speed_picker(ui, pickers.speed_open);
            language_picker(ui, pickers.language_open);
        });
    });
}

/// The speed drop-down: a button with the current speed and, when open, the
/// list of speeds under it.
fn speed_picker(ui: &mut ChildSpawnerCommands, open: bool) {
    let current = speed();
    dropdown(ui, &t!(current.key()), Click::SpeedMenu, open, |ui| {
        for option in Speed::ALL {
            button(
                ui,
                &t!(option.key()),
                Click::Speed(option),
                option == current,
            );
        }
    });
}

/// The language drop-down: a button with the current language and, when open,
/// the list of languages under it.
fn language_picker(ui: &mut ChildSpawnerCommands, open: bool) {
    let current = i18n::language();
    dropdown(ui, current.native_name(), Click::LanguageMenu, open, |ui| {
        for language in Language::ALL {
            button(
                ui,
                language.native_name(),
                Click::Language(language),
                language == current,
            );
        }
    });
}

/// A button showing `label`; when `open`, `options` are listed under it.
fn dropdown(
    ui: &mut ChildSpawnerCommands,
    label: &str,
    toggle: Click,
    open: bool,
    options: impl FnOnce(&mut ChildSpawnerCommands),
) {
    ui.spawn(Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        ..default()
    })
    .with_children(|ui| {
        ui.spawn((
            Button,
            Hovered::default(),
            toggle,
            RestBorder(INK),
            Node {
                padding: UiRect::axes(px(14), px(7)),
                border: UiRect::all(px(3)),
                border_radius: BorderRadius::all(px(10)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: px(12),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(INK),
        ))
        .with_children(|ui| {
            ui.spawn(text(label, 18.0, PAPER));
            caret(ui, open);
        });
        if open {
            // Under the button, over whatever is below it.
            ui.spawn(Node {
                position_type: PositionType::Absolute,
                top: percent(100),
                right: px(0),
                min_width: percent(100),
                margin: UiRect::top(px(4)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: px(4),
                ..default()
            })
            .with_children(options);
        }
    });
}

/// A small chevron: pointing down when the list is closed, up when open.
fn caret(ui: &mut ChildSpawnerCommands, open: bool) {
    let angle = if open { -135.0_f32 } else { 45.0_f32 };
    ui.spawn((
        Node {
            width: px(8),
            height: px(8),
            border: UiRect {
                right: px(2),
                bottom: px(2),
                ..default()
            },
            margin: UiRect::top(px(if open { 4 } else { -4 })),
            ..default()
        },
        BorderColor::all(PAPER),
        UiTransform {
            rotation: Rot2::degrees(angle),
            ..UiTransform::IDENTITY
        },
    ));
}

fn text(content: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(content),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

/// Text whose lines are centred when it wraps.
fn centered(content: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        text(content, size, color),
        TextLayout::justify(Justify::Center),
    )
}

fn row(gap: f32) -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        flex_wrap: FlexWrap::Wrap,
        column_gap: px(gap),
        row_gap: px(gap),
        ..default()
    }
}

/// A wrapping row as wide as its parent. A wrapping row that only takes the
/// width of its content can be measured on one line and then laid out on
/// two, and its second line covers what comes after it.
fn wide_row(gap: f32) -> Node {
    Node {
        width: percent(100),
        ..row(gap)
    }
}

fn column(gap: f32) -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: px(gap),
        ..default()
    }
}

/// A text button. `active` marks the current choice.
fn button(ui: &mut ChildSpawnerCommands, label: &str, click: Click, active: bool) {
    let (fill, ink) = if active { (GOLD, INK) } else { (PANEL, PAPER) };
    ui.spawn((
        Button,
        Hovered::default(),
        click,
        RestBorder(INK),
        Node {
            padding: UiRect::axes(px(18), px(9)),
            border: UiRect::all(px(3)),
            border_radius: BorderRadius::all(px(10)),
            ..default()
        },
        BackgroundColor(fill),
        BorderColor::all(INK),
        children![text(label, 22.0, ink)],
    ));
}

/// A smaller text button, for the round tabs of the log window.
fn small_button(ui: &mut ChildSpawnerCommands, label: &str, click: Click, active: bool) {
    let (fill, ink) = if active { (GOLD, INK) } else { (PANEL, PAPER) };
    ui.spawn((
        Button,
        Hovered::default(),
        click,
        RestBorder(INK),
        Node {
            padding: UiRect::axes(px(12), px(5)),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(8)),
            ..default()
        },
        BackgroundColor(fill),
        BorderColor::all(INK),
        children![text(label, 17.0, ink)],
    ));
}

/// How a card is drawn.
struct CardLook<'a> {
    owner: PlayerId,
    /// Art to show; `None` shows the card back.
    kind: Option<CardKind>,
    /// The human's own face-down card: art shown dimmed.
    secret: bool,
    influence: u32,
    /// Cards in the stack, this one included.
    depth: usize,
    click: Option<Click>,
    /// Draw attention: a legal choice or the card being resolved.
    highlight: bool,
    caption: String,
    /// Second caption line. `Some("")` keeps the line empty so that cards in
    /// one row stay aligned; `None` leaves the line out.
    note: Option<String>,
    /// Arrow above the card. `Some(false)` leaves the space empty so that
    /// cards in one row stay aligned; `None` leaves the space out.
    arrow: Option<bool>,
    /// The card's part in the scene being shown.
    mark: Option<&'a Mark>,
}

/// The space above a queue card, with the arrow in it when `pointing`.
fn arrow_space(ui: &mut ChildSpawnerCommands, art: &CardArt, pointing: bool) {
    let node = Node {
        width: px(ARROW_W),
        height: px(ARROW_H),
        ..default()
    };
    if pointing {
        ui.spawn((ImageNode::new(art.arrow.clone()), node, Motion::Bob));
    } else {
        ui.spawn(node);
    }
}

fn card(ui: &mut ChildSpawnerCommands, art: &CardArt, look: CardLook) {
    let image = ImageNode {
        color: if look.secret {
            HIDDEN_TINT
        } else {
            Color::WHITE
        },
        ..ImageNode::new(art.image(look.kind))
    };
    // The emphasis colour is an outer ring around the owner's colour frame, so
    // the owner stays visible. The ring is transparent when there is none, so
    // every card has the same size.
    let ring = match look.mark.map(|mark| mark.role) {
        Some(Role::Target) => RED,
        Some(Role::Actor) => GOLD,
        None if look.highlight => GOLD,
        None => Color::NONE,
    };

    ui.spawn(column(3.0)).with_children(|ui| {
        if let Some(pointing) = look.arrow {
            arrow_space(ui, art, pointing);
        }
        let mut outer = ui.spawn((
            Node {
                border: UiRect::all(px(5)),
                border_radius: BorderRadius::all(px(17)),
                ..default()
            },
            BorderColor::all(ring),
        ));
        if let Some(click) = look.click {
            outer.insert((Button, Hovered::default(), click, RestBorder(ring)));
        }
        if let Some(mark) = look.mark {
            outer.insert(mark.motion);
            if mark.motion == Motion::Arrive {
                // Start small, so the card is not drawn full size for one frame.
                outer.insert(UiTransform {
                    scale: Vec2::splat(0.4),
                    ..UiTransform::IDENTITY
                });
            }
        }
        outer.with_children(|ui| {
            let mut frame = ui.spawn((
                Node {
                    width: px(CARD_W),
                    height: px(CARD_H),
                    border: UiRect::all(px(5)),
                    border_radius: BorderRadius::all(px(12)),
                    ..default()
                },
                BackgroundColor(family_color(look.owner)),
                BorderColor::all(family_color(look.owner)),
            ));
            frame.with_children(|ui| {
                ui.spawn((
                    image,
                    Node {
                        width: px(ART_W),
                        height: px(ART_H),
                        ..default()
                    },
                ));
                if look.influence > 0 {
                    badge(ui, look.influence.to_string(), GOLD, true);
                }
                if look.depth > 1 {
                    badge(ui, format!("x{}", look.depth), PAPER, false);
                }
                if let Some(mark) = look.mark
                    && let Some(label) = &mark.label
                {
                    effect_label(ui, &label.text(), mark.role);
                }
            });
        });
        ui.spawn(text(look.caption, 14.0, PAPER));
        if let Some(note) = look.note {
            // An empty text node has no height, so a blank line needs a space.
            let note = if note.is_empty() {
                " ".to_owned()
            } else {
                note
            };
            ui.spawn(text(note, 12.0, DIM));
        }
    });
}

/// What happens to a card in the current scene, written across its middle.
fn effect_label(ui: &mut ChildSpawnerCommands, label: &str, role: Role) {
    let (fill, ink) = match role {
        Role::Actor => (GOLD, INK),
        Role::Target => (RED, Color::WHITE),
    };
    ui.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            right: px(0),
            top: px(44),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            Node {
                padding: UiRect::axes(px(6), px(2)),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(INK),
            children![text(label, 15.0, ink)],
        )],
    ));
}

/// An empty card-shaped place in the queue where the picked card can go.
fn place_slot(ui: &mut ChildSpawnerCommands, slot: Slot) {
    ui.spawn(column(3.0)).with_children(|ui| {
        // Same space as the arrow above a card, to stay level with the cards.
        ui.spawn(Node {
            width: px(ARROW_W),
            height: px(ARROW_H),
            ..default()
        });
        ui.spawn((
            Button,
            Hovered::default(),
            Click::Slot(slot),
            RestBorder(GOLD),
            Node {
                width: px(CARD_W),
                height: px(CARD_H),
                border: UiRect::all(px(5)),
                border_radius: BorderRadius::all(px(12)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(FELT),
            BorderColor::all(GOLD),
            children![text(t!("place"), 18.0, GOLD), text(t!("here"), 18.0, GOLD)],
        ));
        // Blank caption lines keep the slot level with the cards beside it.
        ui.spawn(text(" ", 14.0, PAPER));
        ui.spawn(text(" ", 12.0, DIM));
    });
}

/// A small label in a corner of a card.
fn badge(ui: &mut ChildSpawnerCommands, label: String, fill: Color, top: bool) {
    let (top, bottom) = if top {
        (px(2), auto())
    } else {
        (auto(), px(2))
    };
    ui.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: px(2),
            top,
            bottom,
            padding: UiRect::axes(px(7), px(1)),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(10)),
            ..default()
        },
        BackgroundColor(fill),
        BorderColor::all(INK),
        children![text(label, 16.0, INK)],
    ));
}

fn menu(ui: &mut ChildSpawnerCommands, settings: &Settings, art: &CardArt) {
    // The game's name is not translated.
    ui.spawn(text("Oriflamme", 72.0, GOLD));
    ui.spawn(centered(t!("app-tagline"), 22.0, DIM));

    // A hand of cards in one line. When the screen is too narrow the slots
    // shrink and the cards overlap, like cards held in a hand; the last card
    // always stays fully visible.
    const HAND: [CardKind; 5] = [
        CardKind::Lord,
        CardKind::Heir,
        CardKind::Spy,
        CardKind::Archer,
        CardKind::Ambush,
    ];
    ui.spawn(Node {
        width: percent(100),
        height: px(CARD_H + 24.0),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::NoWrap,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        column_gap: px(10),
        ..default()
    })
    .with_children(|ui| {
        let last = HAND.len().saturating_sub(1);
        for (i, kind) in HAND.into_iter().enumerate() {
            // Fan: rotate around the middle card, lift the middle a little.
            let offset = i as f32 - last as f32 / 2.0;
            ui.spawn((
                Node {
                    width: px(CARD_W),
                    height: px(CARD_H),
                    min_width: px(24),
                    flex_shrink: if i == last { 0.0 } else { 1.0 },
                    margin: UiRect::top(px(offset * offset * 4.0)),
                    ..default()
                },
                UiTransform {
                    rotation: Rot2::degrees(offset * 5.0),
                    ..UiTransform::IDENTITY
                },
            ))
            .with_children(|ui| {
                ui.spawn((
                    ImageNode::new(art.image(Some(kind))),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        width: px(CARD_W),
                        height: px(CARD_H),
                        ..default()
                    },
                ));
            });
        }
    });

    ui.spawn(text(t!("players"), 24.0, PAPER));
    ui.spawn(wide_row(10.0)).with_children(|ui| {
        for count in MIN_PLAYERS..=MAX_PLAYERS {
            let label = t!("player-count", count = count, pcs = count.saturating_sub(1));
            button(ui, &label, Click::Players(count), settings.players == count);
        }
    });

    ui.spawn(text(t!("strength"), 24.0, PAPER));
    ui.spawn(wide_row(10.0)).with_children(|ui| {
        for level in Difficulty::ALL {
            let name = match level {
                Difficulty::Easy => t!("level-easy"),
                Difficulty::Medium => t!("level-medium"),
                Difficulty::Hard => t!("level-hard"),
            };
            button(ui, &name, Click::Level(level), settings.difficulty == level);
        }
    });

    ui.spawn(Node {
        height: px(10),
        ..default()
    });
    button(ui, &t!("start-game"), Click::Start, true);
}

/// What the human should do now, or who is moving.
fn prompt(session: &Session) -> String {
    let state = &session.state;
    let Some(player) = state.current_player() else {
        return t!("game-is-over");
    };
    if player != HUMAN {
        return t!("thinking", player = player_name(player));
    }
    match state.phase {
        Phase::Placement { .. } => match session.selected {
            None => t!("pick-hand-card"),
            Some(kind) => t!("pick-place", card = card_name(kind)),
        },
        Phase::Resolution {
            card,
            decision: Decision::RevealOrNot,
        } => t!("reveal-or-not", card = card_name(card.kind)),
        Phase::Resolution {
            decision: Decision::Ability(CardKind::RoyalDecree),
            ..
        } => match session.move_from {
            None => t!("decree-pick-card"),
            Some(_) => t!("decree-pick-place"),
        },
        Phase::Resolution {
            decision: Decision::Ability(kind),
            ..
        } => t!("pick-target", card = card_name(kind)),
        Phase::GameOver => t!("game-is-over"),
    }
}

/// The game table. `width` is the screen width in UI pixels; rows of cards
/// that do not fit in it are split evenly.
fn board(
    ui: &mut ChildSpawnerCommands,
    session: &Session,
    art: &CardArt,
    pickers: Pickers,
    width: f32,
) {
    let state = &session.state;
    let legal = if session.human_turn() {
        state.legal_actions()
    } else {
        Vec::new()
    };
    // While the last action is shown, the table is the scene's frozen copy.
    let scene = session.scene.as_ref();
    let queue = scene.map_or(&state.queue, |scene| &scene.queue);
    let current = if scene.is_some() {
        None
    } else {
        state.current_player()
    };

    // The round title sits in the top bar when there is room, else under it.
    let title = t!("round-of", round = state.round, rounds = ROUNDS);
    let wide = width >= WIDE_W;
    top_bar(
        ui,
        pickers,
        |ui| {
            button(ui, &t!("menu"), Click::Menu, false);
            button(
                ui,
                &t!("log-open"),
                Click::Log,
                session.log_window.is_some(),
            );
        },
        wide.then(|| title.clone()),
    );
    if !wide {
        ui.spawn(centered(title, 26.0, GOLD));
    }

    // Scores.
    ui.spawn(wide_row(10.0)).with_children(|ui| {
        for (player, info) in state.players.iter().enumerate() {
            let active = current == Some(player);
            let influence = scene
                .and_then(|scene| scene.influence.get(player).copied())
                .unwrap_or(info.influence);
            ui.spawn((
                Node {
                    padding: UiRect::axes(px(14), px(6)),
                    border: UiRect::all(px(4)),
                    border_radius: BorderRadius::all(px(10)),
                    column_gap: px(10),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderColor::all(if active { GOLD } else { family_color(player) }),
                children![
                    (
                        Node {
                            width: px(18),
                            height: px(18),
                            border_radius: BorderRadius::MAX,
                            ..default()
                        },
                        BackgroundColor(family_color(player)),
                    ),
                    text(player_name(player), 20.0, PAPER),
                    text(t!("influence", amount = influence), 20.0, GOLD),
                    text(t!("in-hand", count = info.hand.len()), 16.0, DIM),
                ],
            ));
        }
    });

    // The headline: the event being shown, or what to do now.
    match scene {
        Some(scene) => story_line(ui, art, &scene.entry),
        None => {
            ui.spawn(centered(prompt(session), 22.0, PAPER));
        }
    }

    // The card the arrow points at: the one acting in the scene, or the one
    // waiting for its owner's decision.
    let pointed = match scene {
        Some(scene) => scene
            .marks
            .iter()
            .find(|mark| mark.role == Role::Actor)
            .map(|mark| mark.pos),
        None => match state.phase {
            Phase::Resolution { card, .. } => state.position_of(card),
            _ => None,
        },
    };
    let placing = session
        .selected
        .filter(|_| matches!(state.phase, Phase::Placement { .. }));
    let can_place =
        |slot: Slot| placing.is_some_and(|kind| legal.contains(&Action::Place { kind, slot }));

    // The influence queue. The spear points the way it is resolved.
    ui.spawn((
        Node {
            padding: UiRect::all(px(10)),
            width: percent(100),
            ..column(4.0)
        },
        BackgroundColor(PANEL),
    ))
    .with_children(|ui| {
        ui.spawn((
            ImageNode::new(art.spear.clone()),
            Node {
                width: px(SPEAR_W),
                height: px(SPEAR_H),
                ..default()
            },
        ));
        // Cards and "place here" slots, in queue order, split into rows.
        let mut items = Vec::with_capacity(queue.len().saturating_add(2));
        if can_place(Slot::Left) {
            items.push(QueueItem::Place(Slot::Left));
        }
        items.extend((0..queue.len()).map(QueueItem::Card));
        if can_place(Slot::Right) {
            items.push(QueueItem::Place(Slot::Right));
        }
        let (_, item_w) = queue_items(session);
        let row_len = per_row(items.len(), item_w, queue_width(width));
        ui.spawn(Node {
            width: percent(100),
            min_height: px(QUEUE_CARD_H),
            justify_content: JustifyContent::Center,
            ..column(CARD_GAP)
        })
        .with_children(|ui| {
            if items.is_empty() {
                ui.spawn(text(t!("queue-empty"), 20.0, DIM));
            }
            let rows = items.chunks(row_len).count();
            for (row_index, chunk) in items.chunks(row_len).enumerate() {
                ui.spawn(Node {
                    width: percent(100),
                    ..row(CARD_GAP)
                })
                .with_children(|ui| {
                    for &item in chunk {
                        match item {
                            QueueItem::Place(slot) => place_slot(ui, slot),
                            QueueItem::Card(pos) => {
                                move_gap(ui, session, &legal, pos);
                                queue_card(ui, session, art, &legal, pos, pointed, &can_place);
                            }
                        }
                    }
                    if row_index.saturating_add(1) == rows {
                        move_gap(ui, session, &legal, queue.len());
                    }
                });
            }
        });
    });

    // Choices that are not a card on the table.
    ui.spawn(wide_row(10.0)).with_children(|ui| {
        if legal.contains(&Action::Reveal) {
            button(ui, &t!("reveal"), Click::Do(Action::Reveal), true);
            button(ui, &t!("keep-hidden"), Click::Do(Action::KeepHidden), false);
        }
        if session.move_from.is_some() {
            button(ui, &t!("pick-another"), Click::CancelMove, false);
        }
    });

    // The human's hand.
    ui.spawn(text(t!("your-hand"), 18.0, DIM));
    let hand = state.players.get(HUMAN).map_or(&[][..], |p| &p.hand);
    let my_turn_to_place = session.human_turn() && matches!(state.phase, Phase::Placement { .. });
    let row_len = per_row(hand.len(), CARD_SLOT_W, hand_width(width));
    ui.spawn(Node {
        width: percent(100),
        ..column(CARD_GAP)
    })
    .with_children(|ui| {
        for chunk in hand.chunks(row_len) {
            ui.spawn(Node {
                width: percent(100),
                ..row(CARD_GAP)
            })
            .with_children(|ui| {
                for &kind in chunk {
                    hand_card(ui, art, session, kind, my_turn_to_place);
                }
            });
        }
    });
    card_info(ui, session);

    recent_story(ui, session, art);
    if let Some(round) = session.log_window {
        log_window(ui, session, art, round);
    }
}

/// A place in the drawn queue.
#[derive(Clone, Copy)]
enum QueueItem {
    /// A "place here" slot for the picked hand card.
    Place(Slot),
    /// The stack at this queue position.
    Card(usize),
}

/// The top card of the stack at queue position `pos`, clickable when it is a
/// legal choice.
fn queue_card(
    ui: &mut ChildSpawnerCommands,
    session: &Session,
    art: &CardArt,
    legal: &[Action],
    pos: usize,
    pointed: Option<usize>,
    can_place: &dyn Fn(Slot) -> bool,
) {
    let scene = session.scene.as_ref();
    let queue = scene.map_or(&session.state.queue, |scene| &scene.queue);
    let Some(stack) = queue.get(pos) else {
        return;
    };
    let Some(top) = stack.last() else {
        return;
    };
    let click = if can_place(Slot::OnStack(pos)) {
        Some(Click::Slot(Slot::OnStack(pos)))
    } else if legal.contains(&Action::Target(pos)) {
        Some(Click::Do(Action::Target(pos)))
    } else if session.move_from.is_none()
        && legal
            .iter()
            .any(|a| matches!(a, Action::Move { from, .. } if *from == pos))
    {
        Some(Click::MoveFrom(pos))
    } else {
        None
    };
    let mine = top.card.owner == HUMAN;
    let (caption, note) = if top.face_up {
        (card_name(top.card.kind), String::new())
    } else if mine {
        (card_name(top.card.kind), t!("hidden"))
    } else {
        (t!("face-down"), String::new())
    };
    card(
        ui,
        art,
        CardLook {
            owner: top.card.owner,
            kind: (top.face_up || mine).then_some(top.card.kind),
            secret: mine && !top.face_up,
            influence: top.influence,
            depth: stack.len(),
            highlight: click.is_some() || pointed == Some(pos) || session.move_from == Some(pos),
            click,
            caption,
            note: Some(note),
            arrow: Some(pointed == Some(pos)),
            mark: scene.and_then(|scene| scene.marks.iter().find(|mark| mark.pos == pos)),
        },
    );
}

/// A card in the human's hand, clickable while they place a card.
fn hand_card(
    ui: &mut ChildSpawnerCommands,
    art: &CardArt,
    session: &Session,
    kind: CardKind,
    clickable: bool,
) {
    card(
        ui,
        art,
        CardLook {
            owner: HUMAN,
            kind: Some(kind),
            secret: false,
            influence: 0,
            depth: 1,
            click: clickable.then_some(Click::Hand(kind)),
            highlight: session.selected == Some(kind),
            caption: card_name(kind),
            note: None,
            arrow: None,
            mark: None,
        },
    );
}

/// The latest part of the story, on a strip of parchment: the last
/// paragraphs of the latest round that has one. A click opens the whole
/// story.
fn recent_story(ui: &mut ChildSpawnerCommands, session: &Session, art: &CardArt) {
    let story = session
        .log
        .iter()
        .rev()
        .find(|story| !story.lines.is_empty());
    let ink = Ink {
        size: LOG_TEXT,
        text: PAGE_INK,
        name: CRIMSON,
        font: Some(&art.story_font),
    };
    ui.spawn((
        Button,
        Hovered::default(),
        Click::Log,
        RestBorder(INK),
        Node {
            width: px(LOG_W),
            max_width: percent(100),
            height: px(LOG_PANEL_H),
            padding: UiRect::axes(px(18), px(8)),
            border: UiRect::all(px(3)),
            border_radius: BorderRadius::all(px(10)),
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            ..default()
        },
        BackgroundColor(PAGE),
        BorderColor::all(INK),
    ))
    .with_children(|ui| {
        let Some(story) = story else {
            return;
        };
        ui.spawn(ink.words(
            t!("log-round-tab", round = story.round),
            CRIMSON,
            LOG_TEXT + 2.0,
        ));
        // The newest sentences sit at the bottom; older ones leave at the top.
        ui.spawn(Node {
            width: percent(100),
            flex_grow: 1.0,
            min_height: px(0),
            overflow: Overflow::clip(),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexEnd,
            row_gap: px(4),
            ..default()
        })
        .with_children(|ui| {
            let paragraphs = story.paragraphs();
            let start = paragraphs.len().saturating_sub(2);
            for sentences in paragraphs.iter().skip(start) {
                paragraph(ui, art, sentences, ink, false);
            }
        });
    });
}

/// The story of one round over the whole screen, as a page of a book: the
/// round's title, a paragraph for the placements and one for every card's
/// turn, and what everyone has at the end. Tabs switch between rounds; the
/// game waits while it is open.
fn log_window(ui: &mut ChildSpawnerCommands, session: &Session, art: &CardArt, round: u8) {
    let story = session.log.iter().find(|story| story.round == round);
    let latest = session.log.last().is_some_and(|last| last.round == round);
    let running = session.state.current_player().is_some();
    ui.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            right: px(0),
            top: px(0),
            bottom: px(0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            // The top bar stays over the window; start below it.
            padding: UiRect::new(px(16), px(16), px(72), px(16)),
            row_gap: px(10),
            ..default()
        },
        BackgroundColor(FELT),
        // Over the board, under the menu button and the drop-downs.
        GlobalZIndex(5),
    ))
    .with_children(|ui| {
        ui.spawn(text(t!("log-title"), 30.0, GOLD));
        if running {
            ui.spawn(text(t!("log-paused"), 16.0, DIM));
        }
        ui.spawn(wide_row(8.0)).with_children(|ui| {
            for tab in &session.log {
                small_button(
                    ui,
                    &t!("log-round-tab", round = tab.round),
                    Click::LogRound(tab.round),
                    tab.round == round,
                );
            }
        });
        let Some(story) = story else {
            return;
        };
        ui.spawn((
            LogScroll,
            Node {
                width: px(LOG_WINDOW_W),
                max_width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(40), px(24)),
                row_gap: px(14),
                border: UiRect::all(px(3)),
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            // The latest round opens at its newest events; the layout keeps
            // the position within the content.
            ScrollPosition(Vec2::new(0.0, if latest { 1.0e6 } else { 0.0 })),
            BackgroundColor(PAGE),
            BorderColor::all(INK),
        ))
        .with_children(|ui| story_page(ui, art, story, !(latest && running)));
        button(ui, &t!("log-close"), Click::Log, true);
    });
}

/// The page of one round's story. `finished` when the round is over.
fn story_page(ui: &mut ChildSpawnerCommands, art: &CardArt, story: &RoundLog, finished: bool) {
    let ink = Ink {
        size: LOG_WINDOW_TEXT,
        text: PAGE_INK,
        name: CRIMSON,
        font: Some(&art.story_font),
    };
    ui.spawn((
        ink.words(t!("log-round-tab", round = story.round), CRIMSON, 34.0),
        Node {
            align_self: AlignSelf::Center,
            ..default()
        },
    ));
    ornament(ui);
    let paragraphs = story.paragraphs();
    if paragraphs.is_empty() {
        ui.spawn(ink.words(t!("log-empty"), PAGE_DIM, LOG_WINDOW_TEXT));
    }
    for sentences in &paragraphs {
        paragraph(ui, art, sentences, ink, true);
    }
    if let Some(standing) = story.standing() {
        ornament(ui);
        tally(ui, story, standing, finished, ink);
    }
}

/// A thin line with a diamond in the middle, between parts of a page.
fn ornament(ui: &mut ChildSpawnerCommands) {
    let line = || {
        (
            Node {
                width: px(90),
                height: px(2),
                ..default()
            },
            BackgroundColor(CRIMSON),
        )
    };
    ui.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        align_self: AlignSelf::Center,
        column_gap: px(8),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|ui| {
        ui.spawn(line());
        ui.spawn((
            Node {
                width: px(9),
                height: px(9),
                ..default()
            },
            BackgroundColor(CRIMSON),
            UiTransform {
                rotation: Rot2::degrees(45.0),
                ..UiTransform::IDENTITY
            },
        ));
        ui.spawn(line());
    });
}

/// Everyone's influence at the end of the round, or so far, with what they
/// won or lost in it.
fn tally(
    ui: &mut ChildSpawnerCommands,
    story: &RoundLog,
    standing: &[u32],
    finished: bool,
    ink: Ink,
) {
    let totals = story.totals(standing.len());
    ui.spawn(Node {
        width: percent(100),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        flex_shrink: 0.0,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        column_gap: px(18),
        row_gap: px(6),
        ..default()
    })
    .with_children(|ui| {
        let label = if finished {
            t!("story-tally-final")
        } else {
            t!("story-tally-now")
        };
        ui.spawn(ink.words(label, PAGE_DIM, ink.size));
        for (player, (&total, &change)) in standing.iter().zip(&totals).enumerate() {
            let change = if change > 0 {
                format!("+{change}")
            } else {
                change.to_string()
            };
            ui.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            })
            .with_children(|ui| {
                ui.spawn((
                    Node {
                        width: px(12),
                        height: px(12),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(family_color(player)),
                ));
                ui.spawn(ink.words(
                    t!(
                        "story-tally-item",
                        player = player_name(player),
                        total = total,
                        change = change
                    ),
                    ink.text,
                    ink.size,
                ));
            });
        }
    });
}

/// The scrolling page of the log window.
#[derive(Component)]
pub struct LogScroll;

/// Scrolls the log window with the mouse wheel.
pub fn scroll_log(
    mut wheel: MessageReader<MouseWheel>,
    mut lists: Query<(&mut ScrollPosition, &ComputedNode), With<LogScroll>>,
) {
    let delta: f32 = wheel
        .read()
        .map(|event| match event.unit {
            MouseScrollUnit::Line => event.y * 3.0 * LOG_WINDOW_TEXT,
            MouseScrollUnit::Pixel => event.y,
        })
        .sum();
    if delta == 0.0 {
        return;
    }
    for (mut position, node) in &mut lists {
        let max = ((node.content_size().y - node.size().y) * node.inverse_scale_factor()).max(0.0);
        position.y = (position.y.min(max) - delta).clamp(0.0, max);
    }
}

/// How the words of the story are drawn.
#[derive(Clone, Copy)]
struct Ink<'a> {
    size: f32,
    /// Colour of the words.
    text: Color,
    /// Colour of card names.
    name: Color,
    /// The story font, or `None` for the interface font.
    font: Option<&'a Handle<Font>>,
}

impl Ink<'_> {
    /// Text in this ink's font, in `color` and `size`.
    fn words(&self, content: impl Into<String>, color: Color, size: f32) -> impl Bundle {
        let mut font = TextFont {
            font_size: FontSize::Px(size),
            ..default()
        };
        if let Some(handle) = self.font {
            font.font = handle.clone().into();
        }
        (Text::new(content), font, TextColor(color))
    }
}

/// A paragraph of the story, flowing like a page of a book: every word is
/// its own node, so that lines break between words and around the card
/// miniatures. `indent` starts the first line a little to the right.
fn paragraph(
    ui: &mut ChildSpawnerCommands,
    art: &CardArt,
    sentences: &[Sentence],
    ink: Ink,
    indent: bool,
) {
    ui.spawn(Node {
        width: percent(100),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        flex_shrink: 0.0,
        align_items: AlignItems::Center,
        column_gap: px(ink.size * 0.3),
        row_gap: px(ink.size * 0.3),
        ..default()
    })
    .with_children(|ui| {
        if indent {
            ui.spawn(Node {
                width: px(ink.size),
                ..default()
            });
        }
        for sentence in sentences {
            match sentence.line {
                LogLine::Entry(entry) => {
                    for piece in entry.story(sentence.lead) {
                        match piece {
                            Piece::Text(words) => {
                                for word in words.split_whitespace() {
                                    ui.spawn(ink.words(word, ink.text, ink.size));
                                }
                            }
                            Piece::Card {
                                card,
                                prefix,
                                suffix,
                            } => story_card(ui, art, card, &prefix, &suffix, ink),
                        }
                    }
                }
                LogLine::Error(error) => {
                    let line = t!("log-error", error = error.to_string());
                    for word in line.split_whitespace() {
                        ui.spawn(ink.words(word, RED, ink.size));
                    }
                }
            }
        }
    });
}

/// An event as one sentence with card miniatures, followed by the influence
/// each player won or lost: the headline while an action is shown.
fn story_line(ui: &mut ChildSpawnerCommands, art: &CardArt, entry: &Entry) {
    let ink = Ink {
        size: 22.0,
        text: PAPER,
        name: GOLD,
        font: None,
    };
    ui.spawn(Node {
        width: percent(100),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        flex_shrink: 0.0,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        column_gap: px(ink.size * 0.4),
        row_gap: px(3),
        ..default()
    })
    .with_children(|ui| {
        for piece in entry.story(true) {
            match piece {
                Piece::Text(words) => {
                    let words = words.trim();
                    if !words.is_empty() {
                        ui.spawn(ink.words(words, ink.text, ink.size));
                    }
                }
                Piece::Card {
                    card,
                    prefix,
                    suffix,
                } => story_card(ui, art, card, &prefix, &suffix, ink),
            }
        }
        for &(player, change) in &entry.changes {
            change_chip(ui, player, change, ink.size);
        }
    });
}

/// A card named in the story: its miniature framed in the owner's colour,
/// then its name, with the punctuation that touches it. Kept on one line.
fn story_card(
    ui: &mut ChildSpawnerCommands,
    art: &CardArt,
    card: Option<Seen>,
    prefix: &str,
    suffix: &str,
    ink: Ink,
) {
    let kind = card.and_then(|card| card.kind);
    let name = kind.map_or_else(|| t!("log-hidden-card"), card_name);
    let frame = card.map_or(DIM, |card| family_color(card.owner));
    let edge = (ink.size * 1.4).round();
    ui.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(|ui| {
        if !prefix.is_empty() {
            ui.spawn(ink.words(prefix, ink.text, ink.size));
        }
        ui.spawn((
            Node {
                width: px(edge),
                height: px(edge),
                margin: UiRect::horizontal(px(3)),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(edge * 0.3)),
                ..default()
            },
            BackgroundColor(frame),
            BorderColor::all(frame),
            children![(
                ImageNode::new(art.mini(kind)),
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
            )],
        ));
        // A card name stands out; the words for a hidden card do not.
        let color = if kind.is_some() { ink.name } else { ink.text };
        ui.spawn(ink.words(name, color, ink.size));
        if !suffix.is_empty() {
            ui.spawn(ink.words(suffix, ink.text, ink.size));
        }
    });
}

/// A player's influence change, framed in the player's colour.
fn change_chip(ui: &mut ChildSpawnerCommands, player: PlayerId, change: i64, size: f32) {
    let amount = if change > 0 {
        format!("+{change}")
    } else {
        change.to_string()
    };
    let ink = match change {
        1.. => GOLD,
        0 => DIM,
        _ => LOSS,
    };
    ui.spawn((
        Node {
            padding: UiRect::axes(px(7), px(1)),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(8)),
            ..default()
        },
        BackgroundColor(INK),
        BorderColor::all(family_color(player)),
        children![text(
            t!("log-change", player = player_name(player), amount = amount),
            size - 1.0,
            ink
        )],
    ));
}

/// The card whose ability the panel under the hand explains, with the title
/// to show: the hand card the human picked, or the human's card waiting for a
/// decision.
fn focused_card(session: &Session) -> Option<(String, CardKind)> {
    if let Some(kind) = session.selected {
        return Some((card_name(kind), kind));
    }
    if !session.human_turn() {
        return None;
    }
    let Phase::Resolution { card, decision } = session.state.phase else {
        return None;
    };
    let kind = match decision {
        Decision::RevealOrNot => card.kind,
        Decision::Ability(kind) => kind,
    };
    let title = if kind == card.kind {
        card_name(kind)
    } else {
        // A Shapeshifter using the ability it copies.
        t!(
            "copying",
            card = card_name(card.kind),
            copied = card_name(kind)
        )
    };
    Some((title, kind))
}

/// The panel under the hand that explains what the picked card does. It keeps
/// its size when empty, so the board does not jump when a card is picked.
fn card_info(ui: &mut ChildSpawnerCommands, session: &Session) {
    let focus = focused_card(session);
    let may_pick = session.human_turn() && matches!(session.state.phase, Phase::Placement { .. });
    ui.spawn((
        Node {
            width: px(INFO_W),
            max_width: percent(100),
            min_height: px(INFO_H),
            padding: UiRect::axes(px(16), px(8)),
            border: UiRect::all(px(3)),
            border_radius: BorderRadius::all(px(10)),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            row_gap: px(4),
            ..default()
        },
        BackgroundColor(PANEL),
        BorderColor::all(if focus.is_some() { GOLD } else { PANEL }),
    ))
    .with_children(|ui| match focus {
        Some((title, kind)) => {
            ui.spawn(text(title, 20.0, GOLD));
            ui.spawn(text(card_description(kind), 18.0, PAPER));
        }
        None if may_pick => {
            ui.spawn(text(t!("pick-to-see"), 18.0, DIM));
        }
        None => {}
    });
}

/// Royal Decree: a button in the gap before queue position `gap` when the
/// picked card may be moved there.
fn move_gap(ui: &mut ChildSpawnerCommands, session: &Session, legal: &[Action], gap: usize) {
    let Some(from) = session.move_from else {
        return;
    };
    let lone = session
        .state
        .queue
        .get(from)
        .is_some_and(|stack| stack.len() == 1);
    // `to` counts positions after the card is taken out of the queue.
    let to = if lone && gap > from {
        gap.saturating_sub(1)
    } else {
        gap
    };
    let action = Action::Move { from, to };
    // A lone card has the same destination on both of its sides; offer it once.
    let duplicate = lone && gap == from.saturating_add(1);
    if legal.contains(&action) && !duplicate {
        move_slot(ui, Click::Do(action));
    }
}

/// A narrow gold slot between two queue cards where the Royal Decree puts
/// the picked card. Its label is written upwards, so a row of cards with
/// these slots between them stays narrow.
fn move_slot(ui: &mut ChildSpawnerCommands, click: Click) {
    ui.spawn(column(3.0)).with_children(|ui| {
        // Same space as the arrow above a card, to stay level with the cards.
        ui.spawn(Node {
            width: px(ARROW_W),
            height: px(ARROW_H),
            ..default()
        });
        ui.spawn((
            Button,
            Hovered::default(),
            click,
            RestBorder(INK),
            Node {
                width: px(MOVE_SLOT_W),
                height: px(CARD_H + 10.0),
                border: UiRect::all(px(3)),
                border_radius: BorderRadius::all(px(10)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::visible(),
                ..default()
            },
            BackgroundColor(GOLD),
            BorderColor::all(INK),
        ))
        .with_children(|ui| {
            ui.spawn((
                text(t!("move-here"), 20.0, INK),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
                UiTransform {
                    rotation: Rot2::degrees(-90.0),
                    ..UiTransform::IDENTITY
                },
            ));
        });
    });
}

fn game_over(ui: &mut ChildSpawnerCommands, session: &Session, art: &CardArt) {
    let state = &session.state;
    let winners = state.winners();
    let headline = if winners.contains(&HUMAN) {
        if winners.len() == 1 {
            t!("you-win")
        } else {
            t!("you-share-win")
        }
    } else {
        let names: Vec<String> = winners.iter().map(|&p| player_name(p)).collect();
        t!("winners", names = names.join(&t!("and")))
    };
    ui.spawn(Node {
        height: px(20),
        ..default()
    });
    ui.spawn(centered(headline, 64.0, GOLD));

    let mut ranking: Vec<(PlayerId, u32)> = state.scores().into_iter().enumerate().collect();
    ranking.sort_by_key(|&(_, score)| std::cmp::Reverse(score));
    ui.spawn(column(8.0)).with_children(|ui| {
        for (player, score) in ranking {
            ui.spawn((
                Node {
                    width: px(420),
                    padding: UiRect::axes(px(16), px(8)),
                    border: UiRect::all(px(4)),
                    border_radius: BorderRadius::all(px(10)),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderColor::all(family_color(player)),
                children![
                    text(player_name(player), 26.0, PAPER),
                    text(t!("influence", amount = score), 26.0, GOLD),
                ],
            ));
        }
    });
    ui.spawn(centered(t!("tie-break"), 16.0, DIM));
    ui.spawn(wide_row(10.0)).with_children(|ui| {
        button(ui, &t!("log-open"), Click::Log, false);
        button(ui, &t!("back-to-menu"), Click::Menu, true);
    });
    if let Some(round) = session.log_window {
        log_window(ui, session, art, round);
    }
}
