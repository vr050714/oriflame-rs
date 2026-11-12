//! Rule-of-thumb play. Works on a full [`GameState`] but reads only what the
//! player to move may know: their own cards and public information.

use oriflamme_core::{
    Action, Card, CardKind, Decision, GameState, Phase, PlayerId, QueueCard, ROUNDS, Slot,
};

/// Picks an action for the current player of `state`.
///
/// `noise` is the size of a random bonus added to every score; it breaks ties
/// and makes play less predictable. Returns `None` only when `legal` is empty.
pub fn choose(
    state: &GameState,
    legal: &[Action],
    rng: &mut oriflamme_core::Rng64,
    noise: f64,
) -> Option<Action> {
    let mut best: Option<(Action, f64)> = None;
    for (action, score) in rank(state, legal) {
        let score = score + rng.unit() * noise;
        if best.is_none_or(|(_, top)| score > top) {
            best = Some((action, score));
        }
    }
    best.map(|(action, _)| action)
}

/// Scores every legal action for the current player; higher is better.
/// Scores are comparable only within one call.
#[must_use]
pub fn rank(state: &GameState, legal: &[Action]) -> Vec<(Action, f64)> {
    let Some(me) = state.current_player() else {
        return Vec::new();
    };
    legal
        .iter()
        .map(|&action| {
            let score = match action {
                Action::Place { kind, slot } => place_score(state, me, kind, slot),
                Action::Reveal => {
                    if wants_reveal(state) {
                        1.0
                    } else {
                        0.0
                    }
                }
                Action::KeepHidden => 0.5,
                Action::Target(_) | Action::Move { .. } => lookahead(state, me, action, 2),
            };
            (action, score)
        })
        .collect()
}

/// Rounds still to be resolved after the current one.
fn rounds_left(state: &GameState) -> f64 {
    f64::from(ROUNDS.saturating_sub(state.round))
}

fn is_earner(kind: CardKind) -> bool {
    matches!(kind, CardKind::Heir | CardKind::Lord)
}

/// Top cards next to queue position `pos`.
fn neighbours(state: &GameState, pos: usize) -> impl Iterator<Item = &QueueCard> {
    let left = pos.checked_sub(1).and_then(|p| state.top(p));
    let right = state.top(pos.saturating_add(1));
    left.into_iter().chain(right)
}

fn revealed_heir_on_board(state: &GameState) -> bool {
    state
        .queue
        .iter()
        .filter_map(|stack| stack.last())
        .any(|qc| qc.face_up && qc.card.kind == CardKind::Heir)
}

/// Position value for `me`: influence lead plus what the board promises.
#[must_use]
pub fn evaluate(state: &GameState, me: PlayerId) -> f64 {
    let scores = state.scores();
    let mine = scores.get(me).copied().map_or(0.0, f64::from);
    let others: Vec<f64> = scores
        .iter()
        .enumerate()
        .filter(|&(p, _)| p != me)
        .map(|(_, &s)| f64::from(s))
        .collect();
    let opponents = others.len().max(1) as f64;
    let max_other = others.iter().copied().fold(0.0, f64::max);
    let mean_other = others.iter().sum::<f64>() / opponents;

    let future = rounds_left(state).min(2.0) * 0.5;
    let mut my_board = 0.0;
    let mut their_board = 0.0;
    for qc in state.queue.iter().filter_map(|stack| stack.last()) {
        let value = if qc.face_up {
            future
                * match qc.card.kind {
                    CardKind::Heir => 2.0,
                    CardKind::Lord => 1.5,
                    CardKind::Soldier | CardKind::Archer => 0.8,
                    CardKind::Spy => 0.7,
                    _ => 0.4,
                }
        } else {
            0.5 + 0.5 * f64::from(qc.influence)
        };
        if qc.card.owner == me {
            my_board += value;
        } else {
            their_board += value;
        }
    }
    mine - 0.5 * max_other - 0.5 * mean_other + my_board - their_board / opponents
}

/// Value of a targeting action: apply it, finish any follow-up choice of the
/// same player greedily, then evaluate.
fn lookahead(state: &GameState, me: PlayerId, action: Action, depth: u8) -> f64 {
    let mut next = state.clone();
    if next.apply(action).is_err() {
        return f64::NEG_INFINITY;
    }
    let follow_up = matches!(
        next.phase,
        Phase::Resolution { card, decision: Decision::Ability(_) } if card.owner == me
    );
    if follow_up && depth > 0 {
        return next
            .legal_actions()
            .into_iter()
            .map(|a| lookahead(&next, me, a, depth.saturating_sub(1)))
            .fold(f64::NEG_INFINITY, f64::max);
    }
    evaluate(&next, me)
}

/// Should the current face-down card be revealed now?
fn wants_reveal(state: &GameState) -> bool {
    let Phase::Resolution {
        card,
        decision: Decision::RevealOrNot,
    } = state.phase
    else {
        return false;
    };
    let Some(pos) = state.position_of(card) else {
        return false;
    };
    let me = card.owner;
    let on_card = state.top(pos).map_or(0, |qc| qc.influence);
    // Influence left on cards at the end is worth nothing.
    if state.round >= ROUNDS {
        return true;
    }
    let late = state.round >= 4;
    let worth_hitting = |qc: &QueueCard| qc.card.owner != me && (qc.face_up || late);
    match card.kind {
        CardKind::Lord => true,
        CardKind::Heir => !revealed_heir_on_board(state),
        CardKind::Conspiracy => on_card >= 3 || (state.round >= 5 && on_card >= 2),
        CardKind::Ambush => false,
        CardKind::Spy => neighbours(state, pos).any(|qc| {
            qc.card.owner != me
                && state
                    .players
                    .get(qc.card.owner)
                    .is_some_and(|p| p.influence > 0)
        }),
        CardKind::Soldier => neighbours(state, pos).any(worth_hitting),
        CardKind::Archer => {
            let last = state.queue.len().saturating_sub(1);
            [0, last]
                .into_iter()
                .filter(|&end| end != pos)
                .filter_map(|end| state.top(end))
                .any(worth_hitting)
        }
        CardKind::Assassination => state
            .queue
            .iter()
            .filter_map(|stack| stack.last())
            .any(|qc| {
                qc.card.owner != me && ((qc.face_up && is_earner(qc.card.kind)) || state.round >= 5)
            }),
        CardKind::RoyalDecree => on_card >= 2 || state.round >= 5,
        CardKind::Shapeshifter => {
            neighbours(state, pos).any(|qc| qc.face_up && qc.card.kind.is_copyable())
        }
    }
}

/// Value of placing `kind` at `slot`.
fn place_score(state: &GameState, me: PlayerId, kind: CardKind, slot: Slot) -> f64 {
    let len = state.queue.len();
    // Cards that will be next to the new card, and the card it would cover.
    let (around, covered): (Vec<&QueueCard>, Option<&QueueCard>) = match slot {
        Slot::Left => (state.top(0).into_iter().collect(), None),
        Slot::Right => (
            len.checked_sub(1)
                .and_then(|p| state.top(p))
                .into_iter()
                .collect(),
            None,
        ),
        Slot::OnStack(pos) => (neighbours(state, pos).collect(), state.top(pos)),
    };
    // The card at the other end of the queue, for the Archer.
    let far_end = match slot {
        Slot::Left => len.checked_sub(1).and_then(|p| state.top(p)),
        Slot::Right => state.top(0),
        Slot::OnStack(_) => None,
    };

    let left = rounds_left(state);
    let turns = left + 1.0;
    let own_adjacent = around.iter().filter(|qc| qc.card.owner == me).count() as f64;
    let enemy_adjacent = around.iter().any(|qc| qc.card.owner != me);
    let enemy_earner_adjacent = around
        .iter()
        .any(|qc| qc.card.owner != me && qc.face_up && is_earner(qc.card.kind));
    let enemy_earner_anywhere = state
        .queue
        .iter()
        .filter_map(|stack| stack.last())
        .any(|qc| qc.card.owner != me && qc.face_up && is_earner(qc.card.kind));

    let mut score = match kind {
        CardKind::Heir => {
            if revealed_heir_on_board(state) {
                1.0
            } else {
                1.2 * turns
            }
        }
        CardKind::Lord => 0.6 * (1.0 + own_adjacent) * turns,
        CardKind::Conspiracy => 1.6 * left.min(3.0),
        CardKind::Ambush => {
            let bait = if enemy_adjacent { 1.0 } else { 0.0 };
            if left >= 1.0 { 2.5 + bait } else { 1.0 }
        }
        CardKind::Spy => {
            if enemy_adjacent {
                0.8 * turns
            } else {
                0.2
            }
        }
        CardKind::Soldier => {
            if enemy_earner_adjacent {
                4.5
            } else if enemy_adjacent {
                1.5
            } else {
                0.3
            }
        }
        CardKind::Archer => match far_end {
            Some(qc) if qc.card.owner != me && qc.face_up && is_earner(qc.card.kind) => 4.0,
            Some(qc) if qc.card.owner != me => 1.5,
            _ => 0.5,
        },
        CardKind::Assassination => {
            if enemy_earner_anywhere {
                5.0
            } else {
                2.0
            }
        }
        CardKind::RoyalDecree => 0.5,
        CardKind::Shapeshifter => {
            if around
                .iter()
                .any(|qc| qc.face_up && qc.card.kind.is_copyable())
            {
                2.5
            } else {
                0.5
            }
        }
    };

    // Covering a card switches it off.
    if let Some(qc) = covered {
        score -= if qc.face_up && is_earner(qc.card.kind) {
            4.0
        } else if qc.face_up {
            0.3
        } else {
            1.0 + 0.5 * f64::from(qc.influence)
        };
    }
    score
}

/// The card being resolved, if any. Exposed for callers that explain bot moves.
#[must_use]
pub fn resolving_card(state: &GameState) -> Option<Card> {
    match state.phase {
        Phase::Resolution { card, .. } => Some(card),
        _ => None,
    }
}
