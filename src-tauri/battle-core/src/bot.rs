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

/// Насколько далеко бот смотрит. Это и есть вся сложность: бот, которому дали
/// лишнюю ману или лишнее здоровье, ломает и честность, и всякую возможность
/// измерить силу карты — победа над жуликом не говорит о карте ничего.
pub const DEPTH_MIN: u8 = 1;
/// Ступеней две, а не три, и это выбрано замером.
///
/// ```text
/// доля побед, 120 партий в обе стороны
///   глубина 1 против глубины 1   50.0 %   (проверка: та же рука)
///   глубина 2 против глубины 1  100.0 %
///   глубина 3 против глубины 1  100.0 %
///   глубина 3 против глубины 2   44.2 %   <- глубже НЕ сильнее
/// ```
///
/// Третья ступень не давала ничего: перебор, считающий противника глупее, чем
/// тот есть, с глубиной начинает ошибаться охотнее. Пробовали чинить это тем,
/// чтобы противник в ветках отвечал перебором, — стало сильнее и неприемлемо
/// медленно: ход считался секундами вместо миллисекунд.
///
/// Форма предлагает ровно то, что есть. Ручка на три положения, из которых
/// работают два, — это ровно то, что здесь чинилось.
pub const DEPTH_MAX: u8 = 2;

/// Во сколько ценится единица здоровья против единицы силы при оценке позиции.
/// Не курс баланса и не претендует им быть: это грубая мерка «кто впереди»,
/// нужная только чтобы сравнить две ветки перебора между собой.
const HEALTH_WEIGHT: i32 = 2;

/// Насколько дорого стоять далеко от врага. Мало и намеренно: без этого
/// слагаемого перебор с материальной оценкой предпочитает не двигаться вовсе —
/// шаг не меняет материала, — и две линии стоят друг напротив друга до лимита
/// кругов. Ровно та же беда, что уже ловилась однажды у жадного бота.
const APPROACH_WEIGHT: i32 = 3;

/// Чем эта позиция хороша для стороны `side`. Больше — лучше.
///
/// Исход весит на порядки больше материала, а материал — больше расстояния:
/// это не курс баланса, а грубая мерка «кто впереди», нужная только чтобы
/// сравнить две ветки перебора между собой.
fn evaluate(state: &MatchState, side: crate::board::Side) -> i32 {
    let material = |s: crate::board::Side| -> i32 {
        state
            .standing(s)
            .iter()
            .map(|id| {
                let u = &state.units[*id as usize];
                u.health.current * HEALTH_WEIGHT + u.printed_power()
            })
            .sum()
    };

    let approach: i32 = state
        .standing(side)
        .iter()
        .filter_map(|id| state.board.cell_of(*id))
        .map(|cell| nearest_enemy_for(state, side, cell) as i32)
        .sum();

    let standing = 100 * (material(side) - material(side.other())) - APPROACH_WEIGHT * approach;

    let decided = match state.outcome {
        Some(crate::event::Outcome::Draw) | None => 0,
        Some(crate::event::Outcome::Player) => {
            if side == crate::board::Side::Player { 1_000_000 } else { -1_000_000 }
        }
        Some(crate::event::Outcome::Keeper) => {
            if side == crate::board::Side::Keeper { 1_000_000 } else { -1_000_000 }
        }
    };
    decided + standing
}

/// То же, что `nearest_enemy`, но для заданной стороны, а не для ходящей:
/// перебор оценивает позиции, в которых ходит противник.
fn nearest_enemy_for(state: &MatchState, side: crate::board::Side, cell: Cell) -> u8 {
    state
        .standing(side.other())
        .iter()
        .filter_map(|id| state.board.cell_of(*id))
        .map(|other| cell.distance(other))
        .min()
        .unwrap_or(u8::MAX)
}

/// Досмотреть партию до конца обеими жадными руками и оценить, чем кончилось.
///
/// Лист перебора оценивается доигрыванием, а не статической меркой, и это не
/// украшение. Со статической меркой глубина 2 играла **слабее** жадного: мерка
/// из материала и расстояния оказалась хуже его чутья — «сначала добить»,
/// «лечить крупную рану», «ставить ближе к врагу». Доигрывание наследует это
/// чутьё целиком, и перебор может только улучшить выбор первого хода, но не
/// испортить его. Проверено замером: с этой правкой глубина растёт монотонно.
fn playout(state: &MatchState, side: crate::board::Side) -> i32 {
    let mut st = state.clone();
    let mut guard = 0;
    while st.outcome.is_none() && guard < 512 {
        let action = choose(&st);
        let Ok((next, _)) = crate::state::reduce(&st, &action) else { break };
        st = next;
        guard += 1;
    }
    evaluate(&st, side)
}

/// Ход с перебором на `depth` собственных действий вперёд.
///
/// Глубина 1 — это в точности жадный `choose`, байт в байт: так уже сыгранные
/// партии переигрываются, а прежние замеры остаются в силе.
///
/// Глубже — перебираются собственные действия, а противник между ними отвечает
/// жадно. Противник, перебирающий в ответ, был бы честнее и вчетверо дороже, а
/// разницы для трёх ступеней сложности не даёт: важно здесь не то, насколько
/// бот силён, а то, что ручка сложности наконец что-то делает.
pub fn choose_at(state: &MatchState, depth: u8) -> Action {
    if depth <= DEPTH_MIN {
        return choose(state);
    }
    let side = state.active;
    let mut best: Option<(Action, i32)> = None;
    for action in legal_actions(state) {
        let Ok((next, _)) = crate::state::reduce(state, &action) else { continue };
        let score = look(&next, side, depth.min(DEPTH_MAX) - 1);
        // Равные ветки разрешаются порядком обхода — тем же, что у жадного,
        // поэтому одна и та же позиция всегда даёт один и тот же ход.
        if best.as_ref().is_none_or(|(_, seen)| score > *seen) {
            best = Some((action, score));
        }
    }
    best.map(|(a, _)| a).unwrap_or(Action::EndTurn)
}

/// Досмотреть ветку: пока ходит противник — он отвечает жадно; когда очередь
/// возвращается, тратится единица глубины.
fn look(state: &MatchState, side: crate::board::Side, budget: u8) -> i32 {
    let mut st = state.clone();
    // Ответ противника. Ограничен счётчиком, а не «до конца хода»: партия
    // конечна, но полагаться на это внутри перебора не стоит.
    let mut guard = 0;
    while st.outcome.is_none() && st.active != side && guard < 64 {
        let reply = choose(&st);
        let Ok((next, _)) = crate::state::reduce(&st, &reply) else { break };
        st = next;
        guard += 1;
    }
    if budget == 0 || st.outcome.is_some() {
        return playout(&st, side);
    }
    let mut best = i32::MIN;
    for action in legal_actions(&st) {
        let Ok((next, _)) = crate::state::reduce(&st, &action) else { continue };
        best = best.max(look(&next, side, budget - 1));
    }
    if best == i32::MIN { playout(&st, side) } else { best }
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
            let res = state.blow(a, t);
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
    let mut best_mend: Option<(&Action, i32, i32)> = None;
    for action in &actions {
        if let Action::Mend { healer, target } = action {
            let h = &state.units[*healer as usize];
            let t = &state.units[*target as usize];
            let mana = state.side_state(state.active).mana;
            let offered = h
                .ready_heal(mana)
                .map(|a| a.amount)
                .unwrap_or(h.mend);
            let restored = crate::heal::resolve_mend(t, offered).restored;
            if best_mend.is_none_or(|(_, best, _)| restored > best) {
                best_mend = Some((action, restored, offered));
            }
        }
    }
    // Only when the mending is not mostly wasted: half of what is offered has to
    // land, or the turn is better spent on almost anything else.
    if let Some((action, restored, offered)) = best_mend {
        if offered > 0 && restored * 2 >= offered {
            return action.clone();
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
            let score = state.blow(a, t).to_health - t.health.current;
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
