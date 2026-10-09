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

/// Сколько стоит войти в болото сверх обычного шага. Болото кончает ход, и
/// путь через него дороже ровно на то, что тело в нём простоит.
const MIRE_TOLL: u16 = 2;

/// Яма кончает ход, как болото, и ещё ранит — значит, дороже на её урон.
const PIT_TOLL: u16 = MIRE_TOLL + crate::state::PIT_HARM as u16;

/// Сколько сверх шага стоит вход на эту землю.
fn toll(state: &MatchState, cell: Cell) -> u16 {
    match state.ground(cell) {
        Some(crate::state::Ground::Mire) => MIRE_TOLL,
        Some(crate::state::Ground::Pit) => PIT_TOLL,
        _ => 0,
    }
}

/// Недостижимо.
const FAR: u16 = u16::MAX;

fn slot(cell: Cell) -> usize {
    cell.y as usize * crate::board::MAX_WIDTH as usize + cell.x as usize
}

/// Сколько шагов от каждой клетки до ближайшего из `sources` — по земле, а не
/// по прямой.
///
/// Стена и чужое тело не пропускают; `through` — клетка самого идущего, она
/// для него не помеха. Вход в болото и в яму стоит дороже, кроме самой цели.
/// Овраг пропускает только того, кто его перепрыгивает (`leaps`: шаг от двух),
/// и встать на нём нельзя — он бывает серединой пути, но не его концом. Поле — восемнадцать клеток, поэтому это не очередь с приоритетом, а
/// честное повторение, пока числа не перестанут меняться: дешевле написать и
/// невозможно ошибиться.
fn walk_costs(
    state: &MatchState,
    sources: &[Cell],
    through: Option<Cell>,
    leaps: bool,
) -> [u16; crate::board::MAX_CELLS] {
    use crate::state::Ground;
    let mut cost = [FAR; crate::board::MAX_CELLS];
    for s in sources {
        cost[slot(*s)] = 0;
    }
    let mut changed = true;
    while changed {
        changed = false;
        for cell in state.field.cells() {
            let here = cost[slot(cell)];
            if here == FAR {
                continue;
            }
            // Идущий шагает из соседней клетки СЮДА: платит за вход сюда.
            let enter = 1 + if here > 0 { toll(state, cell) } else { 0 };
            for n in state.field.neighbours(cell) {
                let ground = state.ground(n);
                let passable = ground != Some(Ground::Wall)
                    && (leaps || ground != Some(Ground::Ravine))
                    && (state.board.is_free(n) || Some(n) == through);
                if passable && here + enter < cost[slot(n)] {
                    cost[slot(n)] = here + enter;
                    changed = true;
                }
            }
        }
    }
    cost
}

/// Расстояние по земле, а где земли не хватило — по прямой, но всегда дальше
/// любого пути: отрезанное стеной тело не стоит, а тянется хотя бы ближе.
fn ground_or_line(cost: &[u16; crate::board::MAX_CELLS], cell: Cell, line: u8) -> u16 {
    let c = cost[slot(cell)];
    if c == FAR { 1000 + line as u16 } else { c }
}

/// Чего стоит ВСТАТЬ на эту клетку и идти от неё дальше. `walk_costs` берёт
/// плату за вход в топь с соседа, с которого в неё шагают, — значит, у самой
/// топи её цена входа не записана, и шаг в топь без этой прибавки выглядел бы
/// дешёвым.
fn landing(state: &MatchState, cost: &[u16; crate::board::MAX_CELLS], cell: Cell, line: u8) -> u16 {
    ground_or_line(cost, cell, line) + toll(state, cell)
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

/// Сколько здоровья снимет это действие, если оно вообще снимает.
///
/// Удар и чара считаются ОДНОЙ функцией, и это не опрятность: у жадной руки
/// есть два правила про урон — «сначала добить» и «бить того, кто ближе всех к
/// падению», — и пока чара в них не входила, хранитель с дальнобойной чарой
/// подходил ею вплотную и бил кулаком.
fn wound_of(state: &MatchState, action: &Action) -> Option<(i32, crate::unit::UnitId)> {
    match action {
        Action::Attack { attacker, target } => {
            let a = &state.units[*attacker as usize];
            let t = &state.units[*target as usize];
            Some((state.blow(a, t).to_health, *target))
        }
        Action::Cast {
            caster,
            ability,
            target,
        } => {
            let c = &state.units[*caster as usize];
            let a = c.ability_by_key(ability)?;
            if !a.casting()?.wounds() {
                return None;
            }
            // Урон наводят на тело; клеточные чары урона не наносят вовсе.
            let t = &state.units[target.unit()? as usize];
            let res = crate::damage::resolve(
                Some(c),
                t,
                crate::damage::DamagePacket::new(
                    a.amount,
                    a.channel,
                    crate::damage::Source::Ability,
                ),
            );
            Some((res.to_health, t.id))
        }
        _ => None,
    }
}

/// Чего стоит наведённый всадник или щит — грубо и нарочно грубо.
///
/// Перебор глубины 2 оценит чару сам: проклятие силы видно в `evaluate` через
/// `printed_power`. Жадной руке нужно только одно — не стоять, имея в руках
/// проклятие, и выбирать между тремя чарами всегда одинаково.
fn rider_worth(state: &MatchState, action: &Action) -> Option<i32> {
    let Action::Cast {
        caster, ability, ..
    } = action
    else {
        return None;
    };
    let c = &state.units[*caster as usize];
    let a = c.ability_by_key(ability)?;
    let what = a.casting()?;
    if what.wounds() {
        return None;
    }
    // Три глагола жадная рука не берёт вовсе, и это не пробел, а отказ: их
    // польза не выводится из числа на умении. Жертва ОТДАЁТ тело — с одним
    // числом в руках бот отдавал бы их одно за другим; глоток маны на полном
    // запасе не даёт ничего и тратит ход; толчок меняет расстановку, а
    // расстановку жадная рука не оценивает ничем. Перебор глубины 2 их всё
    // равно рассмотрит — он смотрит на доску после, а не на число до.
    if matches!(
        what,
        crate::spell::Casting::Offer | crate::spell::Casting::Coin | crate::spell::Casting::Shove
    ) {
        return None;
    }
    Some(a.amount * a.duration.max(1) as i32)
}

/// One action, chosen. Ties are broken by the order `legal_actions` returns —
/// the field's scan order — so the same position always yields the same move.
pub fn choose(state: &MatchState) -> Action {
    choose_with(state, true)
}

/// Та же рука, но не видящая земли: меряет расстояние по прямой, как до
/// местности. Только для замера — чтобы было с чем сравнить.
pub fn choose_blind(state: &MatchState) -> Action {
    choose_with(state, false)
}

fn choose_with(state: &MatchState, sees_ground: bool) -> Action {
    let actions = legal_actions(state);
    // Без местности обе руки — одна и та же, байт в байт: прежние партии и
    // замеры остаются в силе.
    let sees = sees_ground && !state.terrain.is_empty();
    let foe_cells: Vec<Cell> = state
        .standing(state.active.other())
        .iter()
        .filter_map(|id| state.board.cell_of(*id))
        .collect();
    let breakthrough = state.rules.breakthrough;
    let side = state.active;

    // 0. Чужое тело на моём краю: если оно простоит до конца хода, партия
    //    проиграна, и никакой размен этого не перевешивает. Бьётся сильнейшим,
    //    что есть, — добить лучше, но и ранить лучше, чем стоять.
    if breakthrough {
        let threats = state.on_goal(side.other());
        let mut best_guard: Option<(&Action, i32)> = None;
        for action in &actions {
            if let Some((off, target)) = wound_of(state, action) {
                if !threats.contains(&target) {
                    continue;
                }
                let t = &state.units[target as usize];
                let score = if off >= t.health.current { 1000 + off } else { off };
                if off > 0 && best_guard.is_none_or(|(_, best)| score > best) {
                    best_guard = Some((action, score));
                }
            }
        }
        if let Some((action, _)) = best_guard {
            return action.clone();
        }
    }

    // ½. Сбросить врага в овраг. Толчок жадная рука в остальном не берёт —
    //    расстановку она не оценивает, — но толчок в овраг не расстановка, а
    //    гибель тела при любом здоровье, то есть добивание, которое сильнее
    //    всякого удара. Проверяется самим ходом: свёртка скажет, упал ли.
    if sees {
        for action in &actions {
            let Action::Cast { caster, ability, .. } = action else { continue };
            let shoves = state.units[*caster as usize]
                .ability_by_key(ability)
                .and_then(|a| a.casting())
                .is_some_and(|c| c == crate::spell::Casting::Shove);
            if !shoves {
                continue;
            }
            let Ok((_, events)) = crate::state::reduce(state, action) else { continue };
            let felled = events.iter().any(|e| {
                matches!(e, crate::event::Event::Fell { unit, .. } if state.units[*unit as usize].owner != side)
            });
            if felled {
                return action.clone();
            }
        }
    }

    // 1. A blow that finishes a body. Nothing else is worth more this turn.
    let mut killing: Option<(&Action, i32)> = None;
    for action in &actions {
        if let Some((off, target)) = wound_of(state, action) {
            let t = &state.units[target as usize];
            if off >= t.health.current {
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

    // 1½. Встать на чужой край. Стоит после добивания и до всего остального:
    //     простоявший там ход противника выигрывает партию, а лечение и
    //     выставление подождут. Шаг тело не тратит — ударить оно ещё успеет.
    if breakthrough {
        for action in &actions {
            if let Action::Move { to, .. } = action {
                if to.y == state.field.goal_row(side) {
                    return action.clone();
                }
            }
        }
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
    let play_costs = sees.then(|| walk_costs(state, &foe_cells, None, false));
    let mut best_play: Option<(&Action, (i32, u16))> = None;
    for action in &actions {
        if let Action::Play { hand_index, cell } = action {
            let card = &state.side_state(state.active).hand[*hand_index];
            let near = if let Some(cost) = &play_costs {
                landing(state, cost, *cell, nearest_enemy(state, *cell))
            } else {
                nearest_enemy(state, *cell) as u16
            };
            let rank = (card.cost, u16::MAX - near);
            if best_play.is_none_or(|(_, best)| rank > best) {
                best_play = Some((action, rank));
            }
        }
    }
    if let Some((action, _)) = best_play {
        return action.clone();
    }

    // 4. Otherwise wound whoever is closest to falling — кулаком или чарой,
    //    смотря что снимет больше.
    let mut best_hit: Option<(&Action, i32)> = None;
    for action in &actions {
        if let Some((off, target)) = wound_of(state, action) {
            let t = &state.units[target as usize];
            let score = off - t.health.current;
            if best_hit.is_none_or(|(_, best)| score > best) {
                best_hit = Some((action, score));
            }
        }
    }
    if let Some((action, _)) = best_hit {
        return action.clone();
    }

    // 4½. Достать некого — но можно проклясть, благословить или укрыть щитом.
    //     Стоит это ПОСЛЕ урона и ДО шага: чара, которой можно воспользоваться
    //     не двигаясь, лучше шага, а урон лучше её.
    let mut best_rider: Option<(&Action, i32)> = None;
    for action in &actions {
        if let Some(worth) = rider_worth(state, action) {
            if best_rider.is_none_or(|(_, best)| worth > best) {
                best_rider = Some((action, worth));
            }
        }
    }
    if let Some((action, _)) = best_rider {
        return action.clone();
    }

    // 5. Nothing within reach: walk towards whoever is nearest.
    //
    //    This is the verb that makes the field a field. Before it existed, two
    //    lines that could not touch each other simply stood there until the
    //    round limit and the match was decided on leftover health — which is
    //    not a game, it is a stalemate with a scoreboard.
    //
    //    По земле, а не по прямой, когда земля есть: по прямой ближайшая к
    //    врагу клетка бывает тупиком за стеной, и тело входило туда и стояло.
    //    Из равных шагов — тот, что кончается в укрытии.
    let mut best_step: Option<(&Action, (u16, bool))> = None;
    let mut costs_of: Vec<(crate::unit::UnitId, [u16; crate::board::MAX_CELLS])> = Vec::new();
    for action in &actions {
        if let Action::Move { unit, to } = action {
            let Some(from) = state.board.cell_of(*unit) else { continue };
            let (now, after) = if sees {
                if !costs_of.iter().any(|(u, _)| u == unit) {
                    let leaps = state.units[*unit as usize].step >= 2;
                    costs_of.push((*unit, walk_costs(state, &foe_cells, Some(from), leaps)));
                }
                let cost = &costs_of.iter().find(|(u, _)| u == unit).unwrap().1;
                (
                    ground_or_line(cost, from, nearest_enemy(state, from)),
                    landing(state, cost, *to, nearest_enemy(state, *to)),
                )
            } else {
                (nearest_enemy(state, from) as u16, nearest_enemy(state, *to) as u16)
            };
            let exposed = !(sees && state.ground(*to) == Some(crate::state::Ground::Cover));
            // Only a step that closes the gap. Otherwise a body would shuffle
            // sideways for ever, which is a legal action and a wasted turn.
            if after < now && best_step.is_none_or(|(_, best)| (after, exposed) < best) {
                best_step = Some((action, (after, exposed)));
            }
        }
    }
    if let Some((action, _)) = best_step {
        return action.clone();
    }

    // 6. К врагу не подойти — тогда к чужому краю. Без этой ступени тело,
    //    от которого противник отступил, стояло бы, хотя цель у партии есть.
    if breakthrough {
        let goal: Vec<Cell> = state.field.cells()
            .filter(|c| c.y == state.field.goal_row(side) && state.open(*c))
            .collect();
        let mut best_march: Option<(&Action, u16)> = None;
        for action in &actions {
            if let Action::Move { unit, to } = action {
                let Some(from) = state.board.cell_of(*unit) else { continue };
                let line = |c: Cell| c.y.abs_diff(state.field.goal_row(side));
                let (now, after) = if sees {
                    let leaps = state.units[*unit as usize].step >= 2;
                    let cost = walk_costs(state, &goal, Some(from), leaps);
                    (ground_or_line(&cost, from, line(from)), landing(state, &cost, *to, line(*to)))
                } else {
                    (line(from) as u16, line(*to) as u16)
                };
                if after < now && best_march.is_none_or(|(_, best)| after < best) {
                    best_march = Some((action, after));
                }
            }
        }
        if let Some((action, _)) = best_march {
            return action.clone();
        }
    }

    Action::EndTurn
}
