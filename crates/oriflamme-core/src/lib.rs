//! Rules engine for the card game Oriflamme (base game, 3 to 5 players).
//!
//! The engine is a pure state machine. [`GameState::legal_actions`] lists what
//! the current player may do and [`GameState::apply`] is the only way to change
//! the state. Illegal input returns an error; nothing in this crate panics.
//!
//! Rule choices where the rulebook is silent:
//! - Spy: the owner picks which adjacent opponent to steal from.
//! - Shapeshifter: copies an adjacent revealed Archer, Soldier, Spy, Heir or Lord.
//! - Royal Decree: moves the top card of any other stack to any gap or end of
//!   the queue; it does not put a card onto a stack.
//! - A card that becomes uncovered during a resolution phase is not resolved
//!   until the next round.
//! - Ties after the queue-count tie-break are shared.

mod rng;
mod view;

pub use rng::Rng64;
pub use view::{PlayerView, ViewCard, ViewPhase};

use std::fmt;

/// Smallest supported number of players.
pub const MIN_PLAYERS: usize = 3;
/// Largest supported number of players.
pub const MAX_PLAYERS: usize = 5;
/// Number of rounds in a game.
pub const ROUNDS: u8 = 6;
/// Cards in hand at the start of the game.
pub const HAND_SIZE: usize = 7;

/// Index of a player, `0..player_count`.
pub type PlayerId = usize;

/// The ten card types every family owns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CardKind {
    Archer,
    Soldier,
    Spy,
    Heir,
    Assassination,
    RoyalDecree,
    Lord,
    Shapeshifter,
    Ambush,
    Conspiracy,
}

impl CardKind {
    /// Every card type, in a fixed order.
    pub const ALL: [CardKind; 10] = [
        CardKind::Archer,
        CardKind::Soldier,
        CardKind::Spy,
        CardKind::Heir,
        CardKind::Assassination,
        CardKind::RoyalDecree,
        CardKind::Lord,
        CardKind::Shapeshifter,
        CardKind::Ambush,
        CardKind::Conspiracy,
    ];

    /// Display name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            CardKind::Archer => "Archer",
            CardKind::Soldier => "Soldier",
            CardKind::Spy => "Spy",
            CardKind::Heir => "Heir",
            CardKind::Assassination => "Assassination",
            CardKind::RoyalDecree => "Royal Decree",
            CardKind::Lord => "Lord",
            CardKind::Shapeshifter => "Shapeshifter",
            CardKind::Ambush => "Ambush",
            CardKind::Conspiracy => "Conspiracy",
        }
    }

    /// Lower-case identifier, used for asset file names.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            CardKind::Archer => "archer",
            CardKind::Soldier => "soldier",
            CardKind::Spy => "spy",
            CardKind::Heir => "heir",
            CardKind::Assassination => "assassination",
            CardKind::RoyalDecree => "royal_decree",
            CardKind::Lord => "lord",
            CardKind::Shapeshifter => "shapeshifter",
            CardKind::Ambush => "ambush",
            CardKind::Conspiracy => "conspiracy",
        }
    }

    /// Short rules text.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            CardKind::Archer => "Eliminate the first or last card of the queue.",
            CardKind::Soldier => "Eliminate an adjacent card.",
            CardKind::Spy => "Steal 1 influence from a player with an adjacent card.",
            CardKind::Heir => "Gain 2 influence if no other Heir is revealed.",
            CardKind::Assassination => "Eliminate any card. Discard Assassination.",
            CardKind::RoyalDecree => "Move a card anywhere in the queue. Discard Royal Decree.",
            CardKind::Lord => "Gain 1 influence, plus 1 per adjacent card of your family.",
            CardKind::Shapeshifter => "Copy the ability of an adjacent revealed character.",
            CardKind::Ambush => {
                "Eliminated face down by an opponent: gain 4 and discard the attacker. \
                 Revealed by you: gain 1. Discard Ambush."
            }
            CardKind::Conspiracy => "Gain double the influence on it. Discard Conspiracy.",
        }
    }

    /// True for the abilities a Shapeshifter may copy.
    #[must_use]
    pub fn is_copyable(self) -> bool {
        matches!(
            self,
            CardKind::Archer | CardKind::Soldier | CardKind::Spy | CardKind::Heir | CardKind::Lord
        )
    }
}

impl fmt::Display for CardKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A specific card. Each player owns one card of each kind, so this is unique.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Card {
    pub owner: PlayerId,
    pub kind: CardKind,
}

/// A card lying in the queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueCard {
    pub card: Card,
    pub face_up: bool,
    /// Influence tokens lying on the card.
    pub influence: u32,
}

/// A pile of cards at one queue position. The last element is the top card;
/// only the top card is active.
pub type Stack = Vec<QueueCard>;

/// One player's private and public state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    pub hand: Vec<CardKind>,
    /// The three cards removed during setup.
    pub removed: Vec<CardKind>,
    /// Discarded and eliminated cards, face up.
    pub discard: Vec<CardKind>,
    pub influence: u32,
}

/// Where a card is placed during the placement phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    /// Before the first card of the queue.
    Left,
    /// After the last card of the queue.
    Right,
    /// On top of the player's own card at this queue position.
    OnStack(usize),
}

/// A move by the current player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    /// Placement phase: play a card from hand, face down.
    Place { kind: CardKind, slot: Slot },
    /// Resolution phase: reveal the current face-down card.
    Reveal,
    /// Resolution phase: keep the current card face down and add 1 influence to it.
    KeepHidden,
    /// Choose the queue position an ability applies to.
    Target(usize),
    /// Royal Decree: take the top card at `from` and insert it as a new stack
    /// at index `to` (an index into the queue after the card is taken).
    Move { from: usize, to: usize },
}

/// What the current player must decide during resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Reveal the face-down card or keep it hidden.
    RevealOrNot,
    /// Choose the target of this ability.
    Ability(CardKind),
}

/// Current step of the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// `player` must place a card; `placed` players have placed this round.
    Placement {
        player: PlayerId,
        placed: usize,
    },
    /// The owner of `card` must make `decision`.
    Resolution {
        card: Card,
        decision: Decision,
    },
    GameOver,
}

/// Something that happened as a result of an action. Used for the game log
/// and for replaying an action step by step.
///
/// Queue positions of cards that leave or move (`Eliminated`, `Ambushed`,
/// `Discarded`, `Moved::from`) refer to the queue before the event. All other
/// positions refer to the queue after the event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    RoundStarted(u8),
    Placed {
        player: PlayerId,
        slot: Slot,
    },
    KeptHidden {
        player: PlayerId,
        pos: usize,
        influence: u32,
    },
    Revealed {
        card: Card,
        pos: usize,
        gained: u32,
    },
    /// The card at `pos` earned influence for its owner.
    Gained {
        player: PlayerId,
        pos: Option<usize>,
        amount: u32,
    },
    Stole {
        thief: PlayerId,
        victim: PlayerId,
        pos: Option<usize>,
        target: usize,
    },
    Copied {
        player: PlayerId,
        kind: CardKind,
        pos: Option<usize>,
        target: usize,
    },
    Eliminated {
        by: Card,
        by_pos: Option<usize>,
        target: Card,
        target_pos: usize,
    },
    Ambushed {
        attacker: Card,
        by_pos: Option<usize>,
        owner: PlayerId,
        target_pos: usize,
        gained: u32,
    },
    Discarded {
        card: Card,
        pos: usize,
    },
    Moved {
        player: PlayerId,
        moved_owner: PlayerId,
        from: usize,
        to: usize,
    },
    GameOver,
}

/// The public table at one moment: the queue and every player's influence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub queue: Vec<Stack>,
    pub influence: Vec<u32>,
}

/// One event of an action, with the table before and after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub event: Event,
    pub before: Snapshot,
    pub after: Snapshot,
}

/// Collects the events of one action, and table snapshots when asked to.
#[derive(Default)]
struct Trace {
    events: Vec<Event>,
    /// The table after each event; `None` when snapshots are not wanted.
    frames: Option<Vec<Snapshot>>,
}

/// Error from [`GameState::new`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupError {
    /// The player count is outside `MIN_PLAYERS..=MAX_PLAYERS`.
    InvalidPlayerCount(usize),
}

impl fmt::Display for SetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SetupError::InvalidPlayerCount(n) => {
                write!(
                    f,
                    "player count {n} is not in {MIN_PLAYERS}..={MAX_PLAYERS}"
                )
            }
        }
    }
}

impl std::error::Error for SetupError {}

/// Error from [`GameState::apply`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleError {
    /// The action is not one of the legal actions in this state.
    IllegalAction(Action),
    /// The state is inconsistent. This indicates a bug in the engine.
    Corrupt,
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleError::IllegalAction(action) => write!(f, "illegal action: {action:?}"),
            RuleError::Corrupt => f.write_str("inconsistent game state"),
        }
    }
}

impl std::error::Error for RuleError {}

/// Complete state of a game, including hidden information.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameState {
    pub players: Vec<Player>,
    /// The influence queue. Index 0 is resolved first.
    pub queue: Vec<Stack>,
    /// Current round, `1..=ROUNDS`.
    pub round: u8,
    /// Player who places first this round.
    pub first_player: PlayerId,
    pub phase: Phase,
    /// Cards already handled in the current resolution phase.
    pub resolved: Vec<Card>,
}

impl GameState {
    /// Starts a game: every player gets 7 random cards of their 10 and 1 influence.
    ///
    /// # Errors
    /// Returns [`SetupError::InvalidPlayerCount`] unless `player_count` is 3, 4 or 5.
    pub fn new(player_count: usize, rng: &mut Rng64) -> Result<Self, SetupError> {
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&player_count) {
            return Err(SetupError::InvalidPlayerCount(player_count));
        }
        let players = (0..player_count)
            .map(|_| {
                let mut hand = CardKind::ALL.to_vec();
                rng.shuffle(&mut hand);
                let removed = hand.split_off(HAND_SIZE);
                Player {
                    hand,
                    removed,
                    discard: Vec::new(),
                    influence: 1,
                }
            })
            .collect();
        let first_player = rng.below(player_count);
        Ok(Self {
            players,
            queue: Vec::new(),
            round: 1,
            first_player,
            phase: Phase::Placement {
                player: first_player,
                placed: 0,
            },
            resolved: Vec::new(),
        })
    }

    /// Number of players.
    #[must_use]
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// The player who must act, or `None` when the game is over.
    #[must_use]
    pub fn current_player(&self) -> Option<PlayerId> {
        match self.phase {
            Phase::Placement { player, .. } => Some(player),
            Phase::Resolution { card, .. } => Some(card.owner),
            Phase::GameOver => None,
        }
    }

    /// True when the game has ended.
    #[must_use]
    pub fn is_over(&self) -> bool {
        self.phase == Phase::GameOver
    }

    /// The active (top) card at a queue position.
    #[must_use]
    pub fn top(&self, pos: usize) -> Option<&QueueCard> {
        self.queue.get(pos).and_then(|stack| stack.last())
    }

    /// Queue position whose top card is `card`.
    #[must_use]
    pub fn position_of(&self, card: Card) -> Option<usize> {
        self.queue
            .iter()
            .position(|stack| stack.last().is_some_and(|qc| qc.card == card))
    }

    /// Influence of every player. Influence lying on cards does not count.
    #[must_use]
    pub fn scores(&self) -> Vec<u32> {
        self.players.iter().map(|p| p.influence).collect()
    }

    /// Number of cards a player has in the queue, covered cards included.
    #[must_use]
    pub fn cards_in_queue(&self, player: PlayerId) -> usize {
        self.queue
            .iter()
            .flatten()
            .filter(|qc| qc.card.owner == player)
            .count()
    }

    /// Players with the most influence; ties are broken by cards in the queue.
    /// More than one player is returned when the tie remains.
    #[must_use]
    pub fn winners(&self) -> Vec<PlayerId> {
        let key = |p: PlayerId| {
            let influence = self.players.get(p).map_or(0, |pl| pl.influence);
            (influence, self.cards_in_queue(p))
        };
        let best = (0..self.player_count()).map(key).max();
        (0..self.player_count())
            .filter(|&p| Some(key(p)) == best)
            .collect()
    }

    /// Positions next to `pos` that hold a card.
    fn adjacent(&self, pos: usize) -> Vec<usize> {
        let mut out = Vec::with_capacity(2);
        if let Some(left) = pos.checked_sub(1) {
            out.push(left);
        }
        let right = pos.saturating_add(1);
        if right < self.queue.len() {
            out.push(right);
        }
        out
    }

    /// Every action the current player may take. Empty when the game is over.
    #[must_use]
    pub fn legal_actions(&self) -> Vec<Action> {
        match self.phase {
            Phase::Placement { player, .. } => self.placement_actions(player),
            Phase::Resolution {
                decision: Decision::RevealOrNot,
                ..
            } => {
                vec![Action::Reveal, Action::KeepHidden]
            }
            Phase::Resolution {
                card,
                decision: Decision::Ability(kind),
            } => self.ability_targets(card, kind),
            Phase::GameOver => Vec::new(),
        }
    }

    fn placement_actions(&self, player: PlayerId) -> Vec<Action> {
        let Some(hand) = self.players.get(player).map(|p| &p.hand) else {
            return Vec::new();
        };
        let mut slots = vec![Slot::Left];
        if !self.queue.is_empty() {
            slots.push(Slot::Right);
            // Stacking on your own cards is allowed from the second round.
            if self.round >= 2 {
                for (pos, stack) in self.queue.iter().enumerate() {
                    if stack.last().is_some_and(|qc| qc.card.owner == player) {
                        slots.push(Slot::OnStack(pos));
                    }
                }
            }
        }
        let mut out = Vec::with_capacity(hand.len().saturating_mul(slots.len()));
        for &kind in hand {
            for &slot in &slots {
                out.push(Action::Place { kind, slot });
            }
        }
        out
    }

    /// Legal targets when `card` uses the ability `kind`.
    fn ability_targets(&self, card: Card, kind: CardKind) -> Vec<Action> {
        let Some(pos) = self.position_of(card) else {
            return Vec::new();
        };
        let len = self.queue.len();
        match kind {
            CardKind::Archer => {
                let last = len.saturating_sub(1);
                if last == 0 {
                    vec![Action::Target(0)]
                } else {
                    vec![Action::Target(0), Action::Target(last)]
                }
            }
            CardKind::Soldier => self.adjacent(pos).into_iter().map(Action::Target).collect(),
            CardKind::Spy => {
                let mut victims: Vec<PlayerId> = Vec::new();
                let mut out = Vec::new();
                for adj in self.adjacent(pos) {
                    let Some(owner) = self.top(adj).map(|qc| qc.card.owner) else {
                        continue;
                    };
                    let has_influence = self.players.get(owner).is_some_and(|p| p.influence > 0);
                    if owner != card.owner && has_influence && !victims.contains(&owner) {
                        victims.push(owner);
                        out.push(Action::Target(adj));
                    }
                }
                out
            }
            CardKind::Shapeshifter => self
                .adjacent(pos)
                .into_iter()
                .filter(|&adj| {
                    self.top(adj)
                        .is_some_and(|qc| qc.face_up && qc.card.kind.is_copyable())
                })
                .map(Action::Target)
                .collect(),
            CardKind::Assassination => (0..len).filter(|&p| p != pos).map(Action::Target).collect(),
            CardKind::RoyalDecree => {
                let mut out = Vec::new();
                for (from, stack) in self.queue.iter().enumerate() {
                    if from == pos {
                        continue;
                    }
                    let single = stack.len() == 1;
                    let len_after = if single { len.saturating_sub(1) } else { len };
                    for to in 0..=len_after {
                        // Putting a lone card back where it was changes nothing.
                        if single && to == from {
                            continue;
                        }
                        out.push(Action::Move { from, to });
                    }
                }
                out
            }
            CardKind::Heir | CardKind::Lord | CardKind::Ambush | CardKind::Conspiracy => Vec::new(),
        }
    }

    /// Applies an action of the current player and returns what happened.
    ///
    /// # Errors
    /// Returns [`RuleError::IllegalAction`] when the action is not in
    /// [`GameState::legal_actions`]. The state is unchanged in that case.
    pub fn apply(&mut self, action: Action) -> Result<Vec<Event>, RuleError> {
        let mut trace = Trace::default();
        self.apply_inner(action, &mut trace)?;
        Ok(trace.events)
    }

    /// Like [`GameState::apply`], but also returns the table before and after
    /// every event, so the action can be shown step by step. Slower than
    /// `apply`, because it copies the queue once per event.
    ///
    /// # Errors
    /// Same as [`GameState::apply`].
    pub fn apply_traced(&mut self, action: Action) -> Result<Vec<Step>, RuleError> {
        let mut before = self.snapshot();
        let mut trace = Trace {
            events: Vec::new(),
            frames: Some(Vec::new()),
        };
        self.apply_inner(action, &mut trace)?;
        let frames = trace.frames.unwrap_or_default();
        Ok(trace
            .events
            .into_iter()
            .zip(frames)
            .map(|(event, after)| Step {
                event,
                before: std::mem::replace(&mut before, after.clone()),
                after,
            })
            .collect())
    }

    /// The public table right now.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            queue: self.queue.clone(),
            influence: self.scores(),
        }
    }

    /// Records an event. Must be called right after the event's own change to
    /// the table, so that consecutive snapshots differ by exactly one event.
    fn emit(&self, trace: &mut Trace, event: Event) {
        trace.events.push(event);
        if let Some(frames) = trace.frames.as_mut() {
            frames.push(self.snapshot());
        }
    }

    fn apply_inner(&mut self, action: Action, events: &mut Trace) -> Result<(), RuleError> {
        if !self.legal_actions().contains(&action) {
            return Err(RuleError::IllegalAction(action));
        }
        match (self.phase, action) {
            (Phase::Placement { player, placed }, Action::Place { kind, slot }) => {
                self.place(player, placed, kind, slot, events)?;
            }
            (
                Phase::Resolution {
                    card,
                    decision: Decision::RevealOrNot,
                },
                Action::Reveal,
            ) => {
                self.reveal(card, events)?;
            }
            (
                Phase::Resolution {
                    card,
                    decision: Decision::RevealOrNot,
                },
                Action::KeepHidden,
            ) => {
                let pos = self.position_of(card).ok_or(RuleError::Corrupt)?;
                let qc = self
                    .queue
                    .get_mut(pos)
                    .and_then(|stack| stack.last_mut())
                    .ok_or(RuleError::Corrupt)?;
                qc.influence = qc.influence.saturating_add(1);
                let influence = qc.influence;
                self.emit(
                    events,
                    Event::KeptHidden {
                        player: card.owner,
                        pos,
                        influence,
                    },
                );
                self.advance(events);
            }
            (
                Phase::Resolution {
                    card,
                    decision: Decision::Ability(kind),
                },
                Action::Target(t),
            ) => {
                self.use_target(card, kind, t, events)?;
            }
            (
                Phase::Resolution {
                    card,
                    decision: Decision::Ability(CardKind::RoyalDecree),
                },
                Action::Move { from, to },
            ) => {
                let moved = self.remove_top(from).ok_or(RuleError::Corrupt)?;
                let to = to.min(self.queue.len());
                let moved_owner = moved.card.owner;
                self.queue.insert(to, vec![moved]);
                self.emit(
                    events,
                    Event::Moved {
                        player: card.owner,
                        moved_owner,
                        from,
                        to,
                    },
                );
                self.discard_from_queue(card, events);
                self.advance(events);
            }
            _ => return Err(RuleError::IllegalAction(action)),
        }
        Ok(())
    }

    fn place(
        &mut self,
        player: PlayerId,
        placed: usize,
        kind: CardKind,
        slot: Slot,
        events: &mut Trace,
    ) -> Result<(), RuleError> {
        let hand = &mut self.players.get_mut(player).ok_or(RuleError::Corrupt)?.hand;
        let index = hand
            .iter()
            .position(|&k| k == kind)
            .ok_or(RuleError::Corrupt)?;
        hand.remove(index);
        let qc = QueueCard {
            card: Card {
                owner: player,
                kind,
            },
            face_up: false,
            influence: 0,
        };
        match slot {
            Slot::Left => self.queue.insert(0, vec![qc]),
            Slot::Right => self.queue.push(vec![qc]),
            Slot::OnStack(pos) => self.queue.get_mut(pos).ok_or(RuleError::Corrupt)?.push(qc),
        }
        self.emit(events, Event::Placed { player, slot });
        let placed = placed.saturating_add(1);
        let count = self.player_count();
        if placed >= count {
            self.resolved.clear();
            self.advance(events);
        } else {
            let next = player.saturating_add(1).checked_rem(count).unwrap_or(0);
            self.phase = Phase::Placement {
                player: next,
                placed,
            };
        }
        Ok(())
    }

    fn reveal(&mut self, card: Card, events: &mut Trace) -> Result<(), RuleError> {
        let pos = self.position_of(card).ok_or(RuleError::Corrupt)?;
        let qc = self
            .queue
            .get_mut(pos)
            .and_then(|stack| stack.last_mut())
            .ok_or(RuleError::Corrupt)?;
        qc.face_up = true;
        let on_card = std::mem::take(&mut qc.influence);
        match card.kind {
            CardKind::Conspiracy => {
                let gained = on_card.saturating_mul(2);
                self.gain(card.owner, gained);
                self.emit(events, Event::Revealed { card, pos, gained });
                self.discard_from_queue(card, events);
                self.advance(events);
            }
            CardKind::Ambush => {
                // The influence on a self-revealed Ambush is lost.
                self.gain(card.owner, 1);
                self.emit(
                    events,
                    Event::Revealed {
                        card,
                        pos,
                        gained: 1,
                    },
                );
                self.discard_from_queue(card, events);
                self.advance(events);
            }
            kind => {
                self.gain(card.owner, on_card);
                self.emit(
                    events,
                    Event::Revealed {
                        card,
                        pos,
                        gained: on_card,
                    },
                );
                if !self.begin_ability(card, kind, events) {
                    self.advance(events);
                }
            }
        }
        Ok(())
    }

    fn use_target(
        &mut self,
        card: Card,
        kind: CardKind,
        target: usize,
        events: &mut Trace,
    ) -> Result<(), RuleError> {
        match kind {
            CardKind::Archer | CardKind::Soldier => self.eliminate(card, target, events)?,
            CardKind::Assassination => {
                self.eliminate(card, target, events)?;
                // An Ambush may already have removed the Assassination.
                self.discard_from_queue(card, events);
            }
            CardKind::Spy => {
                let victim = self.top(target).ok_or(RuleError::Corrupt)?.card.owner;
                let pool = &mut self
                    .players
                    .get_mut(victim)
                    .ok_or(RuleError::Corrupt)?
                    .influence;
                if *pool > 0 {
                    *pool = pool.saturating_sub(1);
                    self.gain(card.owner, 1);
                    let pos = self.position_of(card);
                    self.emit(
                        events,
                        Event::Stole {
                            thief: card.owner,
                            victim,
                            pos,
                            target,
                        },
                    );
                }
            }
            CardKind::Shapeshifter => {
                let copied = self.top(target).ok_or(RuleError::Corrupt)?.card.kind;
                let pos = self.position_of(card);
                self.emit(
                    events,
                    Event::Copied {
                        player: card.owner,
                        kind: copied,
                        pos,
                        target,
                    },
                );
                if self.begin_ability(card, copied, events) {
                    return Ok(());
                }
            }
            _ => return Err(RuleError::Corrupt),
        }
        self.advance(events);
        Ok(())
    }

    /// Starts the ability `kind` for the revealed `card`. Returns true when the
    /// owner must choose a target; otherwise the ability is already complete.
    fn begin_ability(&mut self, card: Card, kind: CardKind, events: &mut Trace) -> bool {
        match kind {
            CardKind::Heir => {
                let other_heirs = self
                    .queue
                    .iter()
                    .filter_map(|stack| stack.last())
                    .filter(|qc| qc.face_up && qc.card.kind == CardKind::Heir && qc.card != card)
                    .count();
                // A Shapeshifter copying an Heir needs that Heir to be the only one.
                let allowed = usize::from(card.kind != CardKind::Heir);
                if other_heirs == allowed {
                    self.gain(card.owner, 2);
                    let pos = self.position_of(card);
                    self.emit(
                        events,
                        Event::Gained {
                            player: card.owner,
                            pos,
                            amount: 2,
                        },
                    );
                }
                false
            }
            CardKind::Lord => {
                let own_neighbours = self.position_of(card).map_or(0, |pos| {
                    self.adjacent(pos)
                        .into_iter()
                        .filter(|&adj| self.top(adj).is_some_and(|qc| qc.card.owner == card.owner))
                        .count()
                });
                let amount = u32::try_from(own_neighbours).unwrap_or(2).saturating_add(1);
                self.gain(card.owner, amount);
                let pos = self.position_of(card);
                self.emit(
                    events,
                    Event::Gained {
                        player: card.owner,
                        pos,
                        amount,
                    },
                );
                false
            }
            CardKind::Ambush | CardKind::Conspiracy => false,
            CardKind::Archer
            | CardKind::Soldier
            | CardKind::Spy
            | CardKind::Shapeshifter
            | CardKind::Assassination
            | CardKind::RoyalDecree => {
                if self.ability_targets(card, kind).is_empty() {
                    if matches!(card.kind, CardKind::Assassination | CardKind::RoyalDecree) {
                        self.discard_from_queue(card, events);
                    }
                    false
                } else {
                    self.phase = Phase::Resolution {
                        card,
                        decision: Decision::Ability(kind),
                    };
                    true
                }
            }
        }
    }

    /// `attacker` eliminates the top card at `target`.
    fn eliminate(
        &mut self,
        attacker: Card,
        target: usize,
        events: &mut Trace,
    ) -> Result<(), RuleError> {
        let by_pos = self.position_of(attacker);
        let victim = self.remove_top(target).ok_or(RuleError::Corrupt)?;
        self.gain(attacker.owner, 1);
        self.push_discard(victim.card);
        if victim.card.kind == CardKind::Ambush && !victim.face_up {
            let gained = if victim.card.owner == attacker.owner {
                1
            } else {
                4
            };
            self.gain(victim.card.owner, gained);
            self.emit(
                events,
                Event::Ambushed {
                    attacker,
                    by_pos,
                    owner: victim.card.owner,
                    target_pos: target,
                    gained,
                },
            );
            self.discard_from_queue(attacker, events);
        } else {
            self.emit(
                events,
                Event::Eliminated {
                    by: attacker,
                    by_pos,
                    target: victim.card,
                    target_pos: target,
                },
            );
        }
        Ok(())
    }

    fn gain(&mut self, player: PlayerId, amount: u32) {
        if let Some(p) = self.players.get_mut(player) {
            p.influence = p.influence.saturating_add(amount);
        }
    }

    fn push_discard(&mut self, card: Card) {
        if let Some(p) = self.players.get_mut(card.owner) {
            p.discard.push(card.kind);
        }
    }

    /// Removes the top card at `pos`. A card uncovered this way waits for the
    /// next round.
    fn remove_top(&mut self, pos: usize) -> Option<QueueCard> {
        let stack = self.queue.get_mut(pos)?;
        let removed = stack.pop()?;
        match stack.last() {
            Some(uncovered) => {
                let card = uncovered.card;
                if !self.resolved.contains(&card) {
                    self.resolved.push(card);
                }
            }
            None => {
                self.queue.remove(pos);
            }
        }
        Some(removed)
    }

    /// Discards `card` if it is the top card of a stack.
    fn discard_from_queue(&mut self, card: Card, events: &mut Trace) {
        if let Some(pos) = self.position_of(card)
            && self.remove_top(pos).is_some()
        {
            self.push_discard(card);
            self.emit(events, Event::Discarded { card, pos });
        }
    }

    /// Moves to the next card of the resolution phase, running abilities that
    /// need no choice, until a player must decide or the round ends.
    fn advance(&mut self, events: &mut Trace) {
        loop {
            let next = self
                .queue
                .iter()
                .filter_map(|stack| stack.last())
                .find(|qc| !self.resolved.contains(&qc.card))
                .map(|qc| (qc.card, qc.face_up));
            let Some((card, face_up)) = next else {
                self.end_round(events);
                return;
            };
            self.resolved.push(card);
            if !face_up {
                self.phase = Phase::Resolution {
                    card,
                    decision: Decision::RevealOrNot,
                };
                return;
            }
            if self.begin_ability(card, card.kind, events) {
                return;
            }
        }
    }

    fn end_round(&mut self, events: &mut Trace) {
        self.resolved.clear();
        if self.round >= ROUNDS {
            self.phase = Phase::GameOver;
            self.emit(events, Event::GameOver);
            return;
        }
        self.round = self.round.saturating_add(1);
        self.first_player = self
            .first_player
            .saturating_add(1)
            .checked_rem(self.player_count())
            .unwrap_or(0);
        self.phase = Phase::Placement {
            player: self.first_player,
            placed: 0,
        };
        self.emit(events, Event::RoundStarted(self.round));
    }
}

#[cfg(test)]
mod tests;
