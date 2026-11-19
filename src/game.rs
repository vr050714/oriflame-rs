//! The running game, the step-by-step playback of each action, and the
//! system that lets computer players move.

use crate::{
    Screen,
    screens::{Motion, UiDirty},
    t,
};
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures::check_ready},
};
use oriflamme_ai::{Difficulty, make_strategy};
use oriflamme_core::{
    Action, Card, CardKind, Event, GameState, MIN_PLAYERS, PlayerId, QueueCard, Rng64, RuleError,
    SetupError, Slot, Snapshot, Stack, Step,
};
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicU8, Ordering},
};
use web_time::{SystemTime, UNIX_EPOCH};

/// The human always sits in seat 0.
pub const HUMAN: PlayerId = 0;

/// Seconds a computer player waits before it starts to think.
const BOT_DELAY: f32 = 0.5;

/// How the playback of the resolution goes on from one card to the next.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stepping {
    /// Every scene plays one after another.
    #[default]
    Auto,
    /// The playback stops before each card acts in the resolution, showing what it is about to
    /// do, until the human presses the continue button.
    Manual,
}

impl Stepping {
    /// Every mode, in the order of the stepping menu.
    pub const ALL: [Stepping; 2] = [Stepping::Auto, Stepping::Manual];

    /// Key of the mode's name in the locale files.
    pub fn key(self) -> &'static str {
        match self {
            Stepping::Auto => "stepping-auto",
            Stepping::Manual => "stepping-manual",
        }
    }
}

/// The stepping mode in use, as an index into [`Stepping::ALL`].
static STEPPING: AtomicU8 = AtomicU8::new(0);

/// Switches the stepping mode. Takes effect at once: switching to
/// [`Stepping::Auto`] lets a paused playback go on.
pub fn set_stepping(mode: Stepping) {
    let index = Stepping::ALL.iter().position(|&s| s == mode).unwrap_or(0);
    STEPPING.store(u8::try_from(index).unwrap_or(0), Ordering::Relaxed);
}

/// The stepping mode in use.
pub fn stepping() -> Stepping {
    let index = usize::from(STEPPING.load(Ordering::Relaxed));
    Stepping::ALL.get(index).copied().unwrap_or_default()
}

/// Choices made on the menu screen.
#[derive(Resource, Clone, Copy)]
pub struct Settings {
    pub players: usize,
    pub difficulty: Difficulty,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            players: MIN_PLAYERS,
            difficulty: Difficulty::Medium,
        }
    }
}

/// Part a card plays in a scene.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The card that acts.
    Actor,
    /// The card that is acted upon.
    Target,
}

/// Short text shown on a card in a scene.
#[derive(Clone, Copy)]
pub enum Label {
    Gain(u32),
    Loss(u32),
    Copied,
    Eliminated,
    /// An Ambush that gained this much.
    Ambush(u32),
    Discarded,
    Moves,
}

impl Label {
    /// The text in the current language.
    pub fn text(self) -> String {
        match self {
            Label::Gain(amount) => format!("+{amount}"),
            Label::Loss(amount) => format!("-{amount}"),
            Label::Copied => t!("mark-copied"),
            Label::Eliminated => t!("mark-eliminated"),
            Label::Ambush(gained) => t!("mark-ambush", gained = gained),
            Label::Discarded => t!("mark-discarded"),
            Label::Moves => t!("mark-moves"),
        }
    }
}

/// A card singled out in a scene.
pub struct Mark {
    pub pos: usize,
    pub role: Role,
    pub label: Option<Label>,
    pub motion: Motion,
}

impl Mark {
    fn new(pos: usize, role: Role, label: Option<Label>, motion: Motion) -> Self {
        Self {
            pos,
            role,
            label,
            motion,
        }
    }
}

/// One step of an action as it is shown to the player: a frozen table, the
/// cards involved, and the story entry of the event, shown as the headline.
pub struct Scene {
    pub queue: Vec<Stack>,
    pub influence: Vec<u32>,
    pub entry: Entry,
    pub marks: Vec<Mark>,
    /// False for the second scene of an event shown in two, so that the
    /// event is logged once.
    first: bool,
    /// True for the first scene of an action the human chose: the human has
    /// just decided it, so manual stepping does not stop before it.
    chosen: bool,
    seconds: f32,
}

impl Scene {
    fn new(table: Snapshot, entry: &Entry, marks: Vec<Mark>, seconds: f32) -> Self {
        Self {
            queue: table.queue,
            influence: table.influence,
            entry: entry.clone(),
            marks,
            first: true,
            chosen: false,
            seconds,
        }
    }
}

/// A card as the human knows it: the kind of an opponent's face-down card is
/// unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seen {
    pub owner: PlayerId,
    pub kind: Option<CardKind>,
}

impl Seen {
    /// A card whose identity is public.
    fn open(card: Card) -> Self {
        Self {
            owner: card.owner,
            kind: Some(card.kind),
        }
    }

    /// A card in the queue, as the human sees it.
    fn in_queue(card: &QueueCard) -> Self {
        let known = card.face_up || card.card.owner == HUMAN;
        Self {
            owner: card.card.owner,
            kind: known.then_some(card.card.kind),
        }
    }
}

/// Text written in place of the acting card and the affected card, so that
/// a translated sentence can be cut where the card miniatures go.
const ACTOR_MARK: char = '\u{E000}';
const TARGET_MARK: char = '\u{E001}';

/// A part of a story sentence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    /// A card, drawn as its miniature and name, between the punctuation that
    /// touches the name in the sentence, such as brackets.
    Card {
        card: Option<Seen>,
        prefix: String,
        suffix: String,
    },
}

/// One event in the story of a round: what happened, which card acted on
/// which, and who won or lost influence. Kept untranslated, so the log
/// follows the language when it is changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub event: Event,
    /// The card that acts, or the card the event is about.
    pub actor: Option<Seen>,
    /// The card acted upon.
    pub target: Option<Seen>,
    /// Influence won (positive) or lost by each player whose influence
    /// changed, in seat order.
    pub changes: Vec<(PlayerId, i64)>,
    /// Every player's influence after the event.
    pub after: Vec<u32>,
}

/// The top card at `pos`, as the human sees it.
fn top_seen(table: &Snapshot, pos: usize) -> Option<Seen> {
    table
        .queue
        .get(pos)
        .and_then(|stack| stack.last())
        .map(Seen::in_queue)
}

impl Entry {
    /// The story entry of one step of an action.
    pub fn from_step(step: &Step) -> Self {
        let Step {
            event,
            before,
            after,
        } = step;
        let (actor, target) = match *event {
            Event::RoundStarted(_) | Event::GameOver => (None, None),
            Event::Placed { slot, .. } => {
                let pos = match slot {
                    Slot::Left => 0,
                    Slot::Right => after.queue.len().saturating_sub(1),
                    Slot::OnStack(pos) => pos,
                };
                let stack = after.queue.get(pos);
                let placed = stack.and_then(|stack| stack.last()).map(Seen::in_queue);
                // The card it now covers.
                let covered = match slot {
                    Slot::OnStack(_) => stack
                        .and_then(|stack| stack.iter().rev().nth(1))
                        .map(Seen::in_queue),
                    Slot::Left | Slot::Right => None,
                };
                (placed, covered)
            }
            Event::KeptHidden { pos, .. } => (top_seen(after, pos), None),
            Event::Revealed { card, .. } | Event::Discarded { card, .. } => {
                (Some(Seen::open(card)), None)
            }
            Event::Gained { pos, .. } => (pos.and_then(|pos| top_seen(after, pos)), None),
            Event::Stole { pos, target, .. } | Event::Copied { pos, target, .. } => (
                pos.and_then(|pos| top_seen(after, pos)),
                top_seen(after, target),
            ),
            Event::Eliminated { by, target, .. } => {
                (Some(Seen::open(by)), Some(Seen::open(target)))
            }
            Event::Ambushed {
                attacker, owner, ..
            } => (
                Some(Seen::open(attacker)),
                Some(Seen::open(Card {
                    owner,
                    kind: CardKind::Ambush,
                })),
            ),
            // Only a Royal Decree moves cards.
            Event::Moved { player, from, .. } => (
                Some(Seen::open(Card {
                    owner: player,
                    kind: CardKind::RoyalDecree,
                })),
                top_seen(before, from),
            ),
        };
        let changes = before
            .influence
            .iter()
            .zip(&after.influence)
            .enumerate()
            .filter_map(|(player, (&was, &now))| {
                let change = i64::from(now).saturating_sub(i64::from(was));
                (change != 0).then_some((player, change))
            })
            .collect();
        Self {
            event: *event,
            actor,
            target,
            changes,
            after: after.influence.clone(),
        }
    }

    /// The event as a sentence of the story in the current language, cut
    /// into text and the cards it mentions. A `lead` sentence opens a
    /// paragraph and names the acting card and its owner; the others go on
    /// telling about the same card.
    pub fn story(&self, lead: bool) -> Vec<Piece> {
        let card = || ACTOR_MARK.to_string();
        let target = || TARGET_MARK.to_string();
        let lead = yes_no(lead);
        let target_owner = self.target.map_or(HUMAN, |seen| seen.owner);
        let sentence = match self.event {
            Event::RoundStarted(round) => t!("story-round", round = round),
            Event::Placed { player, slot } => {
                let key = match slot {
                    Slot::Left => "story-placed-left",
                    Slot::Right => "story-placed-right",
                    Slot::OnStack(_) => "story-placed-stack",
                };
                t!(
                    key,
                    player = player_name(player),
                    player_you = you(player),
                    card = card(),
                    target = target()
                )
            }
            Event::KeptHidden {
                player, influence, ..
            } => t!(
                "story-kept-hidden",
                player = player_name(player),
                player_you = you(player),
                card = card(),
                amount = influence
            ),
            Event::Revealed {
                card: revealed,
                gained,
                ..
            } => t!(
                if gained > 0 {
                    "story-revealed-gain"
                } else {
                    "story-revealed"
                },
                player = player_name(revealed.owner),
                player_you = you(revealed.owner),
                card = card(),
                amount = gained
            ),
            Event::Gained { player, amount, .. } => t!(
                "story-gained",
                lead = lead,
                player = player_name(player),
                player_you = you(player),
                card = card(),
                amount = amount
            ),
            Event::Stole { thief, victim, .. } => t!(
                "story-stole",
                lead = lead,
                player = player_name(thief),
                player_you = you(thief),
                card = card(),
                target = target(),
                victim = player_name(victim),
                victim_you = you(victim)
            ),
            Event::Copied { player, .. } => t!(
                "story-copied",
                lead = lead,
                player = player_name(player),
                player_you = you(player),
                card = card(),
                target = target(),
                owner = player_name(target_owner),
                owner_you = you(target_owner)
            ),
            Event::Eliminated {
                by, target: hit, ..
            } => t!(
                if by == hit {
                    "story-eliminated-self"
                } else {
                    "story-eliminated"
                },
                lead = lead,
                player = player_name(by.owner),
                player_you = you(by.owner),
                card = card(),
                target = target(),
                owner = player_name(hit.owner),
                owner_you = you(hit.owner)
            ),
            Event::Ambushed {
                attacker,
                owner,
                gained,
                ..
            } => {
                // The attacker's owner gains 1 as well.
                let (key, amount) = if owner == attacker.owner {
                    ("story-ambushed-own", gained.saturating_add(1))
                } else {
                    ("story-ambushed", gained)
                };
                t!(
                    key,
                    lead = lead,
                    player = player_name(attacker.owner),
                    player_you = you(attacker.owner),
                    card = card(),
                    target = target(),
                    owner = player_name(owner),
                    owner_you = you(owner),
                    amount = amount
                )
            }
            Event::Discarded { card: gone, .. } => t!(
                "story-discarded",
                lead = lead,
                player = player_name(gone.owner),
                player_you = you(gone.owner),
                card = card()
            ),
            Event::Moved { player, to, .. } => t!(
                "story-moved",
                lead = lead,
                player = player_name(player),
                player_you = you(player),
                card = card(),
                target = target(),
                position = to.saturating_add(1)
            ),
            Event::GameOver => t!("game-is-over"),
        };
        split_story(&sentence, self.actor, self.target)
    }
}

/// A yes-or-no argument for a Fluent selector.
fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

/// Whether `player` is the human, for the messages that address the human
/// differently.
fn you(player: PlayerId) -> &'static str {
    yes_no(player == HUMAN)
}

/// Cuts a sentence at the card marks. Punctuation that touches a mark
/// stays with its card, so that it is not drawn apart from the name.
fn split_story(sentence: &str, actor: Option<Seen>, target: Option<Seen>) -> Vec<Piece> {
    let mut pieces = Vec::new();
    // Text since the last card, the card's suffix included.
    let mut text = String::new();
    let mut card: Option<(Option<Seen>, String)> = None;
    let finish_card =
        |pieces: &mut Vec<Piece>, card: Option<(Option<Seen>, String)>, text: &mut String| {
            if let Some((seen, prefix)) = card {
                let split = text.find(char::is_whitespace).unwrap_or(text.len());
                let rest = text.split_off(split);
                pieces.push(Piece::Card {
                    card: seen,
                    prefix,
                    suffix: std::mem::replace(text, rest),
                });
            }
        };
    for c in sentence.chars() {
        let seen = match c {
            ACTOR_MARK => actor,
            TARGET_MARK => target,
            _ => {
                text.push(c);
                continue;
            }
        };
        finish_card(&mut pieces, card.take(), &mut text);
        let split = text
            .rfind(char::is_whitespace)
            .map_or(0, |at| at.saturating_add(1));
        let prefix = text.split_off(split);
        if !text.is_empty() {
            pieces.push(Piece::Text(std::mem::take(&mut text)));
        }
        card = Some((seen, prefix));
    }
    finish_card(&mut pieces, card, &mut text);
    if !text.is_empty() {
        pieces.push(Piece::Text(text));
    }
    pieces
}

/// One line of the game log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogLine {
    Entry(Entry),
    Error(RuleError),
}

/// The story of one round, oldest event first.
pub struct RoundLog {
    pub round: u8,
    pub lines: Vec<LogLine>,
}

/// What a paragraph of the story is about.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Topic {
    /// The cards put down at the start of the round.
    Placement,
    /// One card's turn in the resolution.
    Turn(Option<Seen>),
}

impl Topic {
    /// What `entry` is about, and whether it starts a new card action of the
    /// resolution after one about `last`: a card revealed or kept hidden, or
    /// another card than before acting. The start of a round and the cards
    /// put down before the resolution are no card actions.
    fn card_action(entry: &Entry, last: Option<Topic>) -> (Option<Topic>, bool) {
        match entry.event {
            Event::RoundStarted(_) | Event::GameOver => (None, false),
            Event::Placed { .. } => (Some(Topic::Placement), false),
            Event::KeptHidden { .. } | Event::Revealed { .. } => {
                (Some(Topic::Turn(entry.actor)), true)
            }
            _ => {
                let this = Some(Topic::Turn(entry.actor));
                (this, last != this)
            }
        }
    }
}

/// A sentence of the story: a log line, and whether it opens its paragraph.
pub struct Sentence<'a> {
    pub line: &'a LogLine,
    pub lead: bool,
}

impl RoundLog {
    fn new(round: u8) -> Self {
        Self {
            round,
            lines: Vec::new(),
        }
    }

    /// The story of the round in paragraphs: first the cards put down, then
    /// one paragraph for every card that takes its turn.
    pub fn paragraphs(&self) -> Vec<Vec<Sentence<'_>>> {
        let mut paragraphs: Vec<Vec<Sentence>> = Vec::new();
        let mut topic = None;
        for line in &self.lines {
            let (this, opens) = match line {
                LogLine::Error(_) => (topic, paragraphs.is_empty()),
                LogLine::Entry(entry) => match entry.event {
                    Event::Placed { .. } => {
                        (Some(Topic::Placement), topic != Some(Topic::Placement))
                    }
                    // A card's turn starts when it is revealed or kept hidden.
                    Event::KeptHidden { .. } | Event::Revealed { .. } => {
                        (Some(Topic::Turn(entry.actor)), true)
                    }
                    _ => {
                        let this = Some(Topic::Turn(entry.actor));
                        (this, topic != this)
                    }
                },
            };
            topic = this;
            if opens {
                paragraphs.push(Vec::new());
            }
            if let Some(paragraph) = paragraphs.last_mut() {
                paragraph.push(Sentence { line, lead: opens });
            }
        }
        paragraphs
    }

    /// Every player's influence after the last event of the round, if
    /// anything has happened yet.
    pub fn standing(&self) -> Option<&[u32]> {
        self.lines.iter().rev().find_map(|line| match line {
            LogLine::Entry(entry) => Some(entry.after.as_slice()),
            LogLine::Error(_) => None,
        })
    }

    /// Influence each of `players` players won or lost in the round.
    pub fn totals(&self, players: usize) -> Vec<i64> {
        let mut totals = vec![0_i64; players];
        let changes = self.lines.iter().flat_map(|line| match line {
            LogLine::Entry(entry) => entry.changes.as_slice(),
            LogLine::Error(_) => &[],
        });
        for &(player, change) in changes {
            if let Some(total) = totals.get_mut(player) {
                *total = total.saturating_add(change);
            }
        }
        totals
    }
}

/// One game in progress.
#[derive(Resource)]
pub struct Session {
    pub state: GameState,
    pub difficulty: Difficulty,
    pub rng: Rng64,
    /// Game log: the story of every round so far, oldest first.
    pub log: Vec<RoundLog>,
    /// The round whose story the log window shows, while it is open. The
    /// game waits while the window is open.
    pub log_window: Option<u8>,
    /// Hand card the human picked and is about to place.
    pub selected: Option<CardKind>,
    /// Royal Decree: the position the human picked to move a card from.
    pub move_from: Option<usize>,
    /// The scene on screen. While one is shown, nobody can move.
    pub scene: Option<Scene>,
    /// Scenes still to be shown.
    scenes: VecDeque<Scene>,
    scene_timer: Timer,
    /// Manual stepping: the scene on screen waits for the continue button
    /// before it plays.
    pub waiting: bool,
    /// What the last scene shown was about, to tell when a new card acts.
    last_topic: Option<Topic>,
}

impl Session {
    /// Starts a game seeded from the clock.
    pub fn new(settings: Settings) -> Result<Self, SetupError> {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0x5EED, |d| u64::from(d.subsec_nanos()) ^ d.as_secs());
        let mut rng = Rng64::new(seed);
        let state = GameState::new(settings.players, &mut rng)?;
        Ok(Self {
            state,
            difficulty: settings.difficulty,
            rng,
            log: vec![RoundLog::new(1)],
            log_window: None,
            selected: None,
            move_from: None,
            scene: None,
            scenes: VecDeque::new(),
            scene_timer: Timer::from_seconds(0.0, TimerMode::Once),
            waiting: false,
            last_topic: None,
        })
    }

    /// True while the last action is still being shown.
    pub fn playing_back(&self) -> bool {
        self.scene.is_some()
    }

    /// True when the human must act.
    pub fn human_turn(&self) -> bool {
        !self.playing_back()
            && self.log_window.is_none()
            && self.state.current_player() == Some(HUMAN)
    }

    /// Adds a line to the story of the current round.
    fn record(&mut self, line: LogLine) {
        if let LogLine::Entry(Entry {
            event: Event::RoundStarted(round),
            ..
        }) = line
        {
            self.log.push(RoundLog::new(round));
            return;
        }
        if self.log.is_empty() {
            self.log.push(RoundLog::new(self.state.round));
        }
        if let Some(round) = self.log.last_mut() {
            round.lines.push(line);
        }
    }

    /// Applies an action for the current player and queues its scenes.
    pub fn apply(&mut self, action: Action) {
        let mover = self.state.current_player();
        match self.state.apply_traced(action) {
            Ok(steps) => {
                let first = self.scenes.len();
                self.scenes.extend(steps.into_iter().flat_map(scenes_for));
                if mover == Some(HUMAN)
                    && let Some(scene) = self.scenes.get_mut(first)
                {
                    scene.chosen = true;
                }
                if self.scene.is_none() {
                    self.next_scene();
                }
            }
            Err(error) => self.record(LogLine::Error(error)),
        }
        self.selected = None;
        self.move_from = None;
    }

    /// Shows the next queued scene, or returns to the live table.
    fn next_scene(&mut self) {
        self.waiting = false;
        self.scene = self.scenes.pop_front();
        let Some(scene) = &self.scene else {
            return;
        };
        self.scene_timer = Timer::from_seconds(scene.seconds, TimerMode::Once);
        // An event shown in two scenes is logged once.
        if scene.first {
            let (topic, starts) = Topic::card_action(&scene.entry, self.last_topic);
            self.last_topic = topic;
            self.waiting = starts && !scene.chosen && stepping() == Stepping::Manual;
            let line = LogLine::Entry(scene.entry.clone());
            self.record(line);
        }
    }

    /// Manual stepping: plays the scene that waits on screen.
    pub fn go_on(&mut self) {
        self.waiting = false;
    }
}

/// Advances the playback of the last action.
pub fn play_scenes(time: Res<Time>, mut session: ResMut<Session>, mut dirty: ResMut<UiDirty>) {
    // The playback waits while the log window is open.
    if session.log_window.is_some() {
        return;
    }
    // A scene waiting for the continue button plays at once when the mode is
    // switched to automatic.
    if session.waiting {
        if stepping() == Stepping::Auto {
            session.go_on();
            dirty.0 = true;
        }
        return;
    }
    if session.playing_back() && session.scene_timer.tick(time.delta()).is_finished() {
        session.next_scene();
        dirty.0 = true;
    }
}

/// Display name of a player.
pub fn player_name(player: PlayerId) -> String {
    if player == HUMAN {
        t!("player-you")
    } else {
        t!("player-pc", number = player)
    }
}

/// The scenes that show one event.
fn scenes_for(step: Step) -> Vec<Scene> {
    let entry = Entry::from_step(&step);
    let entry = &entry;
    let Step {
        event,
        mut before,
        after,
    } = step;
    // Shows the identity of a face-down card that is about to leave the table.
    let mut turn_up = |pos: usize| {
        if let Some(card) = before.queue.get_mut(pos).and_then(|stack| stack.last_mut()) {
            card.face_up = true;
        }
    };
    let actor = |pos: Option<usize>, label: Option<Label>| {
        pos.map(|pos| Mark::new(pos, Role::Actor, label, Motion::Pulse))
    };
    match event {
        Event::RoundStarted(_) => vec![Scene::new(after, entry, Vec::new(), 0.9)],
        Event::GameOver => Vec::new(),
        Event::Placed { slot, .. } => {
            let pos = match slot {
                Slot::Left => 0,
                Slot::Right => after.queue.len().saturating_sub(1),
                Slot::OnStack(pos) => pos,
            };
            let marks = vec![Mark::new(pos, Role::Actor, None, Motion::Arrive)];
            vec![Scene::new(after, entry, marks, 0.8)]
        }
        Event::KeptHidden { pos, .. } => {
            vec![Scene::new(
                after,
                entry,
                actor(Some(pos), Some(Label::Gain(1))).into_iter().collect(),
                0.9,
            )]
        }
        Event::Revealed { pos, gained, .. } => {
            let label = (gained > 0).then_some(Label::Gain(gained));
            vec![Scene::new(
                after,
                entry,
                actor(Some(pos), label).into_iter().collect(),
                1.4,
            )]
        }
        Event::Gained { pos, amount, .. } => {
            let marks = actor(pos, Some(Label::Gain(amount))).into_iter().collect();
            vec![Scene::new(after, entry, marks, 1.2)]
        }
        Event::Stole { pos, target, .. } => {
            let mut marks: Vec<Mark> = actor(pos, Some(Label::Gain(1))).into_iter().collect();
            marks.push(Mark::new(
                target,
                Role::Target,
                Some(Label::Loss(1)),
                Motion::Pulse,
            ));
            vec![Scene::new(after, entry, marks, 1.4)]
        }
        Event::Copied { pos, target, .. } => {
            let mut marks: Vec<Mark> = actor(pos, None).into_iter().collect();
            marks.push(Mark::new(
                target,
                Role::Target,
                Some(Label::Copied),
                Motion::Pulse,
            ));
            vec![Scene::new(after, entry, marks, 1.2)]
        }
        Event::Eliminated {
            by_pos, target_pos, ..
        } => {
            turn_up(target_pos);
            let mut marks: Vec<Mark> = actor(
                by_pos.filter(|&pos| pos != target_pos),
                Some(Label::Gain(1)),
            )
            .into_iter()
            .collect();
            marks.push(Mark::new(
                target_pos,
                Role::Target,
                Some(Label::Eliminated),
                Motion::Vanish,
            ));
            vec![Scene::new(before, entry, marks, 1.6)]
        }
        Event::Ambushed {
            by_pos,
            target_pos,
            gained,
            ..
        } => {
            turn_up(target_pos);
            let mut marks: Vec<Mark> = actor(
                by_pos.filter(|&pos| pos != target_pos),
                Some(Label::Gain(1)),
            )
            .into_iter()
            .collect();
            marks.push(Mark::new(
                target_pos,
                Role::Target,
                Some(Label::Ambush(gained)),
                Motion::Pulse,
            ));
            vec![Scene::new(before, entry, marks, 1.8)]
        }
        Event::Discarded { pos, .. } => {
            let marks = vec![Mark::new(
                pos,
                Role::Target,
                Some(Label::Discarded),
                Motion::Vanish,
            )];
            vec![Scene::new(before, entry, marks, 1.3)]
        }
        Event::Moved { from, to, .. } => {
            let leaving = vec![Mark::new(
                from,
                Role::Target,
                Some(Label::Moves),
                Motion::Vanish,
            )];
            let arriving = vec![Mark::new(to, Role::Actor, None, Motion::Arrive)];
            vec![
                Scene::new(before, entry, leaving, 1.2),
                Scene {
                    first: false,
                    ..Scene::new(after, entry, arriving, 0.9)
                },
            ]
        }
    }
}

/// Pacing and background work for computer moves.
#[derive(Resource)]
pub struct BotTurn {
    timer: Timer,
    task: Option<Task<Option<Action>>>,
}

impl Default for BotTurn {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(BOT_DELAY, TimerMode::Once),
            task: None,
        }
    }
}

impl BotTurn {
    /// Drops any move being computed and restarts the delay.
    pub fn reset(&mut self) {
        self.task = None;
        self.timer.reset();
    }
}

/// Lets the computer players move. The strategy runs on a background thread
/// so a long search does not freeze the window.
pub fn bot_turns(
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut bot: ResMut<BotTurn>,
    mut dirty: ResMut<UiDirty>,
    mut next: ResMut<NextState<Screen>>,
) {
    // Nobody moves, and the game does not end, until the last action is shown
    // and the log window is closed.
    if session.playing_back() || session.log_window.is_some() {
        return;
    }
    let Some(player) = session.state.current_player() else {
        next.set(Screen::GameOver);
        return;
    };
    if player == HUMAN {
        return;
    }
    if let Some(task) = bot.task.as_mut() {
        if let Some(choice) = check_ready(task) {
            bot.reset();
            if let Some(action) = choice {
                session.apply(action);
            }
            dirty.0 = true;
        }
        return;
    }
    if !bot.timer.tick(time.delta()).is_finished() {
        return;
    }
    let view = session.state.view_for(player);
    let legal = session.state.legal_actions();
    let mut rng = session.rng.fork();
    let difficulty = session.difficulty;
    bot.task = Some(
        AsyncComputeTaskPool::get()
            .spawn(async move { make_strategy(difficulty).choose(&view, &legal, &mut rng) }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plays a whole game with random legal actions and returns its steps
    /// and the final state.
    fn random_game(seed: u64, players: usize) -> (GameState, Vec<Step>) {
        let mut rng = Rng64::new(seed);
        let mut state = GameState::new(players, &mut rng).unwrap();
        let mut steps = Vec::new();
        while state.current_player().is_some() {
            let legal = state.legal_actions();
            let action = legal[rng.below(legal.len())];
            steps.extend(state.apply_traced(action).unwrap());
        }
        (state, steps)
    }

    /// The starting influence plus the changes of all entries makes the
    /// final scores.
    #[test]
    fn changes_add_up_to_the_scores() {
        for (seed, players) in (0..20).zip([3, 4, 5].into_iter().cycle()) {
            let (state, steps) = random_game(seed, players);
            // Every player starts with some influence.
            let mut totals: Vec<i64> = steps[0]
                .before
                .influence
                .iter()
                .map(|&i| i64::from(i))
                .collect();
            for step in &steps {
                for (player, change) in Entry::from_step(step).changes {
                    totals[player] = totals[player].saturating_add(change);
                }
            }
            let scores: Vec<i64> = state.scores().into_iter().map(i64::from).collect();
            assert_eq!(totals, scores, "seed {seed}");
        }
    }

    /// The log never names an opponent's face-down card, and always names
    /// the human's own cards.
    #[test]
    fn hidden_cards_stay_hidden() {
        for seed in 0..20 {
            let (_, steps) = random_game(seed, 4);
            for step in &steps {
                let entry = Entry::from_step(step);
                match step.event {
                    Event::Placed { player, .. } | Event::KeptHidden { player, .. } => {
                        let actor = entry.actor.unwrap();
                        assert_eq!(actor.owner, player);
                        assert_eq!(actor.kind.is_some(), player == HUMAN, "{:?}", step.event);
                    }
                    Event::Revealed { .. } | Event::Eliminated { .. } | Event::Copied { .. } => {
                        assert!(entry.actor.unwrap().kind.is_some());
                    }
                    _ => {}
                }
            }
        }
    }

    /// Every story names the cards its sentence mentions, and no mark is
    /// left in the text.
    #[test]
    fn stories_have_their_cards() {
        let (_, steps) = random_game(7, 5);
        for step in &steps {
            let entry = Entry::from_step(step);
            for piece in entry.story(true) {
                match piece {
                    Piece::Text(text) => {
                        assert!(!text.is_empty());
                        assert!(!text.contains([ACTOR_MARK, TARGET_MARK]), "{text}");
                    }
                    Piece::Card { card, .. } => {
                        if !matches!(step.event, Event::Gained { pos: None, .. }) {
                            assert!(card.is_some(), "{:?}", step.event);
                        }
                    }
                }
            }
        }
    }

    /// Every round's story tells each line once, in order: the placements
    /// open the round, and each paragraph starts with a lead sentence.
    #[test]
    fn paragraphs_tell_every_line_once() {
        let mut rng = Rng64::new(3);
        let mut state = GameState::new(4, &mut rng).unwrap();
        let mut log = vec![RoundLog::new(1)];
        while state.current_player().is_some() {
            let legal = state.legal_actions();
            let action = legal[rng.below(legal.len())];
            for step in state.apply_traced(action).unwrap() {
                let entry = Entry::from_step(&step);
                match entry.event {
                    Event::RoundStarted(round) => log.push(RoundLog::new(round)),
                    Event::GameOver => {}
                    _ => log.last_mut().unwrap().lines.push(LogLine::Entry(entry)),
                }
            }
        }
        assert_eq!(log.len(), 6);
        for story in &log {
            let paragraphs = story.paragraphs();
            let told: Vec<&LogLine> = paragraphs.iter().flatten().map(|s| s.line).collect();
            assert_eq!(told, story.lines.iter().collect::<Vec<_>>());
            assert!(paragraphs.iter().all(|p| p[0].lead));
            assert!(paragraphs[0].iter().all(|s| matches!(
                s.line,
                LogLine::Entry(Entry {
                    event: Event::Placed { .. },
                    ..
                })
            )));
            assert!(paragraphs.len() > 1);
        }
    }

    #[test]
    fn punctuation_stays_with_its_card() {
        let lord = Some(Seen {
            owner: 1,
            kind: Some(CardKind::Lord),
        });
        let spy = Some(Seen {
            owner: 0,
            kind: Some(CardKind::Spy),
        });
        let pieces = split_story("A (\u{E001}) and \u{E000}.", spy, lord);
        let card = |card, prefix: &str, suffix: &str| Piece::Card {
            card,
            prefix: prefix.to_owned(),
            suffix: suffix.to_owned(),
        };
        assert_eq!(
            pieces,
            vec![
                Piece::Text("A ".to_owned()),
                card(lord, "(", ")"),
                Piece::Text(" and ".to_owned()),
                card(spy, "", "."),
            ]
        );
    }

    /// Manual stepping stops before every computer card that is revealed or
    /// kept hidden, and never while cards are put down or before the human's
    /// own choice.
    #[test]
    fn manual_stepping_stops_before_card_actions() {
        set_stepping(Stepping::Manual);
        for seed in 0..10 {
            let mut session = Session::new(Settings::default()).unwrap();
            session.rng = Rng64::new(seed);
            let mut rng = Rng64::new(seed);
            let mut stops = 0;
            while session.state.current_player().is_some() || session.playing_back() {
                if let Some(scene) = &session.scene {
                    let card_turn = matches!(
                        scene.entry.event,
                        Event::KeptHidden { .. } | Event::Revealed { .. }
                    );
                    if scene.first && card_turn && !scene.chosen {
                        assert!(session.waiting, "{:?}", scene.entry.event);
                    }
                    if scene.chosen
                        || matches!(
                            scene.entry.event,
                            Event::RoundStarted(_) | Event::Placed { .. }
                        )
                    {
                        assert!(!session.waiting, "{:?}", scene.entry.event);
                    }
                    if session.waiting {
                        stops += 1;
                        session.go_on();
                    }
                    session.next_scene();
                    continue;
                }
                let legal = session.state.legal_actions();
                session.apply(legal[rng.below(legal.len())]);
            }
            assert!(stops > 0);
        }
        set_stepping(Stepping::Auto);
    }
}
