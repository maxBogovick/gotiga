//! The keeper's hand at the table.
//!
//! A separate function over the same state, not a part of the rules — it only
//! chooses among what `legal_actions` already allows. Two things it must never
//! be given: sight of what the player holds, and better numbers than the cards
//! say. Difficulty here is depth of search and nothing else, because a win over
//! a cheating opponent says nothing about the strength of a card, and measuring
//! that is half of why this engine exists.
//!
//! This is the greedy one. Later it looks two or three turns ahead by calling
//! the very same `reduce` — which it can only do because `reduce` is pure.

use crate::board::Cell;
use crate::damage::strike;
use crate::state::{Action, MatchState, legal_actions};

/// How far this cell is from the closest standing enemy. `u8::MAX` when there
/// is none left to measure against.
fn nearest_enemy(state: &MatchState, cell: Cell) -> u8 {
    state
        .standing(state.active.other())
        .iter()
        .filter_map(|id| state.board.cell_of(*id))
        .map(|other| cell.distance(other))
        .min()
        .unwrap_or(u8::MAX)
}

/// One action, chosen. Ties are broken by the order `legal_actions` returns —
/// the field's scan order — so the same position always yields the same move.
pub fn choose(state: &MatchState) -> Action {
    let actions = legal_actions(state);

    // 1. A blow that finishes a body. Nothing else is worth more this turn.
    let mut killing: Option<(&Action, i32)> = None;
    for action in &actions {
        if let Action::Attack { attacker, target } = action {
            let a = &state.units[*attacker as usize];
            let t = &state.units[*target as usize];
            let res = strike(a, t);
            if res.to_health >= t.health.current {
                let score = t.power * 10 + t.health.current;
                if killing.is_none_or(|(_, best)| score > best) {
                    killing = Some((action, score));
                }
            }
        }
    }
    if let Some((action, _)) = killing {
        return action.clone();
    }

    // 2. Mend the worst wound within reach. Placed above putting a new body on
    //    the field because a body already standing is worth more than one that
    //    arrives unable to swing — and below the killing blow, because a corpse
    //    needs no mending.
    let mut best_mend: Option<(&Action, i32)> = None;
    for action in &actions {
        if let Action::Mend { healer, target } = action {
            let h = &state.units[*healer as usize];
            let t = &state.units[*target as usize];
            let restored = crate::heal::resolve_mend(t, h.mend).restored;
            if best_mend.is_none_or(|(_, best)| restored > best) {
                best_mend = Some((action, restored));
            }
        }
    }
    // Only when the mending is not mostly wasted: half of what is offered has to
    // land, or the turn is better spent on almost anything else.
    if let Some((action, restored)) = best_mend {
        if let Action::Mend { healer, .. } = action {
            if restored * 2 >= state.units[*healer as usize].mend {
                return action.clone();
            }
        }
    }

    // 3. Put a body on the field while there is mana for it: the field is what
    //    wins, and mana left unspent at the end of a turn is simply lost.
    //
    //    Where matters as much as what. Nothing moves in this slice, so a body
    //    placed in the back rank can never reach anything and the match runs to
    //    the round limit with both sides staring at each other — which is
    //    exactly what the first bot did before this was written down. It places
    //    towards the enemy, and among equal cells keeps the field's scan order.
    let mut best_play: Option<(&Action, (i32, u8))> = None;
    for action in &actions {
        if let Action::Play { hand_index, cell } = action {
            let card = &state.side_state(state.active).hand[*hand_index];
            let rank = (card.cost, u8::MAX - nearest_enemy(state, *cell));
            if best_play.is_none_or(|(_, best)| rank > best) {
                best_play = Some((action, rank));
            }
        }
    }
    if let Some((action, _)) = best_play {
        return action.clone();
    }

    // 4. Otherwise wound whoever is closest to falling.
    let mut best_hit: Option<(&Action, i32)> = None;
    for action in &actions {
        if let Action::Attack { attacker, target } = action {
            let a = &state.units[*attacker as usize];
            let t = &state.units[*target as usize];
            let score = strike(a, t).to_health - t.health.current;
            if best_hit.is_none_or(|(_, best)| score > best) {
                best_hit = Some((action, score));
            }
        }
    }
    if let Some((action, _)) = best_hit {
        return action.clone();
    }

    // 5. Nothing within reach: walk towards whoever is nearest.
    //
    //    This is the verb that makes the field a field. Before it existed, two
    //    lines that could not touch each other simply stood there until the
    //    round limit and the match was decided on leftover health — which is
    //    not a game, it is a stalemate with a scoreboard.
    let mut best_step: Option<(&Action, u8)> = None;
    for action in &actions {
        if let Action::Move { unit, to } = action {
            let Some(from) = state.board.cell_of(*unit) else { continue };
            let now = nearest_enemy(state, from);
            let after = nearest_enemy(state, *to);
            // Only a step that closes the gap. Otherwise a body would shuffle
            // sideways for ever, which is a legal action and a wasted turn.
            if after < now && best_step.is_none_or(|(_, best)| after < best) {
                best_step = Some((action, after));
            }
        }
    }
    if let Some((action, _)) = best_step {
        return action.clone();
    }

    Action::EndTurn
}
