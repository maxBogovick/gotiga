//! Проба маршрута ботами (`TASKS-BATTLE-RUN.md` § 3.4).
//!
//! Без базы и без сети, как `siege.rs`: на вход — готовые волны и три отряда,
//! на выход — сколько волн проходит каждый. Игрока за оборону играет тот же бот
//! (`bot::choose_at`, глубина 2 — потолок движка), поэтому проба занижает силу
//! сильного человека: числа читаются как «не меньше, чем».
//!
//! Награды, замены и трофеи в пробе не участвуют: она меряет сам маршрут и
//! перенос здоровья, а не экономику.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use battle_core::{Action, CardSnapshot, MatchState, Outcome, Rules, Setup, Side, bot, reduce};
use serde::Serialize;
use uuid::Uuid;

use crate::battles::LineupGround;

/// Сколько начал на ступень: отряд ставится на клетки, повёрнутый по кругу.
pub const OPENINGS: usize = 5;
/// Глубина обороны в пробе — потолок движка.
pub const DEFENDER_DEPTH: u8 = 2;
/// Сколько проба может идти. Дальше она отдаёт то, что успела.
pub const BUDGET: Duration = Duration::from_secs(150);

/// Волна, подготовленная к пробе: враги, поле, правила и пределы по картам.
#[derive(Clone)]
pub struct ProbeWave {
    /// Враги, подкрепление, условие, местность, поле. Сторона игрока пуста.
    pub base: Setup,
    pub rules: Rules,
    /// Глубина руки врага — как у этюда-основы.
    pub depth: u8,
    pub max_cards: usize,
    /// Строения игрока: встают после отряда.
    pub structures: Vec<(CardSnapshot, battle_core::Cell)>,
    pub ground: LineupGround,
}

/// Отряд одной ступени: карты, как их видит движок.
#[derive(Clone)]
pub struct ProbeTier {
    pub name: &'static str,
    pub squad: Vec<CardSnapshot>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TierReport {
    pub tier: String,
    pub planned: u32,
    pub played: u32,
    /// Сколько волн прошёл каждый забег (по одному числу на забег).
    pub cleared: Vec<u8>,
    pub avg_cleared: f64,
    /// Сколько забегов прошли весь маршрут.
    pub won: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteProbeReport {
    pub waves: u8,
    pub tiers: Vec<TierReport>,
    /// Все задуманные забеги доиграны (не упёрлись в `BUDGET`).
    pub finished: bool,
}

/// Одна партия: оборона глубины `defender`, враг — `enemy`. `None` — срок вышел.
fn play(setup: Setup, rules: Rules, defender: u8, enemy: u8, deadline: Instant) -> Option<MatchState> {
    let mut st = MatchState::begin_with(setup, rules);
    let mut guard = 0;
    while st.outcome.is_none() {
        if guard > 4000 || Instant::now() > deadline {
            return None;
        }
        let depth = if st.active == Side::Player { defender } else { enemy };
        let action = bot::choose_at(&st, depth);
        st = match reduce(&st, &action) {
            Ok((next, _)) => next,
            Err(_) => reduce(&st, &Action::EndTurn).ok()?.0,
        };
        guard += 1;
    }
    Some(st)
}

/// Один забег: сколько волн пройдено подряд. Здоровье переносится, павшие не
/// возвращаются; в волну идут первые живые карты, сколько позволяет волна,
/// повёрнутые по кругу на `opening`, чтобы начала различались.
pub fn run_once(
    waves: &[ProbeWave],
    squad: &[CardSnapshot],
    opening: usize,
    deadline: Instant,
) -> Option<u8> {
    let mut health: Vec<Option<i32>> = vec![None; squad.len()];
    let mut alive: Vec<bool> = vec![true; squad.len()];
    let mut cleared = 0u8;
    for wave in waves {
        let mut chosen: Vec<usize> = (0..squad.len()).filter(|i| alive[*i]).take(wave.max_cards).collect();
        if chosen.is_empty() {
            break;
        }
        let turn = opening % chosen.len();
        chosen.rotate_left(turn);
        let cells = wave.ground.open_cells();
        chosen.truncate(cells.len());

        let mut setup = wave.base.clone();
        setup.player_board.clear();
        setup.player_health.clear();
        for (slot, idx) in chosen.iter().enumerate() {
            let (x, y) = cells[slot];
            let cell = battle_core::Cell::new(x, y)?;
            setup.player_health.push(health[*idx].unwrap_or(squad[*idx].health));
            setup.player_board.push((squad[*idx].clone(), cell));
        }
        setup.player_board.extend(wave.structures.iter().cloned());
        let st = play(setup, wave.rules, DEFENDER_DEPTH, wave.depth, deadline)?;
        for (i, idx) in chosen.iter().enumerate() {
            let Some(unit) = st.units.get(i) else { continue };
            if unit.health.is_dead() {
                alive[*idx] = false;
                health[*idx] = Some(0);
            } else {
                health[*idx] = Some(unit.health.current);
            }
        }
        if st.outcome != Some(Outcome::Player) {
            break;
        }
        cleared += 1;
    }
    Some(cleared)
}

/// Проба маршрута: каждая ступень играет `OPENINGS` забегов, параллельно.
pub fn probe(waves: &[ProbeWave], tiers: &[ProbeTier], deadline: Instant) -> RouteProbeReport {
    let results: Vec<(usize, Option<u8>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = tiers
            .iter()
            .enumerate()
            .flat_map(|(t, tier)| (0..OPENINGS).map(move |o| (t, tier, o)))
            .map(|(t, tier, o)| scope.spawn(move || (t, run_once(waves, &tier.squad, o, deadline))))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or((0, None))).collect()
    });
    let mut report = RouteProbeReport { waves: waves.len() as u8, ..Default::default() };
    for tier in tiers {
        report.tiers.push(TierReport {
            tier: tier.name.to_string(),
            planned: OPENINGS as u32,
            ..Default::default()
        });
    }
    for (t, result) in results {
        let Some(cleared) = result else { continue };
        let row = &mut report.tiers[t];
        row.played += 1;
        row.cleared.push(cleared);
        row.won += (cleared as usize == waves.len()) as u32;
    }
    for row in report.tiers.iter_mut() {
        row.avg_cleared = if row.cleared.is_empty() {
            0.0
        } else {
            row.cleared.iter().map(|c| f64::from(*c)).sum::<f64>() / row.cleared.len() as f64
        };
    }
    report.finished = report.tiers.iter().all(|r| r.played == r.planned);
    report
}

// ── В фоне ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum RouteProbeState {
    Running,
    Done { report: RouteProbeReport },
}

fn jobs() -> &'static Mutex<HashMap<Uuid, (Instant, RouteProbeState)>> {
    static JOBS: OnceLock<Mutex<HashMap<Uuid, (Instant, RouteProbeState)>>> = OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Замок списка проб: упавший поток не должен делать пробу недоступной насовсем.
fn lock_jobs() -> std::sync::MutexGuard<'static, HashMap<Uuid, (Instant, RouteProbeState)>> {
    jobs().lock().unwrap_or_else(|e| e.into_inner())
}

/// Запустить пробу в своём потоке и вернуть её номер. Одновременно идёт одна:
/// проба занимает все ядра, и вторая только замедлила бы первую. `None` — уже
/// идёт другая.
pub fn start(waves: Vec<ProbeWave>, tiers: Vec<ProbeTier>) -> Option<Uuid> {
    let id = Uuid::new_v4();
    {
        let mut all = lock_jobs();
        all.retain(|_, (at, _)| at.elapsed() < Duration::from_secs(3600));
        if all.values().any(|(_, s)| matches!(s, RouteProbeState::Running)) {
            return None;
        }
        all.insert(id, (Instant::now(), RouteProbeState::Running));
    }
    std::thread::spawn(move || {
        let report = probe(&waves, &tiers, Instant::now() + BUDGET);
        if let Some(job) = lock_jobs().get_mut(&id) {
            job.1 = RouteProbeState::Done { report };
        }
    });
    Some(id)
}

pub fn read(id: Uuid) -> Option<RouteProbeState> {
    lock_jobs().get(&id).map(|(_, s)| s.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use battle_core::Cell;

    fn wave(enemy_health: i32, enemy_power: i32) -> ProbeWave {
        let base = Setup {
            keeper_board: vec![(
                CardSnapshot::new("враг", 0, enemy_health, enemy_power),
                Cell::new(1, 2).unwrap(),
            )],
            ..Default::default()
        };
        ProbeWave {
            base,
            rules: Rules { idle_toll: 0, ..Rules::default() },
            depth: 1,
            max_cards: 3,
            structures: vec![],
            ground: LineupGround {
                field: battle_core::Field::default(),
                terrain: vec![],
                occupied: Default::default(),
            },
        }
    }

    fn far() -> Instant {
        Instant::now() + Duration::from_secs(60)
    }

    #[test]
    fn a_strong_squad_clears_the_route_and_a_hopeless_one_does_not() {
        let waves = vec![wave(4, 1), wave(4, 1)];
        let strong: Vec<CardSnapshot> = (0..3).map(|_| CardSnapshot::new("с", 0, 30, 8)).collect();
        let weak: Vec<CardSnapshot> = (0..1).map(|_| CardSnapshot::new("м", 0, 1, 0)).collect();
        assert_eq!(run_once(&waves, &strong, 0, far()), Some(2));
        assert_eq!(run_once(&waves, &weak, 0, far()), Some(0));
    }

    #[test]
    fn health_is_carried_so_a_worn_squad_fails_a_later_wave() {
        // Каждая волна по отдельности по силам карте (она выигрывает и
        // выживает), но три подряд — нет: здоровье переходит, а не обнуляется.
        let one = vec![CardSnapshot::new("карта", 0, 9, 5)];
        let alone = run_once(&[wave(10, 4)], &one, 0, far()).unwrap();
        let chain = run_once(&[wave(10, 4), wave(10, 4), wave(10, 4)], &one, 0, far()).unwrap();
        assert_eq!(alone, 1, "одна волна проходится");
        assert!(chain < 3, "три подряд с переносом ран не проходятся: {chain}");
    }

    #[test]
    fn the_probe_reports_each_tier_and_says_when_it_is_whole() {
        let waves = vec![wave(4, 1)];
        let tiers = vec![
            ProbeTier { name: "strong", squad: (0..3).map(|_| CardSnapshot::new("с", 0, 30, 8)).collect() },
            ProbeTier { name: "weakest", squad: vec![CardSnapshot::new("м", 0, 1, 0)] },
        ];
        let report = probe(&waves, &tiers, far());
        assert!(report.finished);
        assert_eq!(report.tiers[0].won, OPENINGS as u32);
        assert_eq!(report.tiers[1].won, 0);
        assert!(report.tiers[0].avg_cleared > report.tiers[1].avg_cleared);
    }

    #[test]
    fn a_probe_out_of_time_says_it_did_not_finish() {
        let tiers = vec![ProbeTier { name: "strong", squad: vec![CardSnapshot::new("с", 0, 9, 2)] }];
        let report = probe(&[wave(9, 2)], &tiers, Instant::now());
        assert!(!report.finished);
    }
}
