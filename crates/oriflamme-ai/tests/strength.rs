#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
//! Headless games between the bots.

use oriflamme_ai::{HeuristicBot, MctsBot, RandomBot, Strategy};
use oriflamme_core::{GameState, Rng64};

/// Plays one game; `bots[p]` controls player `p`. Returns the winners.
fn play(bots: &mut [Box<dyn Strategy>], seed: u64) -> Vec<usize> {
    let mut rng = Rng64::new(seed);
    let mut state = GameState::new(bots.len(), &mut rng).unwrap();
    while let Some(player) = state.current_player() {
        let view = state.view_for(player);
        let legal = state.legal_actions();
        let action = bots[player].choose(&view, &legal, &mut rng).unwrap();
        state.apply(action).unwrap();
    }
    state.winners()
}

/// Share of games won by `hero` in seat 0 against copies of `villain`.
fn win_rate(
    hero: impl Fn() -> Box<dyn Strategy>,
    villain: impl Fn() -> Box<dyn Strategy>,
    players: usize,
    games: u64,
) -> f64 {
    let mut wins = 0.0;
    for seed in 0..games {
        let mut bots: Vec<Box<dyn Strategy>> = vec![hero()];
        bots.extend((1..players).map(|_| villain()));
        let winners = play(&mut bots, seed);
        if winners.contains(&0) {
            wins += 1.0 / winners.len() as f64;
        }
    }
    wins / games as f64
}

#[test]
fn every_bot_finishes_games_at_every_player_count() {
    for players in 3..=5 {
        for seed in 0..5 {
            let mut bots: Vec<Box<dyn Strategy>> = vec![
                Box::new(RandomBot),
                Box::new(HeuristicBot),
                Box::new(MctsBot::with_iterations(40)),
            ];
            bots.extend((3..players).map(|_| Box::new(HeuristicBot) as Box<dyn Strategy>));
            assert!(!play(&mut bots, seed).is_empty());
        }
    }
}

#[test]
fn heuristic_beats_random() {
    let rate = win_rate(|| Box::new(HeuristicBot), || Box::new(RandomBot), 4, 300);
    println!("heuristic vs 3 random: {rate:.2}");
    // An even field would win 0.25.
    assert!(rate > 0.45, "win rate {rate}");
}

#[test]
fn search_beats_heuristic() {
    let rate = win_rate(
        || Box::new(MctsBot::with_iterations(300)),
        || Box::new(HeuristicBot),
        3,
        40,
    );
    println!("search vs 2 heuristic: {rate:.2}");
    // An even field would win 0.33.
    assert!(rate > 0.45, "win rate {rate}");
}
