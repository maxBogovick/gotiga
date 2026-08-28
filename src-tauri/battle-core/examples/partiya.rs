//! Одна партия целиком, разыгранная ботом за обе стороны.
//!
//! Существует не ради красоты: это самый дешёвый способ увидеть, что каркас
//! действительно замыкается — поле, ходы, мана, смерть, победа, — и первое
//! место, где видно разбор урона по шагам.
//!
//!     cargo run --example partiya

use battle_core::*;

fn cell(x: u8, y: u8) -> Cell {
    Cell::new(x, y).unwrap()
}

fn setup() -> Setup {
    Setup {
        player_board: vec![
            (CardSnapshot::new("Боец", 1, 6, 3), cell(1, 4)),
            (CardSnapshot::new("Стрелок", 2, 4, 2).with_reach(3), cell(0, 5)),
            (CardSnapshot::new("Швея", 2, 5, 1).with_mend(4).with_reach(2), cell(1, 5)),
        ],
        player_hand: vec![CardSnapshot::new("Пекарь", 3, 7, 3).with_armor(1).with_step(0)],
        keeper_board: vec![
            (CardSnapshot::new("Ворон", 1, 6, 3), cell(1, 1)),
            (CardSnapshot::new("Тень", 2, 4, 2).with_reach(3), cell(2, 0)),
        ],
        keeper_hand: vec![
            CardSnapshot::new("Кот", 2, 5, 2).with_mend(4),
            CardSnapshot::new("Котёл", 3, 7, 3).with_armor(1),
        ],
    }
}

/// Перевод живёт здесь, на краю, а не в ядре. Ровно так же, как i18n на фронте:
/// движок присылает признак шага, а слово к нему подбирает тот, кто показывает.
fn step_name(step: StepId) -> &'static str {
    match step {
        StepId::Immunity => "невосприимчивость",
        StepId::AttackerBless => "благословения бьющего",
        StepId::AttackerCurse => "проклятия бьющего",
        StepId::TargetVulnerable => "уязвимость цели",
        StepId::ChannelDefence => "защита по каналу",
        StepId::Floor => "минимум 1",
        StepId::Shield => "щит",
    }
}

fn side_name(side: Side) -> &'static str {
    match side {
        Side::Player => "игрок",
        Side::Keeper => "хранитель",
    }
}

fn main() {
    let mut st = MatchState::begin(setup());
    println!("── Скромная эпическая битва ──\n");

    while st.outcome.is_none() {
        let action = bot::choose(&st);

        // Разбор считается до применения — resolve ничего не меняет, поэтому
        // спросить «что будет» можно сколько угодно раз.
        if let Action::Attack { attacker, target } = &action {
            let a = st.unit(*attacker).unwrap();
            let t = st.unit(*target).unwrap();
            let res = strike(a, t);
            let trail: Vec<String> =
                res.trail.iter().map(|b| format!("{}: {} → {}", step_name(b.step), b.from, b.to)).collect();
            let why = if trail.is_empty() { String::new() } else { format!("   [{}]", trail.join("; ")) };
            println!(
                "{:>10} · {} бьёт {} на {}{}",
                side_name(st.active),
                a.name(),
                t.name(),
                res.total(),
                why
            );
        }

        let (next, events) = reduce(&st, &action).expect("бот выбирает только из законного");
        st = next;

        for e in &events {
            match e {
                Event::Played { side, unit, cost, .. } => println!(
                    "{:>10} · выставлен {} за {cost}",
                    side_name(*side),
                    st.unit(*unit).unwrap().name()
                ),
                Event::Moved { unit, from, to } => println!(
                    "{:>10} · {} шагает {},{} → {},{}",
                    side_name(st.unit(*unit).unwrap().owner),
                    st.unit(*unit).unwrap().name(),
                    from.x, from.y, to.x, to.y
                ),
                Event::Healed { target, amount } => println!(
                    "{:>10}   {} залечен на {amount}",
                    "",
                    st.unit(*target).unwrap().name()
                ),
                Event::Died { target } => {
                    println!("{:>10}   пал {}", "", st.unit(*target).unwrap().name())
                }
                Event::TurnEnded { side, round } => {
                    if *side == Side::Keeper {
                        println!("           ─── круг {round} ───");
                    }
                }
                Event::Finished { outcome } => println!(
                    "\n── {} ──",
                    match outcome {
                        Outcome::Player => "победил игрок",
                        Outcome::Keeper => "победил хранитель",
                        Outcome::Draw => "ничья",
                    }
                ),
                _ => {}
            }
        }
    }

    println!(
        "здоровье на ногах: игрок {} · хранитель {}",
        st.standing_health(Side::Player),
        st.standing_health(Side::Keeper)
    );
}
