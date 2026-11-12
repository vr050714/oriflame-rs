//! The running game, the step-by-step playback of each action, and the
//! system that lets computer players move.

use crate::{
    Screen,
    screens::{Motion, UiDirty},
};
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures::check_ready},
};
use oriflamme_ai::{Difficulty, make_strategy};
use oriflamme_core::{
    Action, CardKind, Event, GameState, MIN_PLAYERS, PlayerId, Rng64, SetupError, Slot, Snapshot,
    Stack, Step,
};
use std::collections::VecDeque;
use web_time::{SystemTime, UNIX_EPOCH};

/// The human always sits in seat 0.
pub const HUMAN: PlayerId = 0;

/// Seconds a computer player waits before it starts to think.
const BOT_DELAY: f32 = 0.5;

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

/// A card singled out in a scene.
pub struct Mark {
    pub pos: usize,
    pub role: Role,
    /// Short text shown on the card, for example `+2` or `eliminated`.
    pub label: Option<String>,
    pub motion: Motion,
}

impl Mark {
    fn new(pos: usize, role: Role, label: impl Into<String>, motion: Motion) -> Self {
        let label = Some(label.into()).filter(|text| !text.is_empty());
        Self {
            pos,
            role,
            label,
            motion,
        }
    }
}

/// One step of an action as it is shown to the player: a frozen table, the
/// cards involved, and a sentence saying what happens.
pub struct Scene {
    pub queue: Vec<Stack>,
    pub influence: Vec<u32>,
    pub text: String,
    pub marks: Vec<Mark>,
    seconds: f32,
}

impl Scene {
    fn new(table: Snapshot, text: String, marks: Vec<Mark>, seconds: f32) -> Self {
        Self {
            queue: table.queue,
            influence: table.influence,
            text,
            marks,
            seconds,
        }
    }
}

/// One game in progress.
#[derive(Resource)]
pub struct Session {
    pub state: GameState,
    pub difficulty: Difficulty,
    pub rng: Rng64,
    /// Game log, oldest first.
    pub log: Vec<String>,
    /// Hand card the human picked and is about to place.
    pub selected: Option<CardKind>,
    /// Royal Decree: the position the human picked to move a card from.
    pub move_from: Option<usize>,
    /// The scene on screen. While one is shown, nobody can move.
    pub scene: Option<Scene>,
    /// Scenes still to be shown.
    scenes: VecDeque<Scene>,
    scene_timer: Timer,
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
            log: vec!["Round 1 begins.".to_owned()],
            selected: None,
            move_from: None,
            scene: None,
            scenes: VecDeque::new(),
            scene_timer: Timer::from_seconds(0.0, TimerMode::Once),
        })
    }

    /// True while the last action is still being shown.
    pub fn playing_back(&self) -> bool {
        self.scene.is_some()
    }

    /// True when the human must act.
    pub fn human_turn(&self) -> bool {
        !self.playing_back() && self.state.current_player() == Some(HUMAN)
    }

    /// Applies an action for the current player and queues its scenes.
    pub fn apply(&mut self, action: Action) {
        match self.state.apply_traced(action) {
            Ok(steps) => {
                self.scenes.extend(steps.into_iter().flat_map(scenes_for));
                if self.scene.is_none() {
                    self.next_scene();
                }
            }
            Err(error) => self.log.push(error.to_string()),
        }
        self.selected = None;
        self.move_from = None;
    }

    /// Shows the next queued scene, or returns to the live table.
    fn next_scene(&mut self) {
        self.scene = self.scenes.pop_front();
        if let Some(scene) = &self.scene {
            // An action shown in two scenes is logged once.
            if self.log.last() != Some(&scene.text) {
                self.log.push(scene.text.clone());
            }
            self.scene_timer = Timer::from_seconds(scene.seconds, TimerMode::Once);
        }
    }
}

/// Advances the playback of the last action.
pub fn play_scenes(time: Res<Time>, mut session: ResMut<Session>, mut dirty: ResMut<UiDirty>) {
    if session.playing_back() && session.scene_timer.tick(time.delta()).is_finished() {
        session.next_scene();
        dirty.0 = true;
    }
}

/// Display name of a player.
pub fn player_name(player: PlayerId) -> String {
    if player == HUMAN {
        "You".to_owned()
    } else {
        format!("PC {player}")
    }
}

/// The scenes that show one event.
fn scenes_for(step: Step) -> Vec<Scene> {
    let Step {
        event,
        mut before,
        after,
    } = step;
    let text = describe(event);
    // Shows the identity of a face-down card that is about to leave the table.
    let mut turn_up = |pos: usize| {
        if let Some(card) = before.queue.get_mut(pos).and_then(|stack| stack.last_mut()) {
            card.face_up = true;
        }
    };
    let actor = |pos: Option<usize>, label: &str| {
        pos.map(|pos| Mark::new(pos, Role::Actor, label, Motion::Pulse))
    };
    match event {
        Event::RoundStarted(_) => vec![Scene::new(after, text, Vec::new(), 0.9)],
        Event::GameOver => Vec::new(),
        Event::Placed { slot, .. } => {
            let pos = match slot {
                Slot::Left => 0,
                Slot::Right => after.queue.len().saturating_sub(1),
                Slot::OnStack(pos) => pos,
            };
            let marks = vec![Mark::new(pos, Role::Actor, "", Motion::Arrive)];
            vec![Scene::new(after, text, marks, 0.8)]
        }
        Event::KeptHidden { pos, .. } => {
            vec![Scene::new(
                after,
                text,
                actor(Some(pos), "+1").into_iter().collect(),
                0.9,
            )]
        }
        Event::Revealed { pos, gained, .. } => {
            let label = if gained > 0 {
                format!("+{gained}")
            } else {
                String::new()
            };
            vec![Scene::new(
                after,
                text,
                actor(Some(pos), &label).into_iter().collect(),
                1.4,
            )]
        }
        Event::Gained { pos, amount, .. } => {
            let marks = actor(pos, &format!("+{amount}")).into_iter().collect();
            vec![Scene::new(after, text, marks, 1.2)]
        }
        Event::Stole { pos, target, .. } => {
            let mut marks: Vec<Mark> = actor(pos, "+1").into_iter().collect();
            marks.push(Mark::new(target, Role::Target, "-1", Motion::Pulse));
            vec![Scene::new(after, text, marks, 1.4)]
        }
        Event::Copied { pos, target, .. } => {
            let mut marks: Vec<Mark> = actor(pos, "").into_iter().collect();
            marks.push(Mark::new(target, Role::Target, "copied", Motion::Pulse));
            vec![Scene::new(after, text, marks, 1.2)]
        }
        Event::Eliminated {
            by_pos, target_pos, ..
        } => {
            turn_up(target_pos);
            let mut marks: Vec<Mark> = actor(by_pos.filter(|&pos| pos != target_pos), "+1")
                .into_iter()
                .collect();
            marks.push(Mark::new(
                target_pos,
                Role::Target,
                "eliminated",
                Motion::Vanish,
            ));
            vec![Scene::new(before, text, marks, 1.6)]
        }
        Event::Ambushed {
            by_pos,
            target_pos,
            gained,
            ..
        } => {
            turn_up(target_pos);
            let mut marks: Vec<Mark> = actor(by_pos.filter(|&pos| pos != target_pos), "+1")
                .into_iter()
                .collect();
            let label = format!("Ambush! +{gained}");
            marks.push(Mark::new(target_pos, Role::Target, label, Motion::Pulse));
            vec![Scene::new(before, text, marks, 1.8)]
        }
        Event::Discarded { pos, .. } => {
            let marks = vec![Mark::new(pos, Role::Target, "discarded", Motion::Vanish)];
            vec![Scene::new(before, text, marks, 1.3)]
        }
        Event::Moved { from, to, .. } => {
            let leaving = vec![Mark::new(from, Role::Target, "moves", Motion::Vanish)];
            let arriving = vec![Mark::new(to, Role::Actor, "", Motion::Arrive)];
            vec![
                Scene::new(before, text.clone(), leaving, 1.2),
                Scene::new(after, text, arriving, 0.9),
            ]
        }
    }
}

/// One sentence for an event.
fn describe(event: Event) -> String {
    match event {
        Event::RoundStarted(round) => format!("Round {round} begins."),
        Event::Placed { player, slot } => {
            let place = match slot {
                Slot::Left => "at the start of the queue",
                Slot::Right => "at the end of the queue",
                Slot::OnStack(_) => "on top of an own card",
            };
            format!("{} placed a card {place}.", player_name(player))
        }
        Event::KeptHidden {
            player, influence, ..
        } => {
            format!(
                "{} kept a card hidden ({influence} influence on it).",
                player_name(player)
            )
        }
        Event::Revealed { card, gained, .. } => {
            format!(
                "{} revealed {} and took {gained} influence.",
                player_name(card.owner),
                card.kind
            )
        }
        Event::Gained { player, amount, .. } => {
            format!("{} gained {amount} influence.", player_name(player))
        }
        Event::Stole { thief, victim, .. } => {
            format!(
                "{} stole 1 influence from {}.",
                player_name(thief),
                player_name(victim)
            )
        }
        Event::Copied { player, kind, .. } => {
            format!("{}: Shapeshifter copies {kind}.", player_name(player))
        }
        Event::Eliminated { by, target, .. } => format!(
            "{}: {} eliminated {} of {} (+1).",
            player_name(by.owner),
            by.kind,
            target.kind,
            player_name(target.owner)
        ),
        Event::Ambushed {
            attacker,
            owner,
            gained,
            ..
        } => format!(
            "{}: {} hit an Ambush of {} (+{gained} for the Ambush).",
            player_name(attacker.owner),
            attacker.kind,
            player_name(owner)
        ),
        Event::Discarded { card, .. } => {
            format!("{} discarded {}.", player_name(card.owner), card.kind)
        }
        Event::Moved {
            player,
            moved_owner,
            to,
            ..
        } => format!(
            "{} moved a card of {} to position {}.",
            player_name(player),
            player_name(moved_owner),
            to.saturating_add(1)
        ),
        Event::GameOver => "The game is over.".to_owned(),
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
    // Nobody moves, and the game does not end, until the last action is shown.
    if session.playing_back() {
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
