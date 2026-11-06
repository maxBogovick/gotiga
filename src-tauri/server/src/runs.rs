//! Забег: типы и расчёты без базы (`TASKS-BATTLE-RUN.md`).
//!
//! Забег — цепочка волн. Каждая волна — обычная партия движка, помеченная
//! забегом; движок ничего не знает ни о маршруте, ни об отряде. Здесь — то, что
//! стоит между ними: отряд, его здоровье между волнами, расстановка перед волной
//! и то, что вернуть в отряд после неё. Читает и пишет базу служба
//! (`services/runs.rs`).

use battle_core::{MatchState, Reinforcement, Setup, Side, WaveGoal};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::battles::LineupGround;
use crate::models::BattleCardDto;

/// Сколько карт в отряде забега.
pub const SQUAD_MAX: usize = 6;
/// Сколько карт выдаётся тому, у кого собрания нет.
pub const LOAN_CARDS: usize = 3;

pub const STATE_ACTIVE: &str = "active";
pub const STATE_LOST: &str = "lost";
pub const STATE_WON: &str = "won";

// ── Записи базы ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BattleRoute {
    pub id: Uuid,
    pub slug: String,
    pub title_en: String,
    pub title_ru: String,
    pub title_uk: String,
    pub title_es: String,
    pub title_de: String,
    pub title_it: String,
    pub guest_waves: Option<i16>,
    pub status: String,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BattleRouteWave {
    pub id: Uuid,
    pub route_id: Uuid,
    pub wave_no: i16,
    pub challenge_id: Uuid,
    pub reinforcements: String,
    pub goal: Option<String>,
    pub min_cards: i16,
    pub max_cards: i16,
    pub reward_dust: i32,
    pub race_hint: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BattleRun {
    pub id: Uuid,
    pub user_id: Uuid,
    pub route_id: Uuid,
    pub wave_no: i16,
    pub waves_total: i16,
    pub state: String,
    pub squad: String,
    pub swaps: i32,
    pub seed: i64,
    /// `wave` — можно начинать волну; `reward` — сначала выбрать награду.
    pub phase: String,
    pub reward_options: Option<String>,
    pub free_swaps: i32,
}

pub const PHASE_WAVE: &str = "wave";
pub const PHASE_REWARD: &str = "reward";

// ── Отряд ────────────────────────────────────────────────────────────────────

/// Карта отряда и то, что с ней стало за забег.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SquadCard {
    pub card_id: Uuid,
    /// Сколько здоровья осталось. `None` — полное.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<i32>,
    /// Пала в одной из волн: до конца забега в бой не выходит.
    #[serde(default)]
    pub dead: bool,
    /// Выдана забегом, а не взята из собрания человека.
    #[serde(default)]
    pub loaned: bool,
    /// Клетка, на которой карта начала прошлую волну: следующая расстановка
    /// предлагает её же, если она ещё открыта. Только подсказка игроку.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<(u8, u8)>,
}

pub fn read_squad(raw: &str) -> Vec<SquadCard> {
    serde_json::from_str(raw).unwrap_or_default()
}

/// Отряд с купленным трофеем: новая карта встаёт с полным здоровьем, если в
/// отряде есть место и такой карты в нём ещё нет. `None` — отряд не меняется.
pub fn squad_with_trophy(raw: &str, card_id: Uuid) -> Option<String> {
    let mut squad = read_squad(raw);
    if squad.len() >= SQUAD_MAX || squad.iter().any(|c| c.card_id == card_id) {
        return None;
    }
    squad.push(SquadCard { card_id, health: None, dead: false, loaned: false, cell: None });
    serde_json::to_string(&squad).ok()
}

/// Сила карты для отбора «слабейших»: здоровье, сила и броня.
pub fn strength(card: &BattleCardDto) -> i32 {
    card.health as i32 + card.power as i32 + card.armor as i32
}

/// `n` слабейших карт из тех, что выходят на поле. При равенстве — по рангу,
/// затем по номеру: выбор не зависит от порядка, в котором пришла полка.
pub fn weakest(cards: &[&BattleCardDto], n: usize) -> Vec<Uuid> {
    let mut sorted: Vec<&&BattleCardDto> = cards.iter().collect();
    sorted.sort_by(|a, b| (strength(a), a.tier, &a.id).cmp(&(strength(b), b.tier, &b.id)));
    sorted.iter().take(n).filter_map(|c| c.id.parse().ok()).collect()
}

/// Средний отряд для пробы: `n` сильнейших карт рангов 1–2 — то, что есть у
/// человека на середине пути. Выбор из середины общего списка по силе давал
/// колдунов с чарами по площади, которые проходили любой маршрут; ранг
/// называет собрание понятнее. Рангов 1–2 мало — добирает следующие по рангу.
pub fn middle_squad<'a>(cards: &[&'a BattleCardDto], n: usize) -> Vec<&'a BattleCardDto> {
    let mut sorted: Vec<&BattleCardDto> = cards.to_vec();
    sorted.sort_by(|a, b| {
        (a.tier > 2, std::cmp::Reverse(strength(a)), a.tier, &a.id)
            .cmp(&(b.tier > 2, std::cmp::Reverse(strength(b)), b.tier, &b.id))
    });
    sorted.truncate(n);
    sorted
}

/// Сильнейшие `n` — отряд по умолчанию, когда человек сам не выбрал.
pub fn strongest(cards: &[&BattleCardDto], n: usize) -> Vec<Uuid> {
    let mut sorted: Vec<&&BattleCardDto> = cards.iter().collect();
    sorted.sort_by(|a, b| (strength(b), b.tier, &b.id).cmp(&(strength(a), a.tier, &a.id)));
    sorted.iter().take(n).filter_map(|c| c.id.parse().ok()).collect()
}

// ── Расстановка перед волной ─────────────────────────────────────────────────

/// Где какая карта встаёт. Карты без названной клетки встают на открытые
/// клетки в порядке, который называет поле (`open_cells`); названная клетка
/// проверяется так же, как в расстановке перед встречей.
pub fn lay_out(
    ground: &LineupGround,
    chosen: &[(Uuid, Option<(u8, u8)>)],
) -> Result<Vec<(Uuid, (u8, u8))>, &'static str> {
    let mut taken: Vec<(u8, u8)> = Vec::new();
    for (_, at) in chosen {
        if let Some((x, y)) = at {
            ground.cell_fault(*x, *y).map_err(|_| "cellBlocked")?;
            if taken.contains(&(*x, *y)) {
                return Err("cellBlocked");
            }
            taken.push((*x, *y));
        }
    }
    let mut free = ground.open_cells().into_iter().filter(|c| !taken.contains(c));
    let mut out = Vec::with_capacity(chosen.len());
    for (id, at) in chosen {
        let cell = match at {
            Some(c) => *c,
            None => free.next().ok_or("noRoom")?,
        };
        out.push((*id, cell));
    }
    Ok(out)
}

// ── Что вернуть в отряд после волны ──────────────────────────────────────────

/// Перенести здоровье с поля в отряд. `fielded` — карты в порядке
/// `player_board` (так их заморозила расстановка), и тело с тем же номером на
/// доске — та же карта. Павшая остаётся павшей до конца забега.
pub fn carry(state: &MatchState, fielded: &[Uuid], squad: &mut [SquadCard]) {
    for (i, id) in fielded.iter().enumerate() {
        let Some(unit) = state.units.get(i).filter(|u| u.owner == Side::Player) else {
            continue;
        };
        if let Some(card) = squad.iter_mut().find(|c| c.card_id == *id) {
            if unit.health.is_dead() {
                card.dead = true;
                card.health = Some(0);
            } else {
                card.health = Some(unit.health.current);
            }
        }
    }
}

/// Запомнить, где карты начали волну: расстановка заморожена в записи партии
/// в том же порядке, что и `fielded`.
pub fn remember_cells(setup: &Setup, fielded: &[Uuid], squad: &mut [SquadCard]) {
    for (id, (_, cell)) in fielded.iter().zip(&setup.player_board) {
        if let Some(card) = squad.iter_mut().find(|c| c.card_id == *id) {
            card.cell = Some((cell.x, cell.y));
        }
    }
}

// ── Волна: что читается из записи ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveEntry {
    pub round: u8,
    pub x: u8,
    pub y: u8,
    pub card: String,
}

/// Враги волны для показа игроку: стоящие с начала, затем приходящие
/// подкреплением, без повторов и в порядке первого появления.
pub fn wave_enemy_slugs(board: impl IntoIterator<Item = String>, entries: &[WaveEntry]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    board
        .into_iter()
        .chain(entries.iter().map(|e| e.card.clone()))
        .filter(|slug| seen.insert(slug.clone()))
        .collect()
}

/// Подкрепление волны: слаги → снимки, клетки → клетки поля. Отказ словами:
/// расписание с несуществующей картой или клеткой вне поля — ошибка автора.
pub fn read_reinforcements(
    raw: &str,
    field: battle_core::Field,
    snapshot: impl Fn(&str) -> Option<battle_core::CardSnapshot>,
) -> Result<Vec<Reinforcement>, String> {
    let entries: Vec<WaveEntry> =
        serde_json::from_str(raw).map_err(|_| "The reinforcement schedule will not read".to_string())?;
    entries
        .into_iter()
        .map(|e| {
            let cell = field
                .cell(e.x, e.y)
                .ok_or_else(|| "No such cell on the field".to_string())?;
            let card = snapshot(&e.card).ok_or_else(|| format!("Unknown card {}", e.card))?;
            Ok(Reinforcement { round: e.round.max(1), cell, card })
        })
        .collect()
}

pub fn read_goal(raw: Option<&str>) -> Result<Option<WaveGoal>, String> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => serde_json::from_str(s).map(Some).map_err(|_| "The wave goal will not read".to_string()),
    }
}

// ── Числа и рубильник ────────────────────────────────────────────────────────

/// Настройки режима: лежат в `settings` под ключом `battle_run` и правятся на
/// живом доме. Числа — отправная точка, а не замер (`TASKS-BATTLE-RUN.md` § 7.4):
/// их задают после сверки с ценами карт на полке.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RunSettings {
    /// Рубильник: выключен — забеги не начинаются и волны не открываются.
    pub enabled: bool,
    /// Сколько пыли забег может дать одному человеку за сутки (UTC).
    pub daily_dust_cap: i32,
    /// Цена первой замены; каждая следующая дороже на `swap_step`.
    pub swap_base: i32,
    pub swap_step: i32,
    /// Сколько пыли даёт награда «пыль».
    pub reward_dust: i32,
    /// Сколько здоровья возвращает награда «лечение всем».
    pub reward_heal: i32,
    /// Доля награды за волну, которую получает проигравший, в процентах.
    pub loss_share_percent: i32,
    /// Цена трофея по рангу карты (ранги 1..5).
    pub trophy_tier_prices: [i32; 5],
    /// Цена трофея, названная вручную, по слагу карты. Ноль — не продаётся.
    pub trophy_prices: std::collections::BTreeMap<String, i32>,
}

impl Default for RunSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            daily_dust_cap: 60,
            swap_base: 5,
            swap_step: 3,
            reward_dust: 10,
            reward_heal: 3,
            loss_share_percent: 50,
            trophy_tier_prices: [10, 20, 40, 80, 160],
            trophy_prices: Default::default(),
        }
    }
}

impl RunSettings {
    /// Приводит числа к границам, чтобы опечатка не сломала режим.
    pub fn clamped(mut self) -> Self {
        self.daily_dust_cap = self.daily_dust_cap.clamp(0, 10_000);
        self.swap_base = self.swap_base.clamp(0, 1_000);
        self.swap_step = self.swap_step.clamp(0, 1_000);
        self.reward_dust = self.reward_dust.clamp(0, 1_000);
        self.reward_heal = self.reward_heal.clamp(1, 100);
        self.loss_share_percent = self.loss_share_percent.clamp(0, 100);
        for p in self.trophy_tier_prices.iter_mut() {
            *p = (*p).clamp(0, 10_000);
        }
        for p in self.trophy_prices.values_mut() {
            *p = (*p).clamp(0, 10_000);
        }
        self
    }

    /// Во что обходится замена, когда уже сделано `swaps` замен.
    pub fn swap_cost(&self, swaps: i32, free_swaps: i32) -> i32 {
        if free_swaps > 0 { 0 } else { self.swap_base + self.swap_step * swaps.max(0) }
    }

    /// Цена трофея: названная вручную, иначе по рангу. Ноль — не продаётся.
    pub fn trophy_price(&self, slug: &str, tier: i16) -> i32 {
        if let Some(p) = self.trophy_prices.get(slug) {
            return *p;
        }
        let i = (tier.clamp(1, 5) - 1) as usize;
        self.trophy_tier_prices[i]
    }

    /// Сколько пыли за волну: полная за победу, доля за поражение.
    pub fn wave_dust(&self, wave_reward: i32, won: bool) -> i32 {
        if won { wave_reward } else { wave_reward * self.loss_share_percent / 100 }
    }
}

/// Что стало с забегом после волны: следующая волна и состояние.
pub fn after_wave(won_wave: bool, wave_no: i16, waves_total: i16) -> (i16, &'static str) {
    if !won_wave {
        (wave_no, STATE_LOST)
    } else if wave_no >= waves_total {
        (wave_no, STATE_WON)
    } else {
        (wave_no + 1, STATE_ACTIVE)
    }
}

/// Случай забега: считает сервер, из зерна забега и номера шага. Один и тот же
/// запрос даёт один и тот же ответ — повтор ничего не меняет.
pub fn roll(seed: i64, step: u32, upto: u32) -> u32 {
    if upto == 0 {
        return 0;
    }
    let mut x = (seed as u64) ^ (u64::from(step) + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x % u64::from(upto)) as u32
}

/// Замена выбирает случайную карту из собрания, не стоящую в отряде.
/// Кандидаты берутся в порядке номеров, чтобы выбор не зависел от порядка полки.
pub fn pick_replacement(seed: i64, swaps: i32, candidates: &[Uuid]) -> Option<Uuid> {
    let mut sorted = candidates.to_vec();
    sorted.sort();
    sorted.dedup();
    if sorted.is_empty() {
        return None;
    }
    Some(sorted[roll(seed, swaps.max(0) as u32, sorted.len() as u32) as usize])
}

// ── Награда из трёх ──────────────────────────────────────────────────────────

/// Виды награды. Список закрыт: новая награда — новый вид и новая ветка в
/// `apply_reward`, а не поле с произвольным действием.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RewardKind {
    /// Пыль в кошелёк (под суточным пределом).
    Dust,
    /// Всем живым картам отряда здоровье.
    HealAll,
    /// Самой раненой живой карте — полное здоровье.
    HealOne,
    /// Павшая карта возвращается с половиной здоровья.
    Revive,
    /// Следующая замена бесплатна.
    FreeSwap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewardOption {
    pub kind: RewardKind,
    pub amount: i32,
}

/// До трёх наград на выбор. Предлагается только то, что сейчас что-то даёт:
/// лечение — если есть раненые, возвращение — если есть павшие, пыль и
/// бесплатная замена — не гостю (у гостя нет ни кошелька, ни собрания).
/// `dust_room` — сколько пыли забег ещё может дать сегодня: когда предел
/// исчерпан, пыль в награды не попадает, а иначе выбор «пыль» не дал бы ничего.
pub fn reward_options(
    seed: i64,
    wave_no: i16,
    squad: &[SquadCard],
    max_health: impl Fn(Uuid) -> i32,
    guest: bool,
    dust_room: i32,
    settings: &RunSettings,
) -> Vec<RewardOption> {
    let alive = || squad.iter().filter(|c| !c.dead);
    let wounded = alive().any(|c| c.health.is_some_and(|h| h < max_health(c.card_id)));
    let fallen = squad.iter().any(|c| c.dead);

    let mut all = Vec::new();
    let dust = settings.reward_dust.min(dust_room);
    if !guest && dust > 0 {
        all.push(RewardOption { kind: RewardKind::Dust, amount: dust });
    }
    if wounded {
        all.push(RewardOption { kind: RewardKind::HealAll, amount: settings.reward_heal });
        all.push(RewardOption { kind: RewardKind::HealOne, amount: 0 });
    }
    if fallen {
        all.push(RewardOption { kind: RewardKind::Revive, amount: 0 });
    }
    if !guest {
        all.push(RewardOption { kind: RewardKind::FreeSwap, amount: 0 });
    }

    // Перемешать зерном и взять три: порядок на экране не выдаёт «главную».
    let mut out = Vec::new();
    let mut step = (wave_no.max(0) as u32) * 16;
    while out.len() < 3 && !all.is_empty() {
        let i = roll(seed, step, all.len() as u32) as usize;
        out.push(all.remove(i));
        step += 1;
    }
    out
}

/// Имя вида награды для журнала и страницы (то же, что в JSON).
pub fn reward_kind_name(kind: RewardKind) -> &'static str {
    match kind {
        RewardKind::Dust => "dust",
        RewardKind::HealAll => "healAll",
        RewardKind::HealOne => "healOne",
        RewardKind::Revive => "revive",
        RewardKind::FreeSwap => "freeSwap",
    }
}

/// Итог выбранной награды для отряда. Пыль и бесплатная замена возвращаются
/// наружу: они не в отряде.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Applied {
    pub dust: i32,
    pub free_swaps: i32,
}

pub fn apply_reward(
    option: &RewardOption,
    squad: &mut [SquadCard],
    max_health: impl Fn(Uuid) -> i32,
) -> Applied {
    let mut applied = Applied::default();
    match option.kind {
        RewardKind::Dust => applied.dust = option.amount,
        RewardKind::FreeSwap => applied.free_swaps = 1,
        RewardKind::HealAll => {
            for c in squad.iter_mut().filter(|c| !c.dead) {
                let max = max_health(c.card_id);
                if let Some(h) = c.health {
                    c.health = Some((h + option.amount).min(max));
                }
            }
        }
        RewardKind::HealOne => {
            let worst = squad
                .iter_mut()
                .filter(|c| !c.dead)
                .filter(|c| c.health.is_some())
                .min_by_key(|c| c.health.unwrap_or(0) - max_health(c.card_id));
            if let Some(c) = worst {
                c.health = None;
            }
        }
        RewardKind::Revive => {
            if let Some(c) = squad.iter_mut().find(|c| c.dead) {
                c.dead = false;
                c.health = Some((max_health(c.card_id) / 2).max(1));
            }
        }
    }
    applied
}

// ── Трофеи ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BattleRunTrophy {
    pub run_id: Uuid,
    pub card_id: Uuid,
    pub wave_no: i16,
    pub price_dust: i32,
    pub taken: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrophyDto {
    pub card_id: String,
    pub slug: String,
    pub wave_no: i16,
    /// Ноль — не продаётся.
    pub price_dust: i32,
    pub taken: bool,
    /// Эта карта уже лежит на полке игрока: брать её второй раз незачем, и сервер
    /// на такое нажатие ничего не делает.
    pub owned: bool,
    /// Карта опубликована, то есть её вообще можно взять на полку.
    pub on_shelf: bool,
}

// ── Ответы ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteDto {
    pub slug: String,
    pub title_en: String,
    pub title_ru: String,
    pub title_uk: String,
    pub title_es: String,
    pub title_de: String,
    pub title_it: String,
    pub waves: i16,
    /// Сколько волн видит гость; `None` — маршрут ему закрыт.
    pub guest_waves: Option<i16>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SquadCardDto {
    pub card_id: String,
    pub slug: String,
    pub health: i32,
    pub max_health: i32,
    pub dead: bool,
    pub loaned: bool,
    /// Где карта стояла в прошлой волне; нет — не выходила.
    pub cell: Option<battle_core::Cell>,
}

/// Выбранная награда: перед какой волной, какая и на сколько.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickDto {
    pub wave_no: i16,
    pub kind: String,
    pub amount: i32,
}

/// Карта на клетке: враг, строение.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacedDto {
    pub slug: String,
    pub x: u8,
    pub y: u8,
}

/// Вход подкрепления: на каком круге, где и кто.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntranceDto {
    pub round: u8,
    pub x: u8,
    pub y: u8,
    pub slug: String,
}

/// Поле волны для экрана расстановки: что на нём стоит до выхода игрока и куда
/// игрок может встать. Зоны ударов считаются без карт игрока, только со
/// строениями: это подсказка, а не итог боя.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveFieldDto {
    pub width: u8,
    pub depth: u8,
    pub terrain: Vec<battle_core::Tile>,
    pub enemies: Vec<PlacedDto>,
    pub structures: Vec<PlacedDto>,
    pub entrances: Vec<EntranceDto>,
    /// Клетки, на которые можно встать, в порядке, которым сервер расставляет
    /// карты сам.
    pub open_cells: Vec<battle_core::Cell>,
    pub threats: Vec<crate::models::StrikeZoneDto>,
}

/// Следующая волна, как её показывают до начала.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WavePreviewDto {
    pub wave_no: i16,
    pub min_cards: i16,
    pub max_cards: i16,
    pub goal: Option<WaveGoal>,
    pub race_hint: Option<String>,
    pub reward_dust: i32,
    /// Враги волны: стоящие с начала и приходящие подкреплением, без повторов.
    pub enemies: Vec<String>,
    /// Поле волны: нет, если его не удалось собрать (подсказка не мешает начать).
    pub field: Option<WaveFieldDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDto {
    pub id: String,
    pub route: RouteDto,
    pub wave_no: i16,
    pub waves_total: i16,
    pub state: String,
    pub squad: Vec<SquadCardDto>,
    pub swaps: i32,
    /// Идущая партия-волна, если она открыта.
    pub match_id: Option<String>,
    pub next_wave: Option<WavePreviewDto>,
    pub phase: String,
    /// До трёх наград на выбор, пока `phase` — `reward`.
    pub reward: Vec<RewardOption>,
    /// Сколько стоит следующая замена; `None` — гостю замены нет.
    pub swap_cost: Option<i32>,
    pub free_swaps: i32,
    /// Побеждённые враги, которых можно взять на полку.
    pub trophies: Vec<TrophyDto>,
    /// Сколько пыли записала волна, только что кончившаяся этим ответом.
    /// Во всех остальных ответах — ноль.
    /// Выбранные награды этого забега по порядку.
    pub picks: Vec<PickDto>,
    /// Сколько карт из собрания можно взять на замену (не стоят в отряде).
    /// Ноль — замена невозможна, и страница говорит об этом заранее.
    pub bench: i32,
    pub paid_dust: i32,
}

// ── Стол маршрутов ───────────────────────────────────────────────────────────

/// Волна в столе: то, что автор правит. Этюд-основа даёт поле, местность,
/// правила и врагов с начала; остальное — здесь.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminWave {
    pub wave_no: i16,
    pub challenge_id: Uuid,
    #[serde(default)]
    pub reinforcements: Vec<WaveEntry>,
    #[serde(default)]
    pub goal: Option<WaveGoal>,
    pub min_cards: i16,
    pub max_cards: i16,
    pub reward_dust: i32,
    #[serde(default)]
    pub race_hint: Option<String>,
}

/// Маршрут в столе: и то, что читает вкладка, и то, что она сохраняет.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminRoute {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub slug: String,
    pub title_en: String,
    pub title_ru: String,
    #[serde(default)]
    pub title_uk: String,
    #[serde(default)]
    pub title_es: String,
    #[serde(default)]
    pub title_de: String,
    #[serde(default)]
    pub title_it: String,
    #[serde(default)]
    pub guest_waves: Option<i16>,
    pub status: String,
    #[serde(default)]
    pub sort_order: Option<i32>,
    pub waves: Vec<AdminWave>,
    /// Что не доделано: считает сервер при чтении, при сохранении не читается.
    /// Строки вида `код` или `код:слаг` — вкладка переводит их словами.
    #[serde(default, skip_deserializing)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PickRewardRequest {
    pub index: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapRequest {
    /// Карта отряда, которую заменить.
    pub card_id: Uuid,
    /// Сколько замен уже сделано: повтор того же запроса ничего не меняет.
    pub swaps: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeTrophyRequest {
    pub card_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRunRequest {
    pub route: String,
    /// Отряд по выбору из собрания. Пусто — сильнейшие.
    #[serde(default)]
    pub card_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WavePlacement {
    pub card_id: Uuid,
    #[serde(default)]
    pub x: Option<u8>,
    #[serde(default)]
    pub y: Option<u8>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginWaveRequest {
    /// Кто выходит в волну и где. Пусто — живые карты отряда, сколько
    /// позволяет волна, на открытых клетках.
    #[serde(default)]
    pub placements: Vec<WavePlacement>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bought_trophy_joins_the_squad_only_when_there_is_room() {
        let card = |n: u128| Uuid::from_u128(n);
        let seat = |ids: &[u128]| {
            serde_json::to_string(
                &ids.iter()
                    .map(|n| SquadCard { card_id: card(*n), health: Some(2), dead: false, loaned: false, cell: None })
                    .collect::<Vec<_>>(),
            )
            .unwrap()
        };
        let joined = read_squad(&squad_with_trophy(&seat(&[1, 2]), card(9)).unwrap());
        assert_eq!(joined.len(), 3);
        assert_eq!(joined[2].card_id, card(9));
        assert_eq!(joined[2].health, None);
        assert!(squad_with_trophy(&seat(&[1, 2]), card(2)).is_none());
        assert!(squad_with_trophy(&seat(&[1, 2, 3, 4, 5, 6]), card(9)).is_none());
    }

    #[test]
    fn the_wave_preview_lists_each_enemy_once_in_order_of_appearance() {
        let entry = |round, card: &str| WaveEntry { round, x: 0, y: 0, card: card.to_string() };
        let got = wave_enemy_slugs(
            vec!["miner".to_string(), "gnome".to_string(), "miner".to_string()],
            &[entry(2, "gnome"), entry(3, "witch"), entry(4, "witch")],
        );
        assert_eq!(got, vec!["miner", "gnome", "witch"]);
        assert!(wave_enemy_slugs(Vec::new(), &[]).is_empty());
    }
    use battle_core::*;

    fn cell(x: u8, y: u8) -> Cell {
        Cell::new(x, y).unwrap()
    }

    #[test]
    fn the_middle_squad_is_the_strongest_of_the_low_ranks() {
        let mk = |slug: &str, tier: i16, health: i32, power: i32| {
            let mut c = crate::battles::tests::sample_card();
            c.slug = slug.into();
            c.tier = tier as _;
            c.health = health as _;
            c.power = power as _;
            c.armor = 0 as _;
            c
        };
        let cards = vec![
            mk("a", 1, 5, 2),
            mk("b", 2, 7, 4),
            mk("c", 2, 6, 3),
            mk("d", 4, 9, 9),
            mk("e", 3, 8, 8),
            mk("f", 1, 6, 3),
        ];
        let refs: Vec<&BattleCardDto> = cards.iter().collect();
        let got: Vec<&str> = middle_squad(&refs, 3).iter().map(|c| c.slug.as_str()).collect();
        assert_eq!(got, ["b", "f", "c"], "ранги 1–2 по силе, старшие не берутся");
        let all: Vec<&str> = middle_squad(&refs, 5).iter().map(|c| c.slug.as_str()).collect();
        assert_eq!(all, ["b", "f", "c", "a", "d"], "рангов 1–2 мало: добирают старшие, сильнейшие первыми");
    }

    #[test]
    fn health_goes_back_to_the_squad_and_the_fallen_stay_down() {
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let setup = Setup {
            player_board: vec![
                (CardSnapshot::new("а", 0, 8, 1), cell(0, 3)),
                (CardSnapshot::new("б", 0, 6, 1), cell(1, 3)),
            ],
            keeper_board: vec![(CardSnapshot::new("враг", 0, 5, 1), cell(1, 0))],
            ..Default::default()
        };
        let mut st = MatchState::begin(setup);
        st.units[0].health.current = 3;
        st.units[1].health.current = 0;
        let mut squad: Vec<SquadCard> = [a, b, c]
            .iter()
            .map(|id| SquadCard { card_id: *id, health: None, dead: false, loaned: false, cell: None })
            .collect();
        carry(&st, &[a, b], &mut squad);
        assert_eq!((squad[0].health, squad[0].dead), (Some(3), false));
        assert_eq!((squad[1].health, squad[1].dead), (Some(0), true));
        assert_eq!((squad[2].health, squad[2].dead), (None, false), "не выходившая не тронута");
    }

    #[test]
    fn the_squad_remembers_where_each_card_started_the_wave() {
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let setup = Setup {
            player_board: vec![
                (CardSnapshot::new("а", 0, 5, 1), cell(2, 4)),
                (CardSnapshot::new("б", 0, 5, 1), cell(0, 5)),
            ],
            ..Default::default()
        };
        let mut squad: Vec<SquadCard> = [a, b, c]
            .iter()
            .map(|id| SquadCard { card_id: *id, health: None, dead: false, loaned: false, cell: None })
            .collect();
        remember_cells(&setup, &[a, b], &mut squad);
        assert_eq!(squad[0].cell, Some((2, 4)));
        assert_eq!(squad[1].cell, Some((0, 5)));
        assert_eq!(squad[2].cell, None, "не выходившая не тронута");
    }

    #[test]
    fn a_named_cell_is_kept_and_the_rest_fill_open_ones() {
        let ground = LineupGround {
            field: Field::default(),
            terrain: vec![],
            occupied: Default::default(),
        };
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let out = lay_out(&ground, &[(a, Some((0, 5))), (b, None)]).unwrap();
        assert_eq!(out[0], (a, (0, 5)));
        assert_ne!(out[1].1, (0, 5));
        assert!(lay_out(&ground, &[(a, Some((0, 5))), (b, Some((0, 5)))]).is_err());
        assert!(lay_out(&ground, &[(a, Some((0, 0)))]).is_err(), "чужая половина");
    }

    #[test]
    fn the_schedule_reads_and_a_bad_cell_is_refused() {
        let field = Field::default();
        let snap = |s: &str| Some(CardSnapshot::new(s, 0, 3, 1));
        let ok = read_reinforcements(r#"[{"round":2,"x":1,"y":0,"card":"волк"}]"#, field, snap).unwrap();
        assert_eq!((ok[0].round, ok[0].cell), (2, cell(1, 0)));
        assert!(read_reinforcements(r#"[{"round":2,"x":9,"y":0,"card":"волк"}]"#, field, snap).is_err());
        assert!(read_reinforcements(r#"[{"round":2,"x":1,"y":0,"card":"?"}]"#, field, |_| None).is_err());
    }

    #[test]
    fn goals_read_from_json() {
        assert_eq!(read_goal(None).unwrap(), None);
        assert_eq!(
            read_goal(Some(r#"{"kind":"holdRounds","rounds":4}"#)).unwrap(),
            Some(WaveGoal::HoldRounds { rounds: 4 })
        );
        assert!(read_goal(Some("{")).is_err());
    }

    fn sq(n: usize) -> Vec<SquadCard> {
        (0..n)
            .map(|_| SquadCard { card_id: Uuid::new_v4(), health: None, dead: false, loaned: false, cell: None })
            .collect()
    }

    #[test]
    fn a_lost_wave_ends_the_run_and_the_last_won_wave_wins_it() {
        assert_eq!(after_wave(false, 2, 6), (2, STATE_LOST));
        assert_eq!(after_wave(true, 2, 6), (3, STATE_ACTIVE));
        assert_eq!(after_wave(true, 6, 6), (6, STATE_WON));
    }

    #[test]
    fn the_roll_is_repeatable_and_inside_its_bounds() {
        for step in 0..50 {
            assert_eq!(roll(42, step, 7), roll(42, step, 7));
            assert!(roll(42, step, 7) < 7);
        }
        assert_eq!(roll(1, 1, 0), 0);
        let spread: std::collections::HashSet<u32> = (0..50).map(|s| roll(7, s, 5)).collect();
        assert!(spread.len() > 2, "случай не залипает в одном числе");
    }

    #[test]
    fn a_replacement_ignores_the_order_it_is_offered_in() {
        let ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();
        let mut back = ids.clone();
        back.reverse();
        assert_eq!(pick_replacement(9, 2, &ids), pick_replacement(9, 2, &back));
        assert_eq!(pick_replacement(9, 2, &[]), None);
    }

    #[test]
    fn swaps_get_dearer_and_the_free_one_costs_nothing() {
        let s = RunSettings::default();
        assert_eq!((s.swap_cost(0, 0), s.swap_cost(1, 0), s.swap_cost(2, 0)), (5, 8, 11));
        assert_eq!(s.swap_cost(3, 1), 0);
    }

    #[test]
    fn a_hand_set_price_beats_the_rank_price_and_zero_means_not_for_sale() {
        let mut s = RunSettings::default();
        assert_eq!(s.trophy_price("волк", 3), 40);
        s.trophy_prices.insert("волк".into(), 7);
        s.trophy_prices.insert("король".into(), 0);
        assert_eq!((s.trophy_price("волк", 3), s.trophy_price("король", 5)), (7, 0));
    }

    #[test]
    fn a_lost_wave_pays_its_share() {
        let s = RunSettings::default();
        assert_eq!((s.wave_dust(20, true), s.wave_dust(20, false)), (20, 10));
    }

    #[test]
    fn rewards_offer_only_what_gives_something() {
        let s = RunSettings::default();
        let squad = sq(3);
        // Никто не ранен, никто не пал: остаются пыль и замена — не больше двух.
        let opts = reward_options(5, 1, &squad, |_| 10, false, 100, &s);
        assert_eq!(opts.len(), 2);
        assert!(opts.iter().all(|o| matches!(o.kind, RewardKind::Dust | RewardKind::FreeSwap)));
        // Предел дня исчерпан: пыли в наградах нет; остаток меньше награды — отдаётся остаток.
        let spent = reward_options(5, 1, &squad, |_| 10, false, 0, &s);
        assert!(spent.iter().all(|o| o.kind != RewardKind::Dust));
        let part = reward_options(5, 1, &squad, |_| 10, false, 4, &s);
        assert!(part.iter().all(|o| o.kind != RewardKind::Dust || o.amount == 4));
        // Гостю без ран и потерь предложить нечего.
        assert!(reward_options(5, 1, &squad, |_| 10, true, 100, &s).is_empty());
        // Раненый и павший открывают лечение и возвращение; всегда не больше трёх.
        let mut hurt = sq(3);
        hurt[0].health = Some(2);
        hurt[1].dead = true;
        hurt[1].health = Some(0);
        let opts = reward_options(5, 1, &hurt, |_| 10, true, 100, &s);
        assert_eq!(opts.len(), 3);
        assert!(opts.iter().all(|o| !matches!(o.kind, RewardKind::Dust | RewardKind::FreeSwap)));
        assert!(reward_options(5, 1, &hurt, |_| 10, false, 100, &s).len() <= 3);
    }

    #[test]
    fn rewards_change_the_squad_as_they_say() {
        let mut squad = sq(3);
        squad[0].health = Some(2);
        squad[1].health = Some(8);
        squad[2].dead = true;
        squad[2].health = Some(0);
        let max = |_| 10;

        let mut a = squad.clone();
        apply_reward(&RewardOption { kind: RewardKind::HealAll, amount: 3 }, &mut a, max);
        assert_eq!((a[0].health, a[1].health, a[2].dead), (Some(5), Some(10), true));

        let mut b = squad.clone();
        apply_reward(&RewardOption { kind: RewardKind::HealOne, amount: 0 }, &mut b, max);
        assert_eq!((b[0].health, b[1].health), (None, Some(8)), "полное — самой раненой");

        let mut c = squad.clone();
        apply_reward(&RewardOption { kind: RewardKind::Revive, amount: 0 }, &mut c, max);
        assert_eq!((c[2].dead, c[2].health), (false, Some(5)));

        let mut d = squad.clone();
        let got = apply_reward(&RewardOption { kind: RewardKind::FreeSwap, amount: 0 }, &mut d, max);
        assert_eq!((got.free_swaps, d == squad), (1, true));
        let got = apply_reward(&RewardOption { kind: RewardKind::Dust, amount: 10 }, &mut d, max);
        assert_eq!(got.dust, 10);
    }
}
