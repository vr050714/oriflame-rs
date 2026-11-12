//! What one player is allowed to know about a game.

use crate::{Card, CardKind, Decision, GameState, Phase, Player, PlayerId, QueueCard, Rng64};

/// A queue card as seen by one player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewCard {
    pub owner: PlayerId,
    /// `None` for an opponent's face-down card.
    pub kind: Option<CardKind>,
    pub face_up: bool,
    pub influence: u32,
    /// Already handled in the current resolution phase.
    pub resolved: bool,
}

/// Current step of the game without hidden card identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewPhase {
    Placement {
        player: PlayerId,
        placed: usize,
    },
    /// The top card at queue position `pos` is being resolved by `owner`.
    Resolution {
        owner: PlayerId,
        pos: usize,
        decision: Decision,
    },
    GameOver,
}

/// The information available to player `me`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerView {
    pub me: PlayerId,
    pub hand: Vec<CardKind>,
    /// The viewer's own removed cards (a player may look at them).
    pub removed: Vec<CardKind>,
    pub influence: Vec<u32>,
    pub hand_sizes: Vec<usize>,
    /// Face-up discarded and eliminated cards of every player.
    pub discards: Vec<Vec<CardKind>>,
    pub queue: Vec<Vec<ViewCard>>,
    pub round: u8,
    pub first_player: PlayerId,
    pub phase: ViewPhase,
}

impl GameState {
    /// The game as seen by `me`: opponents' hands, removed cards and face-down
    /// cards are hidden.
    #[must_use]
    pub fn view_for(&self, me: PlayerId) -> PlayerView {
        let queue = self
            .queue
            .iter()
            .map(|stack| {
                stack
                    .iter()
                    .map(|qc| ViewCard {
                        owner: qc.card.owner,
                        kind: (qc.face_up || qc.card.owner == me).then_some(qc.card.kind),
                        face_up: qc.face_up,
                        influence: qc.influence,
                        resolved: self.resolved.contains(&qc.card),
                    })
                    .collect()
            })
            .collect();
        let phase = match self.phase {
            Phase::Placement { player, placed } => ViewPhase::Placement { player, placed },
            Phase::Resolution { card, decision } => match self.position_of(card) {
                Some(pos) => ViewPhase::Resolution {
                    owner: card.owner,
                    pos,
                    decision,
                },
                None => ViewPhase::GameOver,
            },
            Phase::GameOver => ViewPhase::GameOver,
        };
        let mine = self.players.get(me);
        PlayerView {
            me,
            hand: mine.map(|p| p.hand.clone()).unwrap_or_default(),
            removed: mine.map(|p| p.removed.clone()).unwrap_or_default(),
            influence: self.players.iter().map(|p| p.influence).collect(),
            hand_sizes: self.players.iter().map(|p| p.hand.len()).collect(),
            discards: self.players.iter().map(|p| p.discard.clone()).collect(),
            queue,
            round: self.round,
            first_player: self.first_player,
            phase,
        }
    }
}

impl PlayerView {
    /// The player who must act, or `None` when the game is over.
    #[must_use]
    pub fn current_player(&self) -> Option<PlayerId> {
        match self.phase {
            ViewPhase::Placement { player, .. } => Some(player),
            ViewPhase::Resolution { owner, .. } => Some(owner),
            ViewPhase::GameOver => None,
        }
    }

    /// Builds a complete game state that agrees with everything this view
    /// shows. Hidden cards of the other players are filled in at random from
    /// the kinds that player has not shown yet.
    #[must_use]
    pub fn determinize(&self, rng: &mut Rng64) -> GameState {
        let count = self.influence.len();
        // Kinds each player could still hold in a hidden place.
        let mut unknown: Vec<Vec<CardKind>> = (0..count)
            .map(|p| {
                let mut kinds: Vec<CardKind> = CardKind::ALL
                    .into_iter()
                    .filter(|kind| {
                        let discarded = self.discards.get(p).is_some_and(|d| d.contains(kind));
                        let visible = self
                            .queue
                            .iter()
                            .flatten()
                            .any(|vc| vc.owner == p && vc.kind == Some(*kind));
                        !discarded && !visible
                    })
                    .collect();
                rng.shuffle(&mut kinds);
                kinds
            })
            .collect();

        let mut resolved = Vec::new();
        let queue: Vec<Vec<QueueCard>> = self
            .queue
            .iter()
            .map(|stack| {
                stack
                    .iter()
                    .filter_map(|vc| {
                        let kind = match vc.kind {
                            Some(kind) => kind,
                            None => unknown.get_mut(vc.owner)?.pop()?,
                        };
                        let card = Card {
                            owner: vc.owner,
                            kind,
                        };
                        if vc.resolved {
                            resolved.push(card);
                        }
                        Some(QueueCard {
                            card,
                            face_up: vc.face_up,
                            influence: vc.influence,
                        })
                    })
                    .collect()
            })
            .collect();

        let players = (0..count)
            .map(|p| {
                let influence = self.influence.get(p).copied().unwrap_or(0);
                let discard = self.discards.get(p).cloned().unwrap_or_default();
                if p == self.me {
                    return Player {
                        hand: self.hand.clone(),
                        removed: self.removed.clone(),
                        discard,
                        influence,
                    };
                }
                let mut rest = unknown.get_mut(p).map(std::mem::take).unwrap_or_default();
                let hand_size = self.hand_sizes.get(p).copied().unwrap_or(0).min(rest.len());
                let removed = rest.split_off(hand_size);
                Player {
                    hand: rest,
                    removed,
                    discard,
                    influence,
                }
            })
            .collect();

        let phase = match self.phase {
            ViewPhase::Placement { player, placed } => Phase::Placement { player, placed },
            ViewPhase::Resolution { pos, decision, .. } => {
                match queue
                    .get(pos)
                    .and_then(|stack: &Vec<QueueCard>| stack.last())
                {
                    Some(qc) => Phase::Resolution {
                        card: qc.card,
                        decision,
                    },
                    None => Phase::GameOver,
                }
            }
            ViewPhase::GameOver => Phase::GameOver,
        };

        GameState {
            players,
            queue,
            round: self.round,
            first_player: self.first_player,
            phase,
            resolved,
        }
    }
}
