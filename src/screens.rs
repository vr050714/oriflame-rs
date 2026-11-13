//! User interface: menu, board and game-over screens.
//!
//! The interface is rebuilt from the game state whenever [`UiDirty`] is set.
//! Every clickable node carries a [`Click`] that says what it does.

use crate::{
    Screen,
    game::{BotTurn, HUMAN, Mark, Role, Session, Settings, player_name},
};
use bevy::{
    asset::RenderAssetUsages,
    ecs::hierarchy::ChildSpawnerCommands,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{Activate, Button},
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
/// Log lines shown on the board.
const LOG_LINES: usize = 7;

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

/// Card images, decoded once and kept for the whole run.
///
/// The PNG files are compiled into the executable, so the game runs from any
/// folder without an `assets` directory beside it.
#[derive(Resource)]
pub struct CardArt {
    faces: Vec<(CardKind, Handle<Image>)>,
    back: Handle<Image>,
    /// Spear pointing in the direction the queue is resolved.
    spear: Handle<Image>,
    /// Arrow pointing down at a card.
    arrow: Handle<Image>,
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
        }
    }
}

impl CardArt {
    /// The face of `kind`, or the card back for `None`.
    fn image(&self, kind: Option<CardKind>) -> Handle<Image> {
        kind.and_then(|kind| self.faces.iter().find(|(k, _)| *k == kind))
            .map_or_else(|| self.back.clone(), |(_, handle)| handle.clone())
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
    for (motion, mut clock, mut transform) in &mut nodes {
        clock.0 += time.delta_secs();
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
) {
    let Ok(&click) = clicks.get(event.entity) else {
        return;
    };
    dirty.0 = true;
    match click {
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
pub fn rebuild_ui(
    mut commands: Commands,
    mut dirty: ResMut<UiDirty>,
    screen: Res<State<Screen>>,
    roots: Query<Entity, With<UiRoot>>,
    settings: Res<Settings>,
    session: Option<Res<Session>>,
    art: Res<CardArt>,
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
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(px(12)),
            row_gap: px(10),
            ..default()
        },
        BackgroundColor(FELT),
    ));
    root.with_children(|ui| match (screen.get(), session.as_deref()) {
        (Screen::Playing, Some(session)) => board(ui, session, &art),
        (Screen::GameOver, Some(session)) => game_over(ui, session),
        _ => menu(ui, &settings, &art),
    });
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
    note: Option<&'static str>,
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
    let border = match look.mark.map(|mark| mark.role) {
        Some(Role::Target) => RED,
        Some(Role::Actor) => GOLD,
        None if look.highlight => GOLD,
        None => family_color(look.owner),
    };

    ui.spawn(column(3.0)).with_children(|ui| {
        if let Some(pointing) = look.arrow {
            arrow_space(ui, art, pointing);
        }
        let mut frame = ui.spawn((
            Node {
                width: px(CARD_W),
                height: px(CARD_H),
                border: UiRect::all(px(5)),
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            BackgroundColor(family_color(look.owner)),
            BorderColor::all(border),
        ));
        if let Some(click) = look.click {
            frame.insert((Button, Hovered::default(), click, RestBorder(border)));
        }
        if let Some(mark) = look.mark {
            frame.insert(mark.motion);
            if mark.motion == Motion::Arrive {
                // Start small, so the card is not drawn full size for one frame.
                frame.insert(UiTransform {
                    scale: Vec2::splat(0.4),
                    ..UiTransform::IDENTITY
                });
            }
        }
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
                effect_label(ui, label, mark.role);
            }
        });
        ui.spawn(text(look.caption, 14.0, PAPER));
        if let Some(note) = look.note {
            // An empty text node has no height, so a blank line needs a space.
            ui.spawn(text(if note.is_empty() { " " } else { note }, 12.0, DIM));
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
            children![text("Place", 18.0, GOLD), text("here", 18.0, GOLD)],
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
    ui.spawn(Node {
        height: px(30),
        ..default()
    });
    ui.spawn(text("Oriflamme", 72.0, GOLD));
    ui.spawn(text(
        "Intrigue for the crown, against the computer",
        22.0,
        DIM,
    ));

    ui.spawn(row(10.0)).with_children(|ui| {
        for kind in [
            CardKind::Lord,
            CardKind::Heir,
            CardKind::Spy,
            CardKind::Archer,
            CardKind::Ambush,
        ] {
            ui.spawn((
                ImageNode::new(art.image(Some(kind))),
                Node {
                    width: px(CARD_W),
                    height: px(CARD_H),
                    ..default()
                },
            ));
        }
    });

    ui.spawn(text("Players", 24.0, PAPER));
    ui.spawn(row(10.0)).with_children(|ui| {
        for count in MIN_PLAYERS..=MAX_PLAYERS {
            let label = format!("{count}  (you + {} PC)", count.saturating_sub(1));
            button(ui, &label, Click::Players(count), settings.players == count);
        }
    });

    ui.spawn(text("Computer strength", 24.0, PAPER));
    ui.spawn(row(10.0)).with_children(|ui| {
        for level in Difficulty::ALL {
            button(
                ui,
                level.name(),
                Click::Level(level),
                settings.difficulty == level,
            );
        }
    });

    ui.spawn(Node {
        height: px(10),
        ..default()
    });
    button(ui, "Start game", Click::Start, true);
}

/// What the human should do now, or who is moving.
fn prompt(session: &Session) -> String {
    let state = &session.state;
    let Some(player) = state.current_player() else {
        return "The game is over.".to_owned();
    };
    if player != HUMAN {
        return format!("{} is thinking...", player_name(player));
    }
    match state.phase {
        Phase::Placement { .. } => match session.selected {
            None => "Your turn: pick a card from your hand.".to_owned(),
            Some(kind) => format!("Pick where to place your {kind}. It goes face down."),
        },
        Phase::Resolution {
            card,
            decision: Decision::RevealOrNot,
        } => format!(
            "Your {} (marked): reveal it, or keep it hidden and add 1 influence to it?",
            card.kind
        ),
        Phase::Resolution {
            decision: Decision::Ability(CardKind::RoyalDecree),
            ..
        } => match session.move_from {
            None => "Royal Decree: pick the card to move.".to_owned(),
            Some(_) => "Royal Decree: pick the new place for the card.".to_owned(),
        },
        Phase::Resolution {
            decision: Decision::Ability(kind),
            ..
        } => format!("{kind}: pick a highlighted card."),
        Phase::GameOver => "The game is over.".to_owned(),
    }
}

fn board(ui: &mut ChildSpawnerCommands, session: &Session, art: &CardArt) {
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

    ui.spawn(row(24.0)).with_children(|ui| {
        ui.spawn(text(
            format!("Round {} of {ROUNDS}", state.round),
            26.0,
            GOLD,
        ));
        button(ui, "Menu", Click::Menu, false);
    });

    // Scores.
    ui.spawn(row(10.0)).with_children(|ui| {
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
                    text(format!("{influence} influence"), 20.0, GOLD),
                    text(format!("{} in hand", info.hand.len()), 16.0, DIM),
                ],
            ));
        }
    });

    let headline = scene.map_or_else(|| prompt(session), |scene| scene.text.clone());
    ui.spawn(text(headline, 22.0, PAPER));

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
        ui.spawn(Node {
            min_height: px(200),
            ..row(8.0)
        })
        .with_children(|ui| {
            if queue.is_empty() && !can_place(Slot::Left) {
                ui.spawn(text("The queue is empty.", 20.0, DIM));
            }
            if can_place(Slot::Left) {
                place_slot(ui, Slot::Left);
            }
            for (pos, stack) in queue.iter().enumerate() {
                move_gap(ui, session, &legal, pos);
                let Some(top) = stack.last() else {
                    continue;
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
                    (top.card.kind.name(), "")
                } else if mine {
                    (top.card.kind.name(), "(hidden)")
                } else {
                    ("face down", "")
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
                        highlight: click.is_some()
                            || pointed == Some(pos)
                            || session.move_from == Some(pos),
                        click,
                        caption: caption.to_owned(),
                        note: Some(note),
                        arrow: Some(pointed == Some(pos)),
                        mark: scene
                            .and_then(|scene| scene.marks.iter().find(|mark| mark.pos == pos)),
                    },
                );
            }
            move_gap(ui, session, &legal, queue.len());
            if can_place(Slot::Right) {
                place_slot(ui, Slot::Right);
            }
        });
    });

    // Choices that are not a card on the table.
    ui.spawn(row(10.0)).with_children(|ui| {
        if legal.contains(&Action::Reveal) {
            button(ui, "Reveal", Click::Do(Action::Reveal), true);
            button(ui, "Keep hidden (+1)", Click::Do(Action::KeepHidden), false);
        }
        if session.move_from.is_some() {
            button(ui, "Pick another card", Click::CancelMove, false);
        }
    });

    // The human's hand.
    ui.spawn(text("Your hand", 18.0, DIM));
    ui.spawn(row(8.0)).with_children(|ui| {
        let my_turn_to_place =
            session.human_turn() && matches!(state.phase, Phase::Placement { .. });
        for &kind in state.players.get(HUMAN).map_or(&[][..], |p| &p.hand) {
            card(
                ui,
                art,
                CardLook {
                    owner: HUMAN,
                    kind: Some(kind),
                    secret: false,
                    influence: 0,
                    depth: 1,
                    click: my_turn_to_place.then_some(Click::Hand(kind)),
                    highlight: session.selected == Some(kind),
                    caption: kind.name().to_owned(),
                    note: None,
                    arrow: None,
                    mark: None,
                },
            );
        }
    });
    card_info(ui, session);

    // Recent events, newest last.
    ui.spawn(column(2.0)).with_children(|ui| {
        let start = session.log.len().saturating_sub(LOG_LINES);
        for line in session.log.iter().skip(start) {
            ui.spawn(text(line.clone(), 15.0, DIM));
        }
    });
}

/// The card whose ability the panel under the hand explains, with the title
/// to show: the hand card the human picked, or the human's card waiting for a
/// decision.
fn focused_card(session: &Session) -> Option<(String, CardKind)> {
    if let Some(kind) = session.selected {
        return Some((kind.name().to_owned(), kind));
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
        kind.name().to_owned()
    } else {
        // A Shapeshifter using the ability it copies.
        format!("{} (copying {kind})", card.kind)
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
            ui.spawn(text(kind.description(), 18.0, PAPER));
        }
        None if may_pick => {
            ui.spawn(text(
                "Pick a card from your hand to see what it does.",
                18.0,
                DIM,
            ));
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
        button(ui, "Here", Click::Do(action), true);
    }
}

fn game_over(ui: &mut ChildSpawnerCommands, session: &Session) {
    let state = &session.state;
    let winners = state.winners();
    let headline = if winners.contains(&HUMAN) {
        if winners.len() == 1 {
            "You win!"
        } else {
            "You share the win!"
        }
        .to_owned()
    } else {
        let names: Vec<String> = winners.iter().map(|&p| player_name(p)).collect();
        format!("{} won", names.join(" and "))
    };
    ui.spawn(Node {
        height: px(60),
        ..default()
    });
    ui.spawn(text(headline, 64.0, GOLD));

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
                    text(format!("{score} influence"), 26.0, GOLD),
                ],
            ));
        }
    });
    ui.spawn(text(
        "Ties are broken by the number of cards left in the queue.",
        16.0,
        DIM,
    ));
    button(ui, "Back to menu", Click::Menu, true);
}
