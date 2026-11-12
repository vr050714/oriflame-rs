//! Monte Carlo search over determinized games.

use crate::{Strategy, heuristic};
use oriflamme_core::{Action, GameState, PlayerId, PlayerView, Rng64};

/// Searches by simulation.
///
/// Each iteration fills in the hidden cards at random (consistent with the
/// player's view), plays one candidate action, finishes the game with the
/// heuristic policy, and records the result. Candidates are sampled with UCB1,
/// so promising actions get more playouts. The work is bounded by
/// `iterations`, so strength does not depend on machine speed.
#[derive(Clone, Copy, Debug)]
pub struct MctsBot {
    /// Number of playouts per decision.
    pub iterations: u32,
    /// At most this many actions are searched; the rest are cut by heuristic score.
    pub max_candidates: usize,
    /// UCB1 exploration weight.
    pub exploration: f64,
    /// Random bonus added to heuristic scores during playouts.
    pub playout_noise: f64,
}

impl Default for MctsBot {
    fn default() -> Self {
        Self {
            iterations: 600,
            max_candidates: 10,
            exploration: 3.0,
            playout_noise: 1.0,
        }
    }
}

impl MctsBot {
    /// A bot with the given number of playouts per decision.
    #[must_use]
    pub fn with_iterations(iterations: u32) -> Self {
        Self {
            iterations,
            ..Self::default()
        }
    }

    /// The actions worth searching: all of them, or the best by heuristic.
    fn candidates(&self, view: &PlayerView, legal: &[Action], rng: &mut Rng64) -> Vec<Action> {
        if legal.len() <= self.max_candidates {
            return legal.to_vec();
        }
        let state = view.determinize(rng);
        let mut ranked = heuristic::rank(&state, legal);
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
        ranked
            .into_iter()
            .take(self.max_candidates)
            .map(|(action, _)| action)
            .collect()
    }

    /// Plays `state` to the end and returns the result for `me`.
    fn playout(&self, mut state: GameState, me: PlayerId, rng: &mut Rng64) -> f64 {
        // A game has well under a thousand decisions; the cap guards the loop.
        for _ in 0..2_000 {
            if state.is_over() {
                break;
            }
            let legal = state.legal_actions();
            let Some(action) = heuristic::choose(&state, &legal, rng, self.playout_noise) else {
                break;
            };
            if state.apply(action).is_err() {
                break;
            }
        }
        reward(&state, me)
    }
}

/// Influence margin over the best opponent, plus a bonus for winning.
fn reward(state: &GameState, me: PlayerId) -> f64 {
    let scores = state.scores();
    let mine = scores.get(me).copied().map_or(0.0, f64::from);
    let best_other = scores
        .iter()
        .enumerate()
        .filter(|&(p, _)| p != me)
        .map(|(_, &s)| f64::from(s))
        .fold(0.0, f64::max);
    let bonus = if state.winners().contains(&me) {
        3.0
    } else {
        0.0
    };
    mine - best_other + bonus
}

impl Strategy for MctsBot {
    fn name(&self) -> &'static str {
        "Monte Carlo"
    }

    fn choose(&mut self, view: &PlayerView, legal: &[Action], rng: &mut Rng64) -> Option<Action> {
        if legal.len() <= 1 {
            return legal.first().copied();
        }
        let candidates = self.candidates(view, legal, rng);
        // (action, playouts, total reward)
        let mut arms: Vec<(Action, f64, f64)> = candidates
            .into_iter()
            .map(|action| (action, 0.0, 0.0))
            .collect();

        for done in 0..self.iterations {
            let log_total = f64::from(done.max(1)).ln();
            let ucb = |&(_, visits, total): &(Action, f64, f64)| {
                if visits == 0.0 {
                    f64::INFINITY
                } else {
                    total / visits + self.exploration * (log_total / visits).sqrt()
                }
            };
            let Some(arm) = arms.iter_mut().max_by(|a, b| ucb(a).total_cmp(&ucb(b))) else {
                break;
            };
            let mut state = view.determinize(rng);
            let result = match state.apply(arm.0) {
                Ok(_) => self.playout(state, view.me, rng),
                Err(_) => -100.0,
            };
            arm.1 += 1.0;
            arm.2 += result;
        }

        arms.iter()
            .filter(|arm| arm.1 > 0.0)
            .max_by(|a, b| (a.2 / a.1).total_cmp(&(b.2 / b.1)))
            .map(|arm| arm.0)
            .or_else(|| legal.first().copied())
    }
}
