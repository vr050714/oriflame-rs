//! Computer-player strategies for Oriflamme.
//!
//! Every strategy implements [`Strategy`] and decides from a [`PlayerView`]
//! only, so a bot never sees hidden cards. Three strategies are provided:
//!
//! - [`RandomBot`]: uniform random legal move.
//! - [`HeuristicBot`]: hand-written rules plus a one-move lookahead.
//! - [`MctsBot`]: Monte Carlo search over random completions of the hidden
//!   information, with the heuristic as playout policy.
//!
//! ```
//! use oriflamme_ai::{Difficulty, make_strategy};
//! use oriflamme_core::{GameState, Rng64};
//!
//! let mut rng = Rng64::new(1);
//! let mut state = GameState::new(3, &mut rng)?;
//! let mut bot = make_strategy(Difficulty::Medium);
//! while let Some(player) = state.current_player() {
//!     let view = state.view_for(player);
//!     let Some(action) = bot.choose(&view, &state.legal_actions(), &mut rng) else { break };
//!     state.apply(action)?;
//! }
//! assert!(state.is_over());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod heuristic;
mod mcts;

pub use mcts::MctsBot;

use oriflamme_core::{Action, PlayerView, Rng64};

/// A computer player.
pub trait Strategy: Send {
    /// Short display name.
    fn name(&self) -> &'static str;

    /// Picks one of `legal` for the player the view belongs to.
    /// Returns `None` only when `legal` is empty.
    fn choose(&mut self, view: &PlayerView, legal: &[Action], rng: &mut Rng64) -> Option<Action>;
}

/// Plays a uniformly random legal action.
#[derive(Clone, Copy, Debug, Default)]
pub struct RandomBot;

impl Strategy for RandomBot {
    fn name(&self) -> &'static str {
        "Random"
    }

    fn choose(&mut self, _view: &PlayerView, legal: &[Action], rng: &mut Rng64) -> Option<Action> {
        legal.get(rng.below(legal.len())).copied()
    }
}

/// Plays by the rules of thumb in [`heuristic`].
#[derive(Clone, Copy, Debug, Default)]
pub struct HeuristicBot;

impl Strategy for HeuristicBot {
    fn name(&self) -> &'static str {
        "Heuristic"
    }

    fn choose(&mut self, view: &PlayerView, legal: &[Action], rng: &mut Rng64) -> Option<Action> {
        // The heuristic reads only the mover's own cards and public cards, so
        // the random fill of hidden cards matters only for lookahead surprises.
        let state = view.determinize(rng);
        heuristic::choose(&state, legal, rng, 0.3)
    }
}

/// Playing strength of a computer player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Difficulty {
    Easy,
    #[default]
    Medium,
    Hard,
}

impl Difficulty {
    /// Every difficulty, weakest first.
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];

    /// Display name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Medium => "Medium",
            Difficulty::Hard => "Hard",
        }
    }
}

/// Creates the strategy for a difficulty level.
#[must_use]
pub fn make_strategy(difficulty: Difficulty) -> Box<dyn Strategy> {
    match difficulty {
        Difficulty::Easy => Box::new(RandomBot),
        Difficulty::Medium => Box::new(HeuristicBot),
        Difficulty::Hard => Box::new(MctsBot::default()),
    }
}
