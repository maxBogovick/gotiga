//! Штурм города: снаряжение и проверка проходимости.
//!
//! Без базы и без сети, как `sheet.rs` и `telegram.rs`: снаряжение — это
//! байты картинок, проверка — партии движка. Читает и пишет базу служба.
//!
//! **Снаряжение.** Стена, башня и таран — карты «только для этюдов» без работы
//! автора. Карты заводит миграция (`20261013000004_siege_gear.sql`), а их
//! картинки встроены сюда и при запуске кладутся на склад сервера
//! (`lay_gear_art`): так снаряжение приезжает на рабочий сервер вместе с
//! выкладкой, и ни одной картинки не нужно загружать руками.
//!
//! **Проверка.** Город выверяется партиями ботов (`BATTLE-GENRE-RESEARCH.md`
//! § 7): жадный нападающий проигрывает почти любой город и мерилом служить не
//! может, а бот с перебором на поле 4 × 5 считает до минуты на партию. Поэтому
//! проверка идёт в фоне (`start_probe`), а стол спрашивает её итог.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use battle_core::{Action, MatchState, Outcome, Rules, Setup, Side, bot, reduce};
use uuid::Uuid;

// ── Снаряжение ───────────────────────────────────────────────────────────────

/// Картинки снаряжения: имя файла на складе (`uploads/frames/`) и байты.
/// Рисует их `tools/siege_art.py`; миграция ссылается на эти же имена.
pub const GEAR_ART: [(&str, &[u8]); 3] = [
    ("siege-fortress-wall.webp", include_bytes!("../assets/siege/fortress-wall.webp")),
    ("siege-watchtower.webp", include_bytes!("../assets/siege/watchtower.webp")),
    ("siege-ram.webp", include_bytes!("../assets/siege/siege-ram.webp")),
];

/// Положить картинки снаряжения на склад. Пишет только недостающее или
/// отличающееся: перерисованная картинка приезжает со следующей выкладкой.
pub fn lay_gear_art(upload_dir: &Path) -> std::io::Result<usize> {
    let dir = upload_dir.join("frames");
    std::fs::create_dir_all(&dir)?;
    let mut written = 0;
    for (name, bytes) in GEAR_ART {
        let path = dir.join(name);
        if std::fs::read(&path).ok().as_deref() != Some(bytes) {
            std::fs::write(&path, bytes)?;
            written += 1;
        }
    }
    Ok(written)
}

// ── Проверка проходимости ────────────────────────────────────────────────────

/// Сколько разных начал проверяется. Начало — расстановка войска нападающего
/// по его же клеткам в другом порядке: одна расстановка даёт одну партию
/// (движок без случая), и «берёт ли город» по одной партии не сказать.
pub const PROBE_OPENINGS: usize = 6;

/// Сколько проверка может идти. Дальше она отдаёт то, что успела.
pub const PROBE_BUDGET: Duration = Duration::from_secs(150);

/// Итог проверки. `skilled` — нападающий с перебором (ближе всего к человеку,
/// который думает), `greedy` — жадный (ближе к тому, кто жмёт наугад).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeReport {
    /// Сколько партий каждого рода задумано.
    pub planned: u32,
    pub skilled_played: u32,
    pub skilled_taken: u32,
    pub greedy_played: u32,
    pub greedy_taken: u32,
    /// Все задуманные партии доиграны (не упёрлись в `PROBE_BUDGET`).
    pub finished: bool,
}

/// Начала: та же расстановка, войско нападающего переставлено по своим же
/// клеткам. Первое — как поставил хранитель. Не больше `PROBE_OPENINGS`.
pub fn openings(setup: &Setup) -> Vec<Setup> {
    let cells: Vec<_> = setup.player_board.iter().map(|(_, c)| *c).collect();
    let mut order: Vec<usize> = (0..cells.len()).collect();
    let mut out: Vec<Setup> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    loop {
        let mut s = setup.clone();
        s.player_board = order
            .iter()
            .zip(&cells)
            .map(|(&card, &cell)| (setup.player_board[card].0.clone(), cell))
            .collect();
        // Одинаковые карты, переставленные между собой, — то же самое начало.
        let key: Vec<String> = s.player_board.iter().map(|(c, _)| c.name.clone()).collect();
        if seen.insert(key) {
            out.push(s);
        }
        if out.len() >= PROBE_OPENINGS || !next_permutation(&mut order) {
            return out;
        }
    }
}

fn next_permutation(v: &mut [usize]) -> bool {
    if v.len() < 2 {
        return false;
    }
    let Some(i) = (0..v.len() - 1).rev().find(|&i| v[i] < v[i + 1]) else {
        return false;
    };
    let j = (i + 1..v.len()).rev().find(|&j| v[j] > v[i]).unwrap();
    v.swap(i, j);
    v[i + 1..].reverse();
    true
}

/// Одна партия: нападающий рукой `attacker_depth`, город — `keeper_depth`.
/// `None` — не уложилась в срок.
fn play(setup: Setup, rules: Rules, attacker_depth: u8, keeper_depth: u8, deadline: Instant) -> Option<bool> {
    let mut st = MatchState::begin_with(setup, rules);
    let mut guard = 0;
    while st.outcome.is_none() {
        if guard > 4000 || Instant::now() > deadline {
            return None;
        }
        let depth = if st.active == Side::Player { attacker_depth } else { keeper_depth };
        let action = bot::choose_at(&st, depth);
        st = match reduce(&st, &action) {
            Ok((next, _)) => next,
            // Бот выбирает из законного; отказ — это поломка, а не исход.
            Err(_) => match reduce(&st, &Action::EndTurn) {
                Ok((next, _)) => next,
                Err(_) => return None,
            },
        };
        guard += 1;
    }
    Some(st.outcome == Some(Outcome::Player))
}

/// Проверить город: на каждом начале — партия жадного и партия думающего
/// нападающего. Чистая функция; время ограничено `deadline`.
pub fn probe(setup: &Setup, rules: Rules, keeper_depth: u8, deadline: Instant) -> ProbeReport {
    let starts = openings(setup);
    let mut report = ProbeReport { planned: starts.len() as u32, ..Default::default() };
    let results: Vec<(u8, Option<bool>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = starts
            .iter()
            .flat_map(|s| [(s.clone(), 1u8), (s.clone(), 2u8)])
            .map(|(s, depth)| scope.spawn(move || (depth, play(s, rules, depth, keeper_depth, deadline))))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or((0, None))).collect()
    });
    for (depth, result) in results {
        let Some(taken) = result else { continue };
        if depth == 1 {
            report.greedy_played += 1;
            report.greedy_taken += taken as u32;
        } else {
            report.skilled_played += 1;
            report.skilled_taken += taken as u32;
        }
    }
    report.finished = report.greedy_played == report.planned && report.skilled_played == report.planned;
    report
}

// ── Проверки в фоне ──────────────────────────────────────────────────────────

/// Где стоит проверка: идёт или готова.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum ProbeState {
    Running,
    Done { report: ProbeReport },
}

/// Последние проверки, в памяти: итог нужен столу на минуту, а не навсегда,
/// и после перезапуска его не ждёт никто.
fn jobs() -> &'static Mutex<HashMap<Uuid, (Instant, ProbeState)>> {
    static JOBS: OnceLock<Mutex<HashMap<Uuid, (Instant, ProbeState)>>> = OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Запустить проверку в своём потоке и вернуть её номер.
pub fn start_probe(setup: Setup, rules: Rules, keeper_depth: u8) -> Uuid {
    let id = Uuid::new_v4();
    {
        let mut all = jobs().lock().unwrap();
        // Старше часа — забыть: стол спрашивает итог сразу, а не назавтра.
        all.retain(|_, (at, _)| at.elapsed() < Duration::from_secs(3600));
        all.insert(id, (Instant::now(), ProbeState::Running));
    }
    std::thread::spawn(move || {
        let report = probe(&setup, rules, keeper_depth, Instant::now() + PROBE_BUDGET);
        if let Some(job) = jobs().lock().unwrap().get_mut(&id) {
            job.1 = ProbeState::Done { report };
        }
    });
    id
}

pub fn read_probe(id: Uuid) -> Option<ProbeState> {
    jobs().lock().unwrap().get(&id).map(|(_, s)| s.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use battle_core::{Cell, CardSnapshot};

    fn cell(x: u8, y: u8) -> Cell {
        Cell::new(x, y).unwrap()
    }

    #[test]
    fn openings_permute_the_attackers_over_their_own_cells() {
        let setup = Setup {
            player_board: vec![
                (CardSnapshot::new("a", 1, 5, 1), cell(0, 4)),
                (CardSnapshot::new("b", 1, 5, 1), cell(1, 4)),
                (CardSnapshot::new("c", 1, 5, 1), cell(2, 4)),
            ],
            ..Default::default()
        };
        let all = openings(&setup);
        assert_eq!(all.len(), 6, "три разные карты — шесть начал");
        assert_eq!(all[0].player_board, setup.player_board, "первое — как поставил хранитель");
        // Клетки те же самые, меняется только кто где.
        for s in &all {
            let mut cells: Vec<_> = s.player_board.iter().map(|(_, c)| *c).collect();
            cells.sort_by_key(|c| (c.y, c.x));
            assert_eq!(cells, vec![cell(0, 4), cell(1, 4), cell(2, 4)]);
        }
    }

    #[test]
    fn identical_cards_do_not_count_as_different_openings() {
        let same = CardSnapshot::new("стена", 1, 5, 1);
        let setup = Setup {
            player_board: vec![(same.clone(), cell(0, 4)), (same, cell(1, 4))],
            ..Default::default()
        };
        assert_eq!(openings(&setup).len(), 1);
    }

    #[test]
    fn the_probe_tells_an_open_field_from_a_hopeless_one() {
        // Два боя, которые различает и бот: нападающий вчетверо сильнее
        // защитника — берёт; вчетверо слабее — нет. Мерило должно отличать
        // одно от другого, иначе стол показывал бы шум.
        let fight = |attacker: i32, defender: i32| Setup {
            player_board: vec![(CardSnapshot::new("нападающий", 1, attacker, attacker / 2), cell(1, 3))],
            keeper_board: vec![(CardSnapshot::new("защитник", 1, defender, defender / 2), cell(1, 2))],
            ..Default::default()
        };
        let rules = Rules { idle_toll: 0, ..Rules::default() };
        let far = Instant::now() + Duration::from_secs(60);

        let easy = probe(&fight(20, 5), rules, 1, far);
        assert!(easy.finished);
        assert_eq!((easy.skilled_taken, easy.greedy_taken), (1, 1), "{easy:?}");

        let hopeless = probe(&fight(5, 20), rules, 1, far);
        assert_eq!((hopeless.skilled_taken, hopeless.greedy_taken), (0, 0), "{hopeless:?}");
    }

    #[test]
    fn a_probe_out_of_time_says_it_did_not_finish() {
        let setup = Setup {
            player_board: vec![(CardSnapshot::new("нападающий", 1, 9, 2), cell(1, 3))],
            keeper_board: vec![(CardSnapshot::new("защитник", 1, 9, 2), cell(1, 2))],
            ..Default::default()
        };
        let report = probe(&setup, Rules::default(), 1, Instant::now());
        assert!(!report.finished, "срок вышел до первого хода — итог не полный");
        assert_eq!(report.skilled_played + report.greedy_played, 0);
    }

    #[test]
    fn gear_art_is_laid_once_and_relaid_when_it_differs() {
        let dir = std::env::temp_dir().join(format!("gotiga-gear-{}", Uuid::new_v4()));
        assert_eq!(lay_gear_art(&dir).unwrap(), 3, "пустой склад — все три");
        assert_eq!(lay_gear_art(&dir).unwrap(), 0, "уже лежат — не трогать");
        std::fs::write(dir.join("frames/siege-ram.webp"), b"old").unwrap();
        assert_eq!(lay_gear_art(&dir).unwrap(), 1, "отличается — переложить");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
