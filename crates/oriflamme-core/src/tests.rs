use super::*;

/// A face-down or face-up queue card.
fn qc(owner: PlayerId, kind: CardKind, face_up: bool, influence: u32) -> QueueCard {
    QueueCard {
        card: Card { owner, kind },
        face_up,
        influence,
    }
}

/// A 3-player state in the resolution phase with the given single-card stacks.
/// Every player has 1 influence. Resolution starts at the first card.
fn board(cards: Vec<QueueCard>) -> GameState {
    let players = (0..3)
        .map(|_| Player {
            hand: Vec::new(),
            removed: Vec::new(),
            discard: Vec::new(),
            influence: 1,
        })
        .collect();
    let mut state = GameState {
        players,
        queue: cards.into_iter().map(|c| vec![c]).collect(),
        round: 2,
        first_player: 0,
        phase: Phase::GameOver,
        resolved: Vec::new(),
    };
    let mut events = Trace::default();
    state.advance(&mut events);
    state
}

/// Marks every card before `pos` as resolved and restarts resolution there.
fn board_at(cards: Vec<QueueCard>, pos: usize) -> GameState {
    let mut state = board(cards);
    state.resolved = state.queue[..pos].iter().map(|s| s[0].card).collect();
    let mut events = Trace::default();
    state.advance(&mut events);
    state
}

use CardKind::*;

#[test]
fn setup_deals_seven_cards_and_one_influence() {
    for count in MIN_PLAYERS..=MAX_PLAYERS {
        let state = GameState::new(count, &mut Rng64::new(7)).unwrap();
        assert_eq!(state.player_count(), count);
        for player in &state.players {
            assert_eq!(player.hand.len(), 7);
            assert_eq!(player.removed.len(), 3);
            assert_eq!(player.influence, 1);
            let mut all: Vec<_> = player.hand.iter().chain(&player.removed).copied().collect();
            all.sort_by_key(|k| k.slug());
            all.dedup();
            assert_eq!(all.len(), 10);
        }
    }
}

#[test]
fn setup_rejects_bad_player_counts() {
    for count in [0, 1, 2, 6] {
        assert_eq!(
            GameState::new(count, &mut Rng64::new(1)),
            Err(SetupError::InvalidPlayerCount(count))
        );
    }
}

#[test]
fn first_round_has_no_stacking_and_first_card_has_one_slot() {
    let mut state = GameState::new(3, &mut Rng64::new(3)).unwrap();
    let first = state.legal_actions();
    assert!(first.iter().all(|a| matches!(
        a,
        Action::Place {
            slot: Slot::Left,
            ..
        }
    )));
    state.apply(first[0]).unwrap();
    let second = state.legal_actions();
    assert!(second.iter().any(|a| matches!(
        a,
        Action::Place {
            slot: Slot::Right,
            ..
        }
    )));
    assert!(!second.iter().any(|a| matches!(
        a,
        Action::Place {
            slot: Slot::OnStack(_),
            ..
        }
    )));
}

#[test]
fn stacking_allowed_from_round_two_on_own_cards_only() {
    let mut state = board(vec![qc(0, Lord, true, 0), qc(1, Lord, true, 0)]);
    state.players[0].hand = vec![Spy];
    state.phase = Phase::Placement {
        player: 0,
        placed: 0,
    };
    let actions = state.legal_actions();
    assert!(actions.contains(&Action::Place {
        kind: Spy,
        slot: Slot::OnStack(0)
    }));
    assert!(!actions.contains(&Action::Place {
        kind: Spy,
        slot: Slot::OnStack(1)
    }));
}

#[test]
fn illegal_action_is_rejected_and_state_unchanged() {
    let mut state = GameState::new(3, &mut Rng64::new(3)).unwrap();
    let before = state.clone();
    assert_eq!(
        state.apply(Action::Reveal),
        Err(RuleError::IllegalAction(Action::Reveal))
    );
    assert_eq!(state, before);
}

#[test]
fn keeping_hidden_adds_influence_and_reveal_collects_it() {
    let mut state = board(vec![qc(0, Spy, false, 2)]);
    state.apply(Action::KeepHidden).unwrap();
    assert_eq!(state.queue[0][0].influence, 3);
    assert_eq!(state.players[0].influence, 1);

    let mut state = board(vec![qc(0, Spy, false, 2)]);
    state.apply(Action::Reveal).unwrap();
    assert_eq!(state.players[0].influence, 3);
    assert!(state.queue[0][0].face_up);
}

#[test]
fn archer_eliminates_first_or_last() {
    let mut state = board_at(
        vec![
            qc(1, Spy, true, 0),
            qc(0, Archer, false, 0),
            qc(2, Lord, true, 3),
        ],
        1,
    );
    state.apply(Action::Reveal).unwrap();
    assert_eq!(
        state.legal_actions(),
        vec![Action::Target(0), Action::Target(2)]
    );
    state.apply(Action::Target(2)).unwrap();
    assert_eq!(state.queue.len(), 2);
    assert_eq!(state.players[0].influence, 2);
    assert_eq!(state.players[2].discard, vec![Lord]);
    // Influence on the eliminated card is lost.
    assert_eq!(state.players[2].influence, 1);
}

#[test]
fn soldier_eliminates_adjacent() {
    let mut state = board(vec![
        qc(0, Soldier, false, 0),
        qc(1, Heir, true, 0),
        qc(2, Lord, true, 0),
    ]);
    state.apply(Action::Reveal).unwrap();
    assert_eq!(state.legal_actions(), vec![Action::Target(1)]);
    state.apply(Action::Target(1)).unwrap();
    assert_eq!(state.players[0].influence, 2);
    assert_eq!(state.players[1].discard, vec![Heir]);
}

#[test]
fn spy_steals_from_adjacent_opponent_pool() {
    let mut state = board(vec![qc(0, Spy, false, 0), qc(1, Conspiracy, false, 5)]);
    state.apply(Action::Reveal).unwrap();
    state.apply(Action::Target(1)).unwrap();
    assert_eq!(state.players[0].influence, 2);
    assert_eq!(state.players[1].influence, 0);
    // The influence on the adjacent card is untouched.
    assert_eq!(state.queue[1][0].influence, 5);
}

#[test]
fn spy_has_no_target_next_to_own_or_broke_players() {
    let mut state = board(vec![qc(0, Spy, false, 0), qc(0, Lord, false, 0)]);
    state.apply(Action::Reveal).unwrap();
    // No choice was offered: resolution moved on to the Lord.
    assert_eq!(
        state.phase,
        Phase::Resolution {
            card: Card {
                owner: 0,
                kind: Lord
            },
            decision: Decision::RevealOrNot
        }
    );
    assert_eq!(state.players[0].influence, 1);
}

#[test]
fn heir_gains_two_only_when_alone() {
    let mut state = board(vec![qc(0, Heir, false, 0)]);
    state.apply(Action::Reveal).unwrap();
    assert_eq!(state.players[0].influence, 3);

    let mut state = board(vec![qc(0, Heir, false, 0), qc(1, Heir, true, 0)]);
    state.apply(Action::Reveal).unwrap();
    // Neither Heir scores.
    assert_eq!(state.players[0].influence, 1);
    assert_eq!(state.players[1].influence, 1);
}

#[test]
fn lord_counts_adjacent_family_cards() {
    let mut state = board_at(
        vec![
            qc(0, Spy, false, 0),
            qc(0, Lord, false, 0),
            qc(0, Heir, false, 0),
        ],
        1,
    );
    state.apply(Action::Reveal).unwrap();
    assert_eq!(state.players[0].influence, 4);

    let mut state = board(vec![qc(0, Lord, false, 0), qc(1, Heir, false, 0)]);
    state.apply(Action::Reveal).unwrap();
    assert_eq!(state.players[0].influence, 2);
}

#[test]
fn assassination_eliminates_any_card_and_is_discarded() {
    let mut state = board(vec![
        qc(0, Assassination, false, 0),
        qc(1, Spy, true, 0),
        qc(2, Lord, true, 0),
    ]);
    state.apply(Action::Reveal).unwrap();
    assert_eq!(
        state.legal_actions(),
        vec![Action::Target(1), Action::Target(2)]
    );
    state.apply(Action::Target(2)).unwrap();
    assert_eq!(state.players[0].influence, 2);
    assert_eq!(state.players[0].discard, vec![Assassination]);
    assert_eq!(state.players[2].discard, vec![Lord]);
    assert_eq!(state.queue.len(), 1);
}

#[test]
fn royal_decree_moves_a_card_and_is_discarded() {
    let mut state = board(vec![
        qc(0, RoyalDecree, false, 0),
        qc(1, Spy, false, 0),
        qc(2, Lord, false, 0),
    ]);
    state.apply(Action::Reveal).unwrap();
    // Take the Lord (position 2) and insert it at the front.
    state.apply(Action::Move { from: 2, to: 0 }).unwrap();
    let owners: Vec<_> = state.queue.iter().map(|s| s[0].card.kind).collect();
    assert_eq!(owners, vec![Lord, Spy]);
    assert_eq!(state.players[0].discard, vec![RoyalDecree]);
}

#[test]
fn conspiracy_doubles_its_influence() {
    let mut state = board(vec![qc(0, Conspiracy, false, 3)]);
    state.apply(Action::Reveal).unwrap();
    assert_eq!(state.players[0].influence, 7);
    assert_eq!(state.players[0].discard, vec![Conspiracy]);
    assert!(state.queue.is_empty());
}

#[test]
fn ambush_revealed_by_owner_gains_one() {
    let mut state = board(vec![qc(0, Ambush, false, 3)]);
    state.apply(Action::Reveal).unwrap();
    assert_eq!(state.players[0].influence, 2);
    assert_eq!(state.players[0].discard, vec![Ambush]);
}

#[test]
fn ambush_hit_by_opponent_gains_four_and_discards_attacker() {
    let mut state = board(vec![qc(1, Soldier, false, 0), qc(0, Ambush, false, 2)]);
    state.apply(Action::Reveal).unwrap();
    state.apply(Action::Target(1)).unwrap();
    assert_eq!(state.players[0].influence, 5);
    assert_eq!(state.players[1].influence, 2);
    assert_eq!(state.players[0].discard, vec![Ambush]);
    assert_eq!(state.players[1].discard, vec![Soldier]);
    assert!(state.queue.is_empty());
}

#[test]
fn ambush_hit_by_own_card_gains_two_in_total() {
    let mut state = board(vec![qc(0, Soldier, false, 0), qc(0, Ambush, false, 2)]);
    state.apply(Action::Reveal).unwrap();
    state.apply(Action::Target(1)).unwrap();
    assert_eq!(state.players[0].influence, 3);
    assert!(state.queue.is_empty());
}

#[test]
fn shapeshifter_copies_adjacent_revealed_character() {
    let mut state = board_at(
        vec![
            qc(1, Soldier, true, 0),
            qc(0, Shapeshifter, false, 0),
            qc(2, Conspiracy, false, 0),
        ],
        1,
    );
    state.resolved.push(Card {
        owner: 1,
        kind: Soldier,
    });
    state.apply(Action::Reveal).unwrap();
    // Only the revealed Soldier can be copied.
    assert_eq!(state.legal_actions(), vec![Action::Target(0)]);
    state.apply(Action::Target(0)).unwrap();
    // Now acting as a Soldier: either neighbour may be eliminated.
    assert_eq!(
        state.legal_actions(),
        vec![Action::Target(0), Action::Target(2)]
    );
    state.apply(Action::Target(0)).unwrap();
    assert_eq!(state.players[0].influence, 2);
    assert_eq!(state.players[1].discard, vec![Soldier]);
}

#[test]
fn shapeshifter_copying_the_only_heir_gains_two() {
    let mut state = board_at(vec![qc(1, Heir, true, 0), qc(0, Shapeshifter, false, 0)], 1);
    state.apply(Action::Reveal).unwrap();
    state.apply(Action::Target(0)).unwrap();
    assert_eq!(state.players[0].influence, 3);
}

#[test]
fn shapeshifter_copying_heir_scores_for_the_shapeshifter_owner_only() {
    // The Heir already scored for its own owner when it resolved (1 + 2).
    let mut state = board(vec![qc(1, Heir, true, 0), qc(0, Shapeshifter, false, 0)]);
    assert_eq!(state.players[1].influence, 3);
    state.apply(Action::Reveal).unwrap();
    assert_eq!(state.legal_actions(), vec![Action::Target(0)]);
    state.apply(Action::Target(0)).unwrap();
    assert_eq!(state.players[0].influence, 3);
    assert_eq!(state.players[1].influence, 3);
    assert_eq!(state.players[2].influence, 1);
    // Nothing is discarded; both cards stay revealed in the queue.
    assert!(state.players.iter().all(|p| p.discard.is_empty()));
    assert_eq!(state.queue.len(), 2);
    assert!(state.queue.iter().all(|s| s[0].face_up));
    assert_eq!(state.round, 3);
}

#[test]
fn shapeshifter_copying_own_heir_adds_to_the_heirs_gain() {
    let mut state = board(vec![qc(0, Heir, true, 0), qc(0, Shapeshifter, false, 0)]);
    assert_eq!(state.players[0].influence, 3);
    state.apply(Action::Reveal).unwrap();
    state.apply(Action::Target(0)).unwrap();
    assert_eq!(state.players[0].influence, 5);
}

#[test]
fn shapeshifter_copying_heir_gains_nothing_when_another_heir_is_revealed() {
    // Two revealed Heirs: neither scores, and neither does a copy of one.
    let mut state = board(vec![
        qc(1, Heir, true, 0),
        qc(0, Shapeshifter, false, 0),
        qc(2, Heir, true, 0),
    ]);
    assert_eq!(state.players[1].influence, 1);
    state.apply(Action::Reveal).unwrap();
    // Both neighbouring Heirs are valid copy targets.
    assert_eq!(
        state.legal_actions(),
        vec![Action::Target(0), Action::Target(2)]
    );
    state.apply(Action::Target(2)).unwrap();
    for player in &state.players {
        assert_eq!(player.influence, 1);
    }
    assert_eq!(state.round, 3);
}

#[test]
fn face_down_heir_does_not_stop_a_shapeshifter_copy_from_scoring() {
    let mut state = board(vec![
        qc(1, Heir, true, 0),
        qc(0, Shapeshifter, false, 0),
        qc(2, Heir, false, 0),
    ]);
    // The revealed Heir was alone when it resolved.
    assert_eq!(state.players[1].influence, 3);
    state.apply(Action::Reveal).unwrap();
    // The face-down Heir is not a copy target.
    assert_eq!(state.legal_actions(), vec![Action::Target(0)]);
    state.apply(Action::Target(0)).unwrap();
    assert_eq!(state.players[0].influence, 3);
    // Revealing the last Heir now finds another revealed Heir, so it scores nothing.
    state.apply(Action::Reveal).unwrap();
    assert_eq!(state.players[2].influence, 1);
    assert_eq!(state.players[1].influence, 3);
}

#[test]
fn two_shapeshifters_around_one_heir_both_score() {
    let mut state = board(vec![
        qc(0, Shapeshifter, false, 0),
        qc(1, Heir, true, 0),
        qc(2, Shapeshifter, false, 0),
    ]);
    state.apply(Action::Reveal).unwrap();
    assert_eq!(state.legal_actions(), vec![Action::Target(1)]);
    state.apply(Action::Target(1)).unwrap();
    // The Heir resolved by itself right after the first copy.
    assert_eq!(state.players[0].influence, 3);
    assert_eq!(state.players[1].influence, 3);
    // The second Shapeshifter copies the same Heir, still the only one.
    state.apply(Action::Reveal).unwrap();
    assert_eq!(state.legal_actions(), vec![Action::Target(1)]);
    state.apply(Action::Target(1)).unwrap();
    assert_eq!(state.players[2].influence, 3);
    assert_eq!(state.players[0].influence, 3);
    assert_eq!(state.players[1].influence, 3);
    assert_eq!(state.round, 3);
}

#[test]
fn two_shapeshifters_and_two_heirs_nobody_scores() {
    let mut state = board(vec![
        qc(1, Heir, true, 0),
        qc(0, Shapeshifter, false, 0),
        qc(2, Heir, true, 0),
        qc(1, Shapeshifter, false, 0),
    ]);
    state.apply(Action::Reveal).unwrap();
    state.apply(Action::Target(0)).unwrap();
    state.apply(Action::Reveal).unwrap();
    assert_eq!(state.legal_actions(), vec![Action::Target(2)]);
    state.apply(Action::Target(2)).unwrap();
    for player in &state.players {
        assert_eq!(player.influence, 1);
    }
    assert_eq!(state.round, 3);
}

#[test]
fn adjacent_shapeshifters_cannot_copy_each_other() {
    let mut state = board(vec![
        qc(2, Lord, true, 0),
        qc(0, Shapeshifter, false, 0),
        qc(1, Shapeshifter, false, 0),
    ]);
    // The Lord resolved alone: +1 for its owner.
    assert_eq!(state.players[2].influence, 2);
    state.apply(Action::Reveal).unwrap();
    // The face-down neighbour is no target, only the Lord is.
    assert_eq!(state.legal_actions(), vec![Action::Target(0)]);
    state.apply(Action::Target(0)).unwrap();
    // Copied Lord: no friendly neighbour, so +1 only.
    assert_eq!(state.players[0].influence, 2);
    // The second Shapeshifter only has a revealed Shapeshifter beside it:
    // nothing to copy, so revealing it does nothing.
    state.apply(Action::Reveal).unwrap();
    assert_eq!(state.players[1].influence, 1);
    assert_eq!(state.players[0].influence, 2);
    assert_eq!(state.players[2].influence, 2);
    assert_eq!(state.round, 3);
}

#[test]
fn two_shapeshifters_in_a_row_with_no_character_do_nothing() {
    let mut state = board(vec![
        qc(0, Shapeshifter, false, 0),
        qc(1, Shapeshifter, false, 0),
    ]);
    state.apply(Action::Reveal).unwrap();
    state.apply(Action::Reveal).unwrap();
    for player in &state.players {
        assert_eq!(player.influence, 1);
    }
    assert!(state.queue.iter().all(|s| s[0].face_up));
    assert_eq!(state.round, 3);
}

#[test]
fn covered_card_is_inactive_until_uncovered() {
    let mut state = board(vec![qc(1, Soldier, false, 0), qc(0, Heir, true, 0)]);
    state.queue[1].push(qc(0, Spy, false, 0));
    state.apply(Action::Reveal).unwrap();
    state.apply(Action::Target(1)).unwrap();
    // The Spy was eliminated; the Heir underneath is back on top but waits a round.
    assert_eq!(state.top(1).unwrap().card.kind, Heir);
    assert_eq!(state.players[0].influence, 1);
    assert_eq!(state.round, 3);
}

#[test]
fn revealed_cards_act_again_every_round() {
    let mut state = board(vec![qc(0, Heir, true, 0)]);
    // `board` already resolved the face-up Heir once and ended the round.
    assert_eq!(state.players[0].influence, 3);
    assert_eq!(state.round, 3);
    for player in &mut state.players {
        player.hand = vec![Conspiracy];
    }
    for _ in 0..3 {
        let player = state.current_player().unwrap();
        state
            .apply(Action::Place {
                kind: Conspiracy,
                slot: Slot::Right,
            })
            .unwrap();
        assert_ne!(state.current_player(), Some(player));
    }
    assert_eq!(state.players[0].influence, 5);
}

#[test]
fn tie_is_broken_by_cards_in_queue() {
    let mut state = board(vec![qc(1, Spy, false, 0)]);
    state.phase = Phase::GameOver;
    state.players[0].influence = 9;
    state.players[1].influence = 9;
    state.players[2].influence = 4;
    assert_eq!(state.winners(), vec![1]);
    state.queue.clear();
    assert_eq!(state.winners(), vec![0, 1]);
}

#[test]
fn view_hides_opponent_secrets() {
    let mut rng = Rng64::new(11);
    let mut state = GameState::new(4, &mut rng).unwrap();
    for _ in 0..4 {
        let actions = state.legal_actions();
        state.apply(actions[0]).unwrap();
    }
    let view = state.view_for(0);
    assert_eq!(view.hand, state.players[0].hand);
    for stack in &view.queue {
        for card in stack {
            assert_eq!(card.kind.is_some(), card.owner == 0 || card.face_up);
        }
    }
}

/// Plays random games and checks the invariants the engine promises.
#[test]
fn random_games_finish_and_determinize_is_consistent() {
    for seed in 0..600_u64 {
        let mut rng = Rng64::new(seed);
        let count = 3 + (seed as usize % 3);
        let mut state = GameState::new(count, &mut rng).unwrap();
        let mut steps = 0;
        while let Some(player) = state.current_player() {
            let actions = state.legal_actions();
            assert!(!actions.is_empty(), "seed {seed}: no legal action");

            // A determinized state must look identical to the viewer.
            let view = state.view_for(player);
            let guess = view.determinize(&mut rng);
            assert_eq!(guess.view_for(player), view, "seed {seed}");
            assert_eq!(guess.legal_actions(), actions, "seed {seed}");
            for p in &guess.players {
                let total = p.hand.len() + p.removed.len() + p.discard.len();
                let queued = guess.queue.iter().flatten();
                let mine = queued
                    .filter(|qc| std::ptr::eq(&guess.players[qc.card.owner], p))
                    .count();
                assert_eq!(total + mine, 10, "seed {seed}");
            }

            let action = actions[rng.below(actions.len())];
            state.apply(action).unwrap();
            steps += 1;
            assert!(steps < 5_000, "seed {seed}: game does not end");
        }
        assert!(state.is_over());
        assert_eq!(state.round, ROUNDS);
        for p in &state.players {
            assert_eq!(p.hand.len(), 1, "seed {seed}");
        }
        assert!(!state.winners().is_empty());
    }
}

#[test]
fn traced_steps_chain_from_the_old_table_to_the_new_one() {
    let mut state = board(vec![
        qc(1, Soldier, false, 2),
        qc(0, Ambush, false, 0),
        qc(2, Lord, true, 0),
    ]);
    let start = state.snapshot();
    let mut plain = state.clone();

    state.apply(Action::Reveal).unwrap();
    let before_hit = state.snapshot();
    let steps = state.apply_traced(Action::Target(1)).unwrap();

    // Same result and same events as the untraced call.
    plain.apply(Action::Reveal).unwrap();
    let events = plain.apply(Action::Target(1)).unwrap();
    assert_eq!(plain, state);
    assert_eq!(steps.iter().map(|s| s.event).collect::<Vec<_>>(), events);

    assert_ne!(start, before_hit);
    assert_eq!(steps.first().unwrap().before, before_hit);
    assert_eq!(steps.last().unwrap().after, state.snapshot());
    for pair in steps.windows(2) {
        assert_eq!(pair[0].after, pair[1].before);
    }
    // The Ambush is still on the table in the snapshot before it is hit.
    let Event::Ambushed {
        by_pos,
        target_pos,
        gained,
        ..
    } = steps[0].event
    else {
        panic!("expected an ambush, got {:?}", steps[0].event);
    };
    assert_eq!((by_pos, target_pos, gained), (Some(0), 1, 4));
    assert_eq!(steps[0].before.queue[target_pos][0].card.kind, Ambush);
}
