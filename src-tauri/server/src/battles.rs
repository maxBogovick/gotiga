//! Скромные эпические битвы — pure rules for the shelf of cards.
//!
//! No database here. The service layer uses these to clamp what the keeper
//! types, to keep a card's rank and its owner's level from drifting out of
//! range, and to hand the room a set of frames even before the keeper has
//! opened the design panel once.
//!
//! Two ranges that look alike and mean different things:
//!   * `tier`  — the card's rank, 1..5. A property of the card.
//!   * `level` — the state of one person's copy, 1..5. A property of owning it.

use crate::slug::slugify;
use serde::{Deserialize, Serialize};

pub const CARD_STATUSES: &[&str] = &["draft", "published", "retired"];

pub const TIER_MIN: i16 = 1;
pub const TIER_MAX: i16 = 5;
pub const LEVEL_MIN: i16 = 1;
pub const LEVEL_MAX: i16 = 5;

pub const TITLE_MAX: usize = 80;
/// Room for a list of named abilities, not just one line: a dressed frame with
/// a parchment panel is written to be read, and 160 characters could not hold
/// even three of them. What will not fit on the shelf is clamped by the
/// renderer, not refused by the table.
pub const EFFECT_MAX: usize = 400;
pub const LORE_MAX: usize = 400;

pub const COST_MAX: i16 = 20;
pub const POWER_MAX: i16 = 99;
pub const STAT_MAX: i16 = 99;

/// A card is read at a glance. Past about this many named properties it stops
/// being a card and becomes a page, and the properties band cannot hold them.
pub const TRAITS_MAX: usize = 8;
pub const TRAIT_NAME_MAX: usize = 60;
pub const TRAIT_TEXT_MAX: usize = 200;
pub const RACE_NAME_MAX: usize = 60;
pub const TYPE_MAX: usize = 40;

/// Сколько сыгранных партий читает разбор. Не лента и не бесконечность: сводка
/// считается по прочитанному, и хранителю сказано, по скольким именно.
pub const MATCHES_SHOWN: i64 = 500;

/// The shelf is handed out whole. It is a shelf, not a feed — the same reason
/// the tales room refuses pagination.
pub const SHELF_CARDS: i64 = 500;

// ── Стол гостя ───────────────────────────────────────────────────────────────
//
// Числа не выбраны вкусом. `TASKS-BATTLE-ENGINE.md` §13.1: правило первого хода
// измерено на расстановке «три стоят, три в руке» — «примерно то, что остаётся
// от колоды из шести после расстановки». Колода другой глубины делает выбранное
// правило непроверенным, поэтому это константы, а не настройка.

/// Кем задана сторона гостя.
///
///   `scripted` — рукой хранителя. Этюд: обе стороны расставлены, есть решение.
///   `deck`     — столом гостя. Встреча: хранитель ставит своё, гость приводит своё.
// ── Годность карты ───────────────────────────────────────────────────────────
//
// Одна функция и для отказа при сохранении, и для живой подсказки на весах.
// Две реализации разошлись бы, и хранитель увидел бы предупреждение об одном,
// а отказ — о другом.
//
// Различаются две вещи. ПРЕПЯТСТВИЕ — карта, опубликованная в таком виде, не
// заработает никогда, и публиковать её нельзя. ЗАМЕЧАНИЕ — так можно, но стоит
// знать: это подсказка, а не запрет.

/// Потолок маны из движка, а не число здесь: карта дороже него не может быть
/// оплачена ни на каком ходу, и знать этот предел должен один источник.
pub const MANA_CEILING: i16 = battle_core::state::MANA_CAP as i16;

/// Почему опубликованная карта не заработает. Слова, а не текст: текст живёт
/// в `i18n` на двух языках.
///
/// `points` — вес карты целиком, тело и способности. Нужен ради потолка чина:
/// чин — это не сила, а РАЗРЕШЁННАЯ сила, и скромная карта не имеет права быть
/// бомбой, даже если дорого стоит в мане.
///
/// Потолок — единственное, для чего сумма очков годится как приговор. Замер
/// (`battle-core/examples/revizia.rs`) говорит, почему именно так: связь суммы
/// с долей побед на всей полке 0.95, а среди близких по очкам карт падает до
/// 0.6–0.8. Крупно сумма права, тонко — нет. Поэтому здесь забор, а не весы:
/// «не больше двадцати на третий чин» сумма ответить может, «сыграет ли это»
/// — только прогон.
pub fn card_blockers(
    status: &str,
    health: i16,
    cost: i16,
    points: f64,
    tier: i16,
) -> Vec<&'static str> {
    let mut out = Vec::new();
    if status != "published" {
        // Черновик имеет право быть недоделанным — он затем и черновик.
        return out;
    }
    // Округляется до сотых, как и число, которое видит хранитель: иначе карта
    // ровно в бюджет отказывалась бы из-за двоичной пыли в пятнадцатом знаке.
    if (points * 100.0).round() / 100.0 > tier_budget(tier) {
        out.push("overTierBudget");
    }
    // `can_take_the_field` требует здоровья. Без него карту можно купить, но
    // нельзя ни положить в колоду, ни выставить на поле: она падает в тот же
    // миг, как встанет. Продавать такую — брать плату за невозможное.
    if health <= 0 {
        out.push("noHealth");
    }
    // Мана растёт по единице за ход и упирается в потолок. Карта дороже
    // потолка не может быть выложена никогда, ни на каком ходу.
    if cost > MANA_CEILING {
        out.push("costBeyondMana");
    }
    out
}

/// Так можно, но стоит знать.
pub fn card_notes(
    status: &str,
    tier: i16,
    lendable: bool,
    price_dust: Option<i32>,
    price_feed: Option<i32>,
) -> Vec<&'static str> {
    let mut out = Vec::new();
    // Ноль — это «даром», а пусто — «за эту монету не продаётся». Их легко
    // перепутать в поле ввода, и разница видна только гостю.
    if price_dust == Some(0) || price_feed == Some(0) {
        out.push("freeForACoin");
    }
    // Корм не оседает сам: его выдаёт хранитель руками. Карта только за корм
    // недостижима, пока он никому его не дал.
    if price_dust.is_none() && price_feed.is_some() {
        out.push("onlyForFeed");
    }
    if lendable {
        // Одалживается только первый чин — отбор идёт по чину, а не по отметке.
        if tier != DECK_LOAN_TIER {
            out.push("lendableNotFirstTier");
        }
        if status != "published" {
            out.push("lendableNotPublished");
        }
    }
    out
}

// ── Из рук ───────────────────────────────────────────────────────────────────

/// Сколько записок показывает полка. Немного и намеренно: это поля, а не лента.
pub const GIFTS_SHOWN: i64 = 5;

/// Потолок одной выдачи. Не про баланс — про опечатку: лишний ноль в поле
/// хранителя не должен становиться экономикой.
pub const GRANT_MAX: i32 = 10_000;

pub const CURRENCIES: &[&str] = &["dust", "feed"];

pub fn valid_currency(coin: &str) -> bool {
    CURRENCIES.contains(&coin)
}

/// Записка режется, а не отвергается: выдача не должна срываться из-за
/// лишнего абзаца — тот же счёт, что у заголовка карты.
pub fn clamp_note(note: Option<&str>) -> Option<String> {
    clamp_text(note, 400)
}

pub const SIDE_SCRIPTED: &str = "scripted";
pub const SIDE_DECK: &str = "deck";

/// Ступени сложности бота: рука жадная и рука с перебором.
///
/// Две, а не три. Третья была измерена и не оказалась сильнее второй (44 %
/// побед против неё), а попытка сделать её сильнее — умнее моделью противника
/// — считала ход секундами вместо миллисекунд. Обоснование и таблица лежат в
/// `battle_core::bot::DEPTH_MAX`.
pub const BOT_DEPTH_MIN: i16 = battle_core::bot::DEPTH_MIN as i16;
pub const BOT_DEPTH_MAX: i16 = battle_core::bot::DEPTH_MAX as i16;

pub fn default_bot_depth() -> i16 {
    BOT_DEPTH_MIN
}

/// Глубина, с которой бот действительно умеет играть. Старые испытания могли
/// быть сохранены с тройкой, пока она была в форме, — они читаются как двойка,
/// а не отвергаются: испытание от этого не ломается.
pub fn clamp_bot_depth(raw: i16) -> i16 {
    raw.clamp(BOT_DEPTH_MIN, BOT_DEPTH_MAX)
}

pub fn valid_player_side(side: &str) -> bool {
    side == SIDE_SCRIPTED || side == SIDE_DECK
}

pub const DECK_BOARD: usize = 3;
pub const DECK_HAND: usize = 3;

/// Ограничение по чинам, `TASKS-BATTLE-ENGINE.md` §1.9. Единственная «стоимость
/// колоды», которая нужна на старте: дублей не бывает по построению
/// (`UNIQUE (user_id, card_id)` в собрании и `UNIQUE (figurine_id)` на карте).
pub const DECK_TIER5_MAX: usize = 1;
pub const DECK_TIER4_MAX: usize = 2;

/// Куда дом ставит то, что одолжил, когда гость не выбрал клетку сам.
/// Средний ряд своей половины: не в упор к хранителю и не у самой стены.
pub const DECK_DEFAULT_CELLS: [(u8, u8); DECK_BOARD] = [(0, 4), (1, 4), (2, 4)];

/// Чин, которым дом одалживает. Только первый: заём не должен заменять собой
/// собрание — он должен давать сыграть, пока собрания нет.
pub const DECK_LOAN_TIER: i16 = 1;

/// Клетка своей половины. Половина гостя — `y` 3..5 (`battle-core::board`),
/// и это единственное место, где это правило записано на стороне сервера.
pub fn own_half(x: u8, y: u8) -> bool {
    x < 3 && (3..6).contains(&y)
}

/// Почему стол нельзя сохранить. Закрытый список, чтобы отказ можно было
/// показать человеку на двух языках, а не отдать строкой из середины сервера.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckFault {
    TooManyOnBoard,
    TooManyInHand,
    NotYourHalf,
    CellTaken,
    SameCardTwice,
    NotYours,
    TooManyOfRankFive,
    TooManyOfRankFour,
}

impl DeckFault {
    /// Слово, по которому комната находит свою строку. Не текст: текст живёт в
    /// `i18n`, и сервер, который его сочиняет, сочиняет его на одном языке.
    pub fn word(self) -> &'static str {
        match self {
            DeckFault::TooManyOnBoard => "tooManyOnBoard",
            DeckFault::TooManyInHand => "tooManyInHand",
            DeckFault::NotYourHalf => "notYourHalf",
            DeckFault::CellTaken => "cellTaken",
            DeckFault::SameCardTwice => "sameCardTwice",
            DeckFault::NotYours => "notYours",
            DeckFault::TooManyOfRankFive => "tooManyOfRankFive",
            DeckFault::TooManyOfRankFour => "tooManyOfRankFour",
        }
    }
}

/// Законен ли стол. Чистая функция: ни базы, ни часов — то же правило потом
/// перечитывается в момент начала партии, и второй его реализации быть не
/// должно.
///
/// `owned` — что у гостя есть, `tier_of` — чин карты. Обе справки приходят
/// снаружи, потому что читать их — не дело правила.
pub fn check_deck(
    board: &[(uuid::Uuid, u8, u8)],
    hand: &[uuid::Uuid],
    owned: &std::collections::HashSet<uuid::Uuid>,
    tier_of: &std::collections::HashMap<uuid::Uuid, i16>,
) -> Result<(), DeckFault> {
    if board.len() > DECK_BOARD {
        return Err(DeckFault::TooManyOnBoard);
    }
    if hand.len() > DECK_HAND {
        return Err(DeckFault::TooManyInHand);
    }

    let mut cells = std::collections::HashSet::new();
    for (_, x, y) in board {
        if !own_half(*x, *y) {
            return Err(DeckFault::NotYourHalf);
        }
        if !cells.insert((*x, *y)) {
            return Err(DeckFault::CellTaken);
        }
    }

    // Дублей не бывает по построению — но клиент, который прислал одну карту
    // дважды, построения не знает, и молча проглотить это значило бы выставить
    // на поле два тела с одним владением.
    let mut seen = std::collections::HashSet::new();
    let named = board.iter().map(|(c, _, _)| *c).chain(hand.iter().copied());
    let mut fives = 0usize;
    let mut fours = 0usize;
    for card in named {
        if !seen.insert(card) {
            return Err(DeckFault::SameCardTwice);
        }
        if !owned.contains(&card) {
            return Err(DeckFault::NotYours);
        }
        match tier_of.get(&card).copied().unwrap_or(1) {
            5 => fives += 1,
            4 => fours += 1,
            _ => {}
        }
    }
    if fives > DECK_TIER5_MAX {
        return Err(DeckFault::TooManyOfRankFive);
    }
    if fours > DECK_TIER4_MAX {
        return Err(DeckFault::TooManyOfRankFour);
    }
    Ok(())
}

// ── The body, as the engine reads it ─────────────────────────────────────────
//
// The numbers below are the same ones `battle-core` works in, and the CHECKs in
// the migration repeat them. Repeated deliberately: the database refuses what a
// broken client sends, and these clamp what a keeper types by hand.

pub const CARD_KINDS: &[&str] = &["unit", "spell", "relic"];
pub const ATTACK_CHANNELS: &[&str] = &["physical", "magic", "pure", "none"];

/// Range and step are counted in king's moves on a field three wide and six
/// deep, so 5 means "the whole field, corner to corner".
pub const REACH_MAX: i16 = 5;
pub const STEP_MAX: i16 = 3;
pub const SPEED_MIN: i16 = 1;
pub const SPEED_MAX: i16 = 5;
pub const DEFENCE_MAX: i16 = 20;
pub const MEND_MAX: i16 = 20;

pub const ABILITIES_MAX: usize = 6;
pub const ABILITY_NAME_MAX: usize = 60;
pub const ABILITY_TEXT_MAX: usize = 200;
pub const KEYWORD_NAME_MAX: usize = 60;
pub const KEYWORD_RULES_MAX: usize = 300;

/// The closed list of verbs. A new card brings a new combination, never a new
/// verb — that is the whole reason the engine can be written once.
pub const ABILITY_VERBS: &[&str] = &[
    "damage", "dot", "heal", "hot", "shield", "zone", "bless", "curse", "control",
    "silence", "disarm", "charm", "veil", "guard", "immune", "thorns", "move",
    "summon", "sacrifice", "cleanse", "dispel", "mana",
];

/// How many the effect reaches. `chain` and `radius` carry a number in `radius`.
pub const ABILITY_SHAPES: &[&str] =
    &["self", "one", "adjacent", "chain", "line", "radius", "side", "cell"];

/// When it happens.
pub const ABILITY_TRIGGERS: &[&str] = &[
    "active", "onPlay", "onHit", "onDamaged", "onDeath", "turnStart", "aura", "once",
];

pub fn default_kind() -> String {
    "unit".to_string()
}

pub fn default_channel() -> String {
    "physical".to_string()
}

pub fn default_reach() -> i16 {
    1
}

pub fn default_step() -> i16 {
    1
}

pub fn default_speed() -> i16 {
    3
}

pub fn valid_kind(kind: &str) -> bool {
    CARD_KINDS.contains(&kind)
}

pub fn valid_channel(channel: &str) -> bool {
    ATTACK_CHANNELS.contains(&channel)
}

pub fn clamp_defence(raw: i16) -> i16 {
    raw.clamp(0, DEFENCE_MAX)
}

pub fn clamp_reach(raw: i16) -> i16 {
    raw.clamp(0, REACH_MAX)
}

pub fn clamp_step(raw: i16) -> i16 {
    raw.clamp(0, STEP_MAX)
}

pub fn clamp_speed(raw: i16) -> i16 {
    raw.clamp(SPEED_MIN, SPEED_MAX)
}

pub fn clamp_mend(raw: i16) -> i16 {
    raw.clamp(0, MEND_MAX)
}

/// Doors of the battles API. A card must not take one for a slug.
const RESERVED_SLUGS: &[&str] = &["cards", "frames", "me", "buy", "wallet", "new"];

pub fn valid_status(status: &str) -> bool {
    CARD_STATUSES.contains(&status)
}

pub fn clamp_tier(tier: i16) -> i16 {
    tier.clamp(TIER_MIN, TIER_MAX)
}

pub fn clamp_level(level: i16) -> i16 {
    level.clamp(LEVEL_MIN, LEVEL_MAX)
}

pub fn clamp_cost(cost: i16) -> i16 {
    cost.clamp(0, COST_MAX)
}

pub fn clamp_power(power: i16) -> i16 {
    power.clamp(0, POWER_MAX)
}

pub fn clamp_stat(stat: i16) -> i16 {
    stat.clamp(0, STAT_MAX)
}

fn clamp_text(raw: Option<&str>, max: usize) -> Option<String> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(max).collect())
}

pub fn clamp_title(raw: &str) -> String {
    raw.trim().chars().take(TITLE_MAX).collect()
}

pub fn clamp_effect(raw: Option<&str>) -> Option<String> {
    clamp_text(raw, EFFECT_MAX)
}

pub fn clamp_lore(raw: Option<&str>) -> Option<String> {
    clamp_text(raw, LORE_MAX)
}

pub fn clamp_type(raw: Option<&str>) -> Option<String> {
    clamp_text(raw, TYPE_MAX)
}

/// One named property of a card: "Вихрь Души (Wind of Soul): каждое третье
/// заклинание создаёт копию эффекта." Both names are kept because the card
/// shows them together, the way the keeper's own drawing does.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardTrait {
    #[serde(default)]
    pub name_en: String,
    #[serde(default)]
    pub name_ru: String,
    #[serde(default)]
    pub text_en: String,
    #[serde(default)]
    pub text_ru: String,
}

/// The properties of a card, as they will be stored.
///
/// A property with no name at all is dropped rather than written: an empty row
/// left behind by the editor is not something the keeper meant to say. Text is
/// cut to fit instead of refused, as everywhere else on this card.
pub fn normalize_traits(traits: &[CardTrait]) -> Option<String> {
    let cleaned: Vec<CardTrait> = traits
        .iter()
        .map(|t| CardTrait {
            name_en: t.name_en.trim().chars().take(TRAIT_NAME_MAX).collect(),
            name_ru: t.name_ru.trim().chars().take(TRAIT_NAME_MAX).collect(),
            text_en: t.text_en.trim().chars().take(TRAIT_TEXT_MAX).collect(),
            text_ru: t.text_ru.trim().chars().take(TRAIT_TEXT_MAX).collect(),
        })
        .filter(|t| !t.name_en.is_empty() || !t.name_ru.is_empty())
        .take(TRAITS_MAX)
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    serde_json::to_string(&cleaned).ok()
}

/// Read back what was stored. A column that will not parse yields no properties
/// rather than failing the whole card — the rest of it is still worth showing.
pub fn read_traits(raw: Option<&str>) -> Vec<CardTrait> {
    raw.and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default()
}

/// One executable ability, beside the prose in `traits`.
///
/// Same storage choice as `CardTrait`, for the same three reasons: read only
/// with the card, never searched across cards, order matters. The difference is
/// who reads it — a person reads the trait, the engine reads this.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardAbility {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name_en: String,
    #[serde(default)]
    pub name_ru: String,
    /// From `ABILITY_VERBS`. Anything else is refused.
    pub verb: String,
    #[serde(default = "default_channel")]
    pub channel: String,
    #[serde(default)]
    pub amount: i16,
    #[serde(default = "default_shape")]
    pub shape: String,
    /// The number carried by `chain` (links) and `radius` (cells).
    #[serde(default)]
    pub radius: i16,
    #[serde(default = "default_reach")]
    pub range: i16,
    /// Turns of the bearer. Zero — it happens and is over.
    #[serde(default)]
    pub duration: i16,
    #[serde(default = "default_trigger")]
    pub trigger: String,
    #[serde(default)]
    pub mana_cost: i16,
    #[serde(default)]
    pub cooldown: i16,
    #[serde(default)]
    pub keywords: Vec<String>,
}

pub fn default_shape() -> String {
    "one".to_string()
}

pub fn default_trigger() -> String {
    "active".to_string()
}

/// The abilities of a card, as they will be stored.
///
/// An ability whose verb is not in the closed list is dropped rather than
/// stored: an unknown verb is not a card the engine could ever run, and keeping
/// it would mean the shelf shows a rule that never fires.
pub fn normalize_abilities(abilities: &[CardAbility]) -> Option<String> {
    let cut = |raw: &str, max: usize| -> String { raw.trim().chars().take(max).collect() };
    let cleaned: Vec<CardAbility> = abilities
        .iter()
        .filter(|a| ABILITY_VERBS.contains(&a.verb.trim()))
        .map(|a| {
            let shape = if ABILITY_SHAPES.contains(&a.shape.trim()) {
                a.shape.trim().to_string()
            } else {
                default_shape()
            };
            let trigger = if ABILITY_TRIGGERS.contains(&a.trigger.trim()) {
                a.trigger.trim().to_string()
            } else {
                default_trigger()
            };
            let channel = if valid_channel(a.channel.trim()) {
                a.channel.trim().to_string()
            } else {
                default_channel()
            };
            CardAbility {
                id: cut(&a.id, ABILITY_NAME_MAX),
                name_en: cut(&a.name_en, ABILITY_NAME_MAX),
                name_ru: cut(&a.name_ru, ABILITY_NAME_MAX),
                verb: a.verb.trim().to_string(),
                channel,
                amount: a.amount.clamp(0, POWER_MAX),
                shape,
                radius: a.radius.clamp(0, 3),
                range: clamp_reach(a.range),
                duration: a.duration.clamp(0, 5),
                trigger,
                mana_cost: a.mana_cost.clamp(0, COST_MAX),
                cooldown: a.cooldown.clamp(0, 5),
                keywords: a
                    .keywords
                    .iter()
                    .map(|k| cut(k, ABILITY_NAME_MAX))
                    .filter(|k| !k.is_empty())
                    .take(4)
                    .collect(),
            }
        })
        .take(ABILITIES_MAX)
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    serde_json::to_string(&cleaned).ok()
}

/// Read back what was stored. A column that will not parse yields no abilities
/// rather than failing the whole card — the rest of it is still worth showing.
pub fn read_abilities(raw: Option<&str>) -> Vec<CardAbility> {
    raw.and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default()
}

// ── The scales ───────────────────────────────────────────────────────────────
//
// One unit: a single point of damage, to one target, from range 1, at once, on
// command. Everything else is quoted against it. The rates come from
// `TASKS-BATTLE-ENGINE.md` §7.2 — and §12 records that the range multiplier is
// already known to be too low against what simulation measured. It is left as
// written until the runner replaces it with a measured one, because a number
// half-corrected by hand is worse than a number known to be wrong.

/// What a card of this rank is allowed to weigh. Rank is not strength; rank is
/// *permitted* strength.
pub fn tier_budget(tier: i16) -> f64 {
    8.0 + 6.0 * ((clamp_tier(tier) - 1) as f64)
}

fn shape_multiplier(shape: &str, radius: i16) -> f64 {
    match shape {
        "self" => 0.8,
        "one" | "cell" => 1.0,
        "adjacent" => 1.8,
        "chain" => 1.0 + 0.7 * ((radius.max(1) - 1) as f64),
        "line" => 2.0,
        "radius" => {
            if radius >= 2 {
                3.2
            } else {
                2.4
            }
        }
        "side" => 3.6,
        _ => 1.0,
    }
}

fn trigger_multiplier(trigger: &str) -> f64 {
    match trigger {
        "onHit" | "onDamaged" | "once" => 0.8,
        "onDeath" => 0.7,
        "aura" => 1.5,
        _ => 1.0,
    }
}

fn channel_multiplier(channel: &str) -> f64 {
    match channel {
        "magic" => 1.05,
        "pure" => 1.4,
        _ => 1.0,
    }
}

/// Range is worth real but modest money: 1.0 at arm's length, 1.4 across the
/// whole field.
fn range_multiplier(range: i16) -> f64 {
    0.9 + 0.1 * (clamp_reach(range) as f64)
}

/// What one ability costs, in points.
pub fn ability_points(a: &CardAbility, tier: i16) -> f64 {
    let amount = a.amount as f64;
    let turns = a.duration.max(1) as f64;
    let control = (3.0 + clamp_tier(tier) as f64) * turns;

    let base = match a.verb.as_str() {
        "damage" => amount,
        "dot" => 0.8 * amount * turns,
        "heal" => 0.7 * amount,
        "hot" => 0.6 * amount * turns,
        "shield" => 0.8 * amount,
        "zone" => 0.9 * amount * turns,
        "bless" | "curse" => 0.9 * amount * turns,
        "control" => control,
        "silence" => 0.6 * control,
        "disarm" => 0.5 * control,
        "charm" => 2.0 * control,
        "veil" => 4.0 * turns,
        "guard" => 2.5 * amount.max(1.0),
        "immune" => 6.0 * turns,
        "thorns" => 1.5 * amount,
        "move" => amount,
        "summon" => 0.9 * amount,
        // The one verb with a discount: it pays with a body.
        "sacrifice" => -0.7 * amount,
        "cleanse" | "dispel" => 2.0,
        "mana" => 2.0 * amount,
        _ => 0.0,
    };

    base * shape_multiplier(&a.shape, a.radius)
        * trigger_multiplier(&a.trigger)
        * channel_multiplier(&a.channel)
        * range_multiplier(a.range)
}

/// What a body costs before a single ability is written on it.
///
/// `speed` сюда больше не входит, и это не подгонка курса, а удаление вранья:
/// характеристика бралась в 2 очка за ступень — 8 очков за пятую, то есть весь
/// бюджет первого чина, — а в `battle-core` слова `speed` нет ни разу. Ход
/// чередуется, очерёдности по скорости в игре не существует. Столбец в таблице
/// оставлен и значения сохраняются: появится очерёдность — вернётся и строка
/// здесь, но выдумывать ей цену раньше движка нельзя.
pub fn body_points(
    health: i16,
    armor: i16,
    ward: i16,
    power: i16,
    reach: i16,
    step: i16,
    mend: i16,
) -> f64 {
    0.5 * health as f64
        + 1.2 * armor as f64
        + 1.2 * ward as f64
        + power as f64 * range_multiplier(reach)
        + step_points(step)
        + 0.7 * mend as f64 * range_multiplier(reach)
}

/// Цена шага, четверть очка за клетку сверх первой.
///
/// Число выбрано замером, а не на слух, и оно намеренно скромное — потому что
/// замер даёт только ПОТОЛОК, а не точку. При равных очках тело с шагом 2
/// выигрывает у тела с шагом 1 в 64 % стычек и 50 % встреч на расстоянии; шаг 3
/// — 53 % и 67 %. То есть шаг выигрывает, и стоить ноль он не имеет права.
///
/// Но стоит он мало: чтобы медленное тело сравнялось, хватает ОДНОГО очка
/// здоровья, то есть половины очка весов, — и хватает его одинаково против
/// шага 2 и против шага 3. Значит вся цена шага целиком не больше половины
/// очка, и четверть за клетку кладёт шаг 3 ровно на этот потолок.
///
/// Взято по верхнему краю намеренно. Настоящая цена, судя по тому, что при том
/// же лишнем очке здоровья медленное тело выигрывает уже 86–89 %, заметно ниже
/// — но это забор, и ошибаться ему лучше в сторону строгости: пропущенная
/// бомба дороже, чем карта, которой велели срезать полочка.
///
/// Точнее эта линейка не покажет никогда: бой прыгает целыми ударами, и между
/// «здоровья не хватило» и «хватило» нет промежуточных состояний. Тонкую
/// разницу решает прогон, а не формула.
///
/// Шаг 0 — неподвижное тело — считается наравне с шагом 1, а не в минус.
/// Неподвижность и правда недостаток, но насколько — не измерено, а платить
/// бюджетом за неизмеренное значит выдавать хранителю очки из воздуха: любой
/// котёл получил бы скидку, которую никто не проверял.
fn step_points(step: i16) -> f64 {
    0.25 * (clamp_step(step).max(1) - 1) as f64
}

/// Points against price. 1.0 is on the curve; above 1.15 the card is overloaded,
/// below 0.85 nobody will put it in a deck.
///
/// The "+2" is a tax on existing: even an empty body takes a place on the field.
pub fn balance_index(points: f64, cost: i16) -> f64 {
    let denominator = 4.0 * clamp_cost(cost) as f64 + 2.0;
    if denominator <= 0.0 {
        return 0.0;
    }
    points / denominator
}

// ── The card, as the engine takes it ─────────────────────────────────────────

/// Turn a card into the body a match will fight with.
///
/// The one place where the archive and the engine meet. Everything above this
/// line is the house's own vocabulary; everything the engine sees comes through
/// here, which is why it is a function and not a scattering of field accesses.
///
/// `name` carries the **slug** rather than a title. The journal outlives the
/// session that wrote it and is read in both languages, so a Russian title
/// frozen into it would be a language baked into a record. The scene looks the
/// real card up by that slug and renders whichever title the reader wants.
pub fn to_snapshot(card: &crate::models::BattleCardDto) -> battle_core::CardSnapshot {
    battle_core::CardSnapshot {
        name: card.slug.clone(),
        cost: card.cost as i32,
        health: card.health as i32,
        power: card.power as i32,
        armor: card.armor as i32,
        ward: card.ward as i32,
        reach: card.reach.clamp(0, REACH_MAX) as u8,
        step: card.step.clamp(0, STEP_MAX) as u8,
        mend: card.mend as i32,
        channel: to_channel(&card.attack_channel),
        strikes: strikes(&card.attack_channel),
    }
}

/// Наносит ли тело удары вообще. Канал «не бьёт» — единственное, что это
/// значит: в движке каналов три, и отсутствие удара живёт отдельным полем.
pub fn strikes(channel: &str) -> bool {
    channel != "none"
}

/// Сила, за которую весы берут деньги.
///
/// У тела, которое не бьёт, сила мертва: она напечатана на карте и никогда не
/// превращается в урон. Брать за неё очки — та же ошибка, что была со
/// скоростью, только тише: котёл с силой 5 платил бы пять очков ни за что и
/// мог не пройти потолок чина.
pub fn striking_power(power: i16, channel: &str) -> i16 {
    if strikes(channel) { power } else { 0 }
}

/// The stored word for a channel, as the engine's own.
///
/// An unknown word becomes bodily rather than refusing the card: the column has
/// a CHECK on it, so this can only be reached by a hand-edited database, and a
/// match that will not start is a worse answer than a card that hits normally.
pub fn to_channel(raw: &str) -> battle_core::Channel {
    match raw {
        "magic" => battle_core::Channel::Magic,
        "pure" => battle_core::Channel::Pure,
        _ => battle_core::Channel::Physical,
    }
}

/// Whether a card can stand on a field at all.
///
/// A body with no health falls the moment it is raised, which is not a card the
/// keeper meant to publish — it is one that was never given numbers. Caught here
/// so a match refuses to start rather than ending on its own first tick.
pub fn can_take_the_field(card: &crate::models::BattleCardDto) -> bool {
    card.health > 0 && card.status == "published"
}

/// A price of `None` means "not to be had for this coin", which is not zero.
/// Negative is nonsense either way, so it becomes "not for this coin" too.
pub fn clamp_price(raw: Option<i32>) -> Option<i32> {
    raw.filter(|p| *p >= 0)
}

/// The four rungs of the level ladder, or nothing.
///
/// Four exactly — 1→2, 2→3, 3→4, 4→5 — because there are five levels and there
/// will not be a sixth. A list of any other length is not a shorter ladder, it
/// is a mistake, and it is refused rather than padded: a card silently given
/// free rungs would be discovered by whoever climbed them.
pub fn clamp_level_prices(raw: Option<&[i32]>) -> Option<Vec<i32>> {
    let steps = raw?;
    if steps.len() != 4 || steps.iter().any(|p| *p < 0) {
        return None;
    }
    Some(steps.to_vec())
}

pub fn unique_slug(preferred: Option<&str>, title: &str, taken: &[String]) -> String {
    let mut base = preferred
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(slugify)
        .unwrap_or_default();
    if base.is_empty() {
        base = slugify(title);
    }
    if base.is_empty() {
        base = "card".into();
    }
    if RESERVED_SLUGS.contains(&base.as_str()) {
        base = format!("card-{base}");
    }
    if !taken.iter().any(|s| s == &base) {
        return base;
    }
    for n in 2..200 {
        let candidate = format!("{base}-{n}");
        if !taken.iter().any(|s| s == &candidate) {
            return candidate;
        }
    }
    format!("{base}-{}", uuid::Uuid::new_v4().simple())
}

/// How the photograph sits inside the frame: a point to centre on and how close
/// to stand. Stored as JSON text, the same shape the figurine keyhole uses.
/// Anything unparseable becomes `None` — a card with a broken focus is centred,
/// never blank.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ArtFocal {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

pub fn normalize_focal(raw: Option<&str>) -> Option<String> {
    let parsed: ArtFocal = serde_json::from_str(raw?.trim()).ok()?;
    let focal = ArtFocal {
        x: parsed.x.clamp(0.0, 1.0),
        y: parsed.y.clamp(0.0, 1.0),
        zoom: parsed.zoom.clamp(1.0, 3.0),
    };
    serde_json::to_string(&focal).ok()
}

/// A single card's exception to the tier's shared frame — "wear a picture of
/// your own" without touching the frame every other card of that rank still
/// wears. Every field optional and `None` when absent, unlike `BattleFrame`
/// itself: this is a patch, not a whole dressing, and a field left out here
/// means "keep the tier's own", not "keep it empty".
///
/// The four insets travel with the picture rather than staying tier-only: a
/// picture chosen just for this card (or, for a race's `level_frames`, just
/// for this level) was never measured against the tier's own window, so
/// keeping the tier's insets would centre the content on ornament that isn't
/// there and cut into ornament that is.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aspect: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inset_top: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inset_right: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inset_bottom: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inset_left: Option<f32>,
}

/// The clamps a `FrameOverride` patch must pass however it arrives — alone or
/// as one slot of a race's `level_frames`. `None` for "no override at all": a
/// broken or empty patch is the same as none, never a patch that blanks the
/// tier's own picture.
///
/// Each inset is clamped on its own, unlike a tier's own (`clamp_pair`, which
/// also caps top+bottom together): a race's own picture isn't mirrored
/// top-to-bottom by `applyInsetDelta` in the first place, so there is no pair
/// to check the sum of.
fn clean_frame_override(parsed: FrameOverride) -> Option<FrameOverride> {
    let frame_image = parsed
        .frame_image
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let frame_mode = parsed.frame_mode.filter(|m| valid_frame_mode(m));
    let aspect = parsed.aspect.map(|a| a.clamp(0.3, 2.0));
    let inset = |v: Option<f32>| v.filter(|v| v.is_finite()).map(|v| v.clamp(0.0, 45.0));
    let inset_top = inset(parsed.inset_top);
    let inset_right = inset(parsed.inset_right);
    let inset_bottom = inset(parsed.inset_bottom);
    let inset_left = inset(parsed.inset_left);
    if frame_image.is_none()
        && frame_mode.is_none()
        && aspect.is_none()
        && inset_top.is_none()
        && inset_right.is_none()
        && inset_bottom.is_none()
        && inset_left.is_none()
    {
        return None;
    }
    Some(FrameOverride {
        frame_image,
        frame_mode,
        aspect,
        inset_top,
        inset_right,
        inset_bottom,
        inset_left,
    })
}

pub fn normalize_frame_override(raw: Option<&str>) -> Option<String> {
    let parsed: FrameOverride = serde_json::from_str(raw?.trim()).ok()?;
    serde_json::to_string(&clean_frame_override(parsed)?).ok()
}

/// A race's own dress per level of an owned copy — five slots, each the same
/// patch a card's own `frame_override` is. Padded/truncated to exactly 5 so a
/// client always finds a slot for every level; `None` (not `[null,null,...]`)
/// when every slot ends up empty, keeping the column unused rather than
/// storing five nulls forever.
pub fn normalize_level_frames(raw: Option<&str>) -> Option<String> {
    let parsed: Vec<Option<FrameOverride>> = serde_json::from_str(raw?.trim()).ok()?;
    let mut slots: Vec<Option<FrameOverride>> = (0..5)
        .map(|i| parsed.get(i).cloned().flatten().and_then(clean_frame_override))
        .collect();
    if slots.iter().all(Option::is_none) {
        return None;
    }
    slots.truncate(5);
    serde_json::to_string(&slots).ok()
}

/// How a rank is dressed. Frames are the design, cards are the data — a rule
/// every card-authoring tool converges on, and the reason the keeper edits five
/// frames instead of forty cards.
///
/// Two ways to dress a card, and a frame may use either. PAINTED: paper, ink,
/// border and foil, drawn by the renderer. DRESSED: a photograph of a real
/// frame laid under the whole card, with the card's content set inside it.
///
/// The photograph goes BEHIND everything rather than over it. An overlay would
/// need a transparent window, and this house re-encodes every uploaded image to
/// JPEG, which has no alpha — an overlay frame would arrive with its window
/// filled in. Laid behind, an ordinary photograph works.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleFrame {
    pub tier: i16,
    pub name_en: String,
    pub name_ru: String,
    /// The card's paper. Still the ground under a photograph that fails to load.
    pub paper: String,
    /// Ink for title and stats.
    pub ink: String,
    pub border: String,
    /// Colour of the single slow sweep across the face. Empty = no foil at all,
    /// which is what a humble card should be.
    #[serde(default)]
    pub foil: String,
    /// A picture of a frame. Empty = painted.
    #[serde(default)]
    pub frame_image: String,
    /// `overlay` — the picture lies ON TOP of the card and the card shows through
    ///             the hole in it. What a cut-out frame with transparency wants:
    ///             the carving's inner edge overlaps the photograph, and a window
    ///             set a little wrong is hidden rather than exposed.
    /// `behind`  — the picture is the card's ground. For a frame with no hole in
    ///             it, where an overlay would simply cover everything.
    #[serde(default)]
    pub frame_mode: String,
    /// A texture for the card's ground — the paper the content is written on.
    /// A cut-out frame has nothing behind it but this. Empty = flat `paper`.
    #[serde(default)]
    pub paper_image: String,
    /// The reverse — what a card you do not own shows lying in dust. The
    /// frame (carving or dressed ground) is the FRONT's own dress and never
    /// worn on the back, whatever this is set to; empty means the plain dusty
    /// texture the renderer already draws.
    #[serde(default)]
    pub back_image: String,
    /// Where the card's content sits inside that photograph, as a percentage of
    /// the card on each side. A carved frame has thick sides; this is how the
    /// keeper says where the window actually is.
    #[serde(default)]
    pub inset_top: f32,
    #[serde(default)]
    pub inset_right: f32,
    #[serde(default)]
    pub inset_bottom: f32,
    #[serde(default)]
    pub inset_left: f32,
    /// Card width ÷ height. A dressed card is often squarer than a bare one.
    #[serde(default)]
    pub aspect: f32,
    /// The card is four bands: header, photograph, properties, footer. Three of
    /// them are given a share of the content's height; the properties band takes
    /// whatever is left, because it is the one that has to hold a paragraph.
    ///
    /// `Option` rather than a bare number, unlike the photograph's share: zero is
    /// a real choice here — a bare card wants no header and no footer — so
    /// "absent" and "set to nothing" cannot be the same value. Absent means a
    /// frame saved before the card had bands, and it takes the default;
    /// `Some(0.0)` means the keeper dragged the slider to the floor.
    /// Always `Some` after `normalize_frames`, so the client only ever sees a
    /// number.
    #[serde(default)]
    pub header_share: Option<f32>,
    /// How much of the content the work's photograph takes, 0..1. Zero is not a
    /// choice here — a card with no picture band has nowhere to look — so zero
    /// keeps meaning "not set".
    #[serde(default)]
    pub art_share: f32,
    #[serde(default)]
    pub foot_share: Option<f32>,
    /// Face for the name, by id from the site's own font list. Empty = the
    /// card's ordinary serif.
    #[serde(default)]
    pub title_font: String,
    /// Ink for the name alone, when the frame wants it apart. Empty = `ink`.
    #[serde(default)]
    pub title_ink: String,
    /// `corners` — cost and power ride the corners, as on a card held in a hand.
    /// `plaque`  — the name stands on a plate under the picture and the stats
    ///             are read as a line, as on a card standing in a case.
    #[serde(default)]
    pub layout: String,
    /// Centre of the cost badge, `corners` layout only — X in % of the card's
    /// width, Y in % of its height. `Option` for the same reason as the band
    /// shares: a frame saved before the badge was draggable has none, and
    /// zero is a real place on the card (the top-left corner itself).
    #[serde(default)]
    pub cost_x: Option<f32>,
    #[serde(default)]
    pub cost_y: Option<f32>,
    /// Centre of the power badge, same units.
    #[serde(default)]
    pub power_x: Option<f32>,
    #[serde(default)]
    pub power_y: Option<f32>,
    /// The badge's own outline — a coin is not the only shape a cost or a
    /// power has ever worn. Independent per badge, so a keeper can, say, keep
    /// cost round and give power a shield.
    #[serde(default)]
    pub cost_shape: String,
    #[serde(default)]
    pub power_shape: String,
}

pub const LAYOUTS: &[&str] = &["corners", "plaque"];
pub const FRAME_MODES: &[&str] = &["overlay", "behind"];
pub const BADGE_SHAPES: &[&str] = &["circle", "square", "diamond", "hex", "shield"];

pub fn valid_layout(layout: &str) -> bool {
    LAYOUTS.contains(&layout)
}

pub fn valid_frame_mode(mode: &str) -> bool {
    FRAME_MODES.contains(&mode)
}

pub fn valid_badge_shape(shape: &str) -> bool {
    BADGE_SHAPES.contains(&shape)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleFrames {
    pub frames: Vec<BattleFrame>,
}

impl Default for BattleFrames {
    fn default() -> Self {
        Self {
            frames: default_frames(),
        }
    }
}

/// A card that is not dressed in a photograph. 5 : 7, the ratio of a card held
/// in a hand.
pub const DEFAULT_ASPECT: f32 = 5.0 / 7.0;
pub const DEFAULT_ART_SHARE: f32 = 0.44;
pub const DEFAULT_HEADER_SHARE: f32 = 0.09;
pub const DEFAULT_FOOT_SHARE: f32 = 0.10;
pub const DEFAULT_COST_X: f32 = 10.0;
pub const DEFAULT_COST_Y: f32 = 9.0;
pub const DEFAULT_POWER_X: f32 = 90.0;
pub const DEFAULT_POWER_Y: f32 = 91.0;

fn painted(
    tier: i16,
    name_en: &str,
    name_ru: &str,
    paper: &str,
    ink: &str,
    border: &str,
    foil: &str,
) -> BattleFrame {
    BattleFrame {
        tier,
        name_en: name_en.into(),
        name_ru: name_ru.into(),
        paper: paper.into(),
        ink: ink.into(),
        border: border.into(),
        foil: foil.into(),
        frame_image: String::new(),
        frame_mode: "overlay".into(),
        paper_image: String::new(),
        back_image: String::new(),
        inset_top: 0.0,
        inset_right: 0.0,
        inset_bottom: 0.0,
        inset_left: 0.0,
        aspect: DEFAULT_ASPECT,
        header_share: Some(DEFAULT_HEADER_SHARE),
        art_share: DEFAULT_ART_SHARE,
        foot_share: Some(DEFAULT_FOOT_SHARE),
        title_font: String::new(),
        title_ink: String::new(),
        layout: "corners".into(),
        cost_x: Some(DEFAULT_COST_X),
        cost_y: Some(DEFAULT_COST_Y),
        power_x: Some(DEFAULT_POWER_X),
        power_y: Some(DEFAULT_POWER_Y),
        cost_shape: "circle".into(),
        power_shape: "circle".into(),
    }
}

/// The house palette, five steps deeper. Nothing here glows: rank shows as
/// darker paper and a heavier border, not as a brighter colour.
pub fn default_frames() -> Vec<BattleFrame> {
    vec![
        painted(1, "Humble", "Скромная", "#f8f1e7", "#34251c", "#d8c6b1", ""),
        painted(2, "Sturdy", "Крепкая", "#f3e9db", "#34251c", "#c3ad93", ""),
        painted(3, "Remembered", "Памятная", "#eeddc8", "#34251c", "#a8845f", "rgba(198,95,60,0.16)"),
        painted(4, "Rare", "Редкая", "#e6cfb2", "#2a1a11", "#6f3b24", "rgba(198,95,60,0.28)"),
        painted(5, "Epic", "Эпическая", "#3a2a1e", "#f3e4cd", "#c99a52", "rgba(214,178,110,0.42)"),
    ]
}

/// Keep exactly five frames, one per rank, whatever the keeper saved. A missing
/// rank falls back to its default rather than leaving a card undressed, and a
/// number that would fold the card in half is pulled back into range instead of
/// being refused — the keeper is dragging sliders, not filling in a form.
///
/// Zero is read as "not set" for the two ratios, so frames saved before a card
/// could wear a photograph keep working without a data migration.
pub fn normalize_frames(mut saved: Vec<BattleFrame>) -> Vec<BattleFrame> {
    default_frames()
        .into_iter()
        .map(|fallback| {
            match saved
                .iter()
                .position(|f| clamp_tier(f.tier) == fallback.tier)
            {
                Some(at) => {
                    let mut found = saved.remove(at);
                    found.tier = fallback.tier;
                    if found.name_en.trim().is_empty() {
                        found.name_en = fallback.name_en;
                    }
                    if found.name_ru.trim().is_empty() {
                        found.name_ru = fallback.name_ru;
                    }
                    if found.paper.trim().is_empty() {
                        found.paper = fallback.paper;
                    }
                    if found.ink.trim().is_empty() {
                        found.ink = fallback.ink;
                    }
                    if found.border.trim().is_empty() {
                        found.border = fallback.border;
                    }
                    if !valid_layout(&found.layout) {
                        found.layout = fallback.layout;
                    }
                    if !valid_badge_shape(&found.cost_shape) {
                        found.cost_shape = fallback.cost_shape;
                    }
                    if !valid_badge_shape(&found.power_shape) {
                        found.power_shape = fallback.power_shape;
                    }
                    if !valid_frame_mode(&found.frame_mode) {
                        // A frame saved before cards could wear a cut-out has no
                        // mode, and back then a picture was always the card's
                        // ground. Defaulting it to `overlay` would lay that solid
                        // picture over the card and hide everything on it — so an
                        // unset mode on a frame that already HAS a picture means
                        // `behind`, whatever the default for a fresh frame is.
                        found.frame_mode = if found.frame_image.trim().is_empty() {
                            fallback.frame_mode
                        } else {
                            "behind".into()
                        };
                    }
                    found.frame_image = found.frame_image.trim().to_string();
                    found.paper_image = found.paper_image.trim().to_string();
                    found.back_image = found.back_image.trim().to_string();
                    found.title_font = found.title_font.trim().to_string();
                    found.title_ink = found.title_ink.trim().to_string();
                    found.aspect = if found.aspect > 0.0 {
                        found.aspect.clamp(0.45, 1.4)
                    } else {
                        fallback.aspect
                    };
                    found.art_share = if found.art_share > 0.0 {
                        found.art_share.clamp(0.12, 0.85)
                    } else {
                        fallback.art_share
                    };
                    let mut header = clamp_band(found.header_share, fallback.header_share);
                    let mut foot = clamp_band(found.foot_share, fallback.foot_share);
                    // Capped together with the photograph so the properties band
                    // always keeps room; scaled in proportion rather than
                    // truncated, so the keeper's balance between them survives.
                    let taken = header + found.art_share + foot;
                    if taken > 0.9 {
                        let scale = 0.9 / taken;
                        header *= scale;
                        foot *= scale;
                        found.art_share *= scale;
                    }
                    found.header_share = Some(header);
                    found.foot_share = Some(foot);
                    let (top, bottom) = clamp_pair(found.inset_top, found.inset_bottom);
                    let (left, right) = clamp_pair(found.inset_left, found.inset_right);
                    found.inset_top = top;
                    found.inset_bottom = bottom;
                    found.inset_left = left;
                    found.inset_right = right;
                    found.cost_x = Some(clamp_pos(found.cost_x, fallback.cost_x));
                    found.cost_y = Some(clamp_pos(found.cost_y, fallback.cost_y));
                    found.power_x = Some(clamp_pos(found.power_x, fallback.power_x));
                    found.power_y = Some(clamp_pos(found.power_y, fallback.power_y));
                    found
                }
                None => fallback,
            }
        })
        .collect()
}

/// One band's share of the card. Absent takes the default; present is honoured,
/// including zero. A number that is not a number falls back rather than poisoning
/// the layout with NaN.
fn clamp_band(given: Option<f32>, fallback: Option<f32>) -> f32 {
    let fallback = fallback.unwrap_or(0.0);
    match given {
        Some(v) if v.is_finite() => v.clamp(0.0, 0.3),
        Some(_) => fallback,
        None => fallback,
    }
}

/// A badge's centre, in % of the card. Absent or non-finite falls back to the
/// tier's default position rather than snapping to a corner.
fn clamp_pos(given: Option<f32>, fallback: Option<f32>) -> f32 {
    let fallback = fallback.unwrap_or(0.0);
    match given {
        Some(v) if v.is_finite() => v.clamp(0.0, 100.0),
        _ => fallback,
    }
}

/// Two insets facing each other. Each is capped on its own, and together they
/// are never allowed to close the window they are supposed to open.
fn clamp_pair(a: f32, b: f32) -> (f32, f32) {
    const MOST: f32 = 45.0;
    const TOGETHER: f32 = 85.0;
    let a = if a.is_finite() { a.clamp(0.0, MOST) } else { 0.0 };
    let b = if b.is_finite() { b.clamp(0.0, MOST) } else { 0.0 };
    if a + b <= TOGETHER {
        return (a, b);
    }
    let scale = TOGETHER / (a + b);
    (a * scale, b * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Стол гостя ────────────────────────────────────────────────────
    //
    // Правило колоды — чистая функция, и проверяется оно здесь, а не через
    // базу: то же самое правило перечитывает начало партии, и если оно
    // однажды разойдётся с собой, разойдётся оно молча.

    fn table(
        board: &[(uuid::Uuid, u8, u8)],
        hand: &[uuid::Uuid],
        tiers: &[(uuid::Uuid, i16)],
    ) -> Result<(), DeckFault> {
        let owned: std::collections::HashSet<uuid::Uuid> = board
            .iter()
            .map(|(c, _, _)| *c)
            .chain(hand.iter().copied())
            .collect();
        check_deck(board, hand, &owned, &tiers.iter().copied().collect())
    }

    fn card() -> uuid::Uuid {
        uuid::Uuid::new_v4()
    }

    // ── Потолок чина ──────────────────────────────────────────────────
    //
    // Забор, а не весы. Проверяется здесь потому, что тем же разбором
    // сервер и отказывает при сохранении, и подсказывает, пока хранитель
    // печатает: разойдись они — хранитель увидел бы одно, а получил другое.

    #[test]
    fn a_draft_may_weigh_anything() {
        // Черновик имеет право быть недоделанным — он затем и черновик.
        assert!(card_blockers("draft", 10, 2, 999.0, 1).is_empty());
    }

    #[test]
    fn a_published_card_may_not_outweigh_its_rank() {
        assert_eq!(tier_budget(1), 8.0);
        assert!(card_blockers("published", 10, 2, 8.0, 1).is_empty(), "ровно в бюджет — можно");
        assert_eq!(
            card_blockers("published", 10, 2, 8.01, 1),
            vec!["overTierBudget"],
            "сверх бюджета — на полку не выходит"
        );
        // Тот же вес на чине повыше проходит: чин и есть разрешение.
        assert!(card_blockers("published", 10, 2, 8.01, 2).is_empty());
    }

    #[test]
    fn speed_no_longer_costs_anything() {
        // Характеристика бралась в 2 очка за ступень при том, что в движке её
        // нет вовсе. Тело считается без неё — и подпись это стережёт.
        assert_eq!(body_points(10, 0, 0, 5, 1, 1, 0), 0.5 * 10.0 + 5.0);
    }

    #[test]
    fn a_body_that_does_not_strike_pays_nothing_for_its_power() {
        // Сила напечатана на карте и никогда не превращается в урон.
        assert_eq!(striking_power(5, "physical"), 5);
        assert_eq!(striking_power(5, "none"), 0);
        assert!(!strikes("none"));
        assert!(strikes("magic"));
    }

    #[test]
    fn a_step_costs_a_quarter_of_a_point_a_cell() {
        let at = |step| body_points(10, 0, 0, 5, 1, step, 0);
        assert_eq!(at(1), 10.0);
        assert_eq!(at(2), 10.25);
        assert_eq!(at(3), 10.5);
        // Стоять на месте не даёт скидки: недостаток это настоящий, но
        // неизмеренный, а бюджет из воздуха выдавать нельзя.
        assert_eq!(at(0), 10.0);
        // Весь шаг целиком не дороже одного очка здоровья — тот потолок,
        // который дал замер. Шаг 3 стоит ровно по этому потолку.
        assert!(at(3) - at(1) <= 0.5);
    }

    #[test]
    fn the_ceiling_is_read_at_the_same_rounding_the_keeper_sees() {
        // Хранитель видит 8.0 и должен иметь право сохранить. Без округления
        // до сотых карта ровно в бюджет отказывалась бы из-за двоичной пыли.
        let almost = 8.0_f64 + 1e-14;
        assert!(almost > 8.0, "именно та пыль, ради которой округление и есть");
        assert!(card_blockers("published", 10, 2, almost, 1).is_empty());
        // А настоящий перебор — ловится.
        assert!(!card_blockers("published", 10, 2, 8.02, 1).is_empty());
    }

    #[test]
    fn three_and_three_is_the_table() {
        let (a, b, c) = (card(), card(), card());
        let (d, e, f) = (card(), card(), card());
        assert_eq!(table(&[(a, 0, 4), (b, 1, 4), (c, 2, 4)], &[d, e, f], &[]), Ok(()));
    }

    #[test]
    fn a_fourth_body_does_not_fit() {
        let placed: Vec<_> = (0..4).map(|i| (card(), i as u8 % 3, 3 + i as u8 / 3)).collect();
        assert_eq!(table(&placed, &[], &[]), Err(DeckFault::TooManyOnBoard));
    }

    #[test]
    fn a_fourth_card_does_not_fit_the_hand() {
        let held: Vec<_> = (0..4).map(|_| card()).collect();
        assert_eq!(table(&[], &held, &[]), Err(DeckFault::TooManyInHand));
    }

    /// Половина гостя — ряды 3..5. Ряд 2 принадлежит хранителю, и поставить
    /// туда своё тело значило бы начать партию в его тылу.
    #[test]
    fn the_keepers_half_is_not_yours() {
        assert_eq!(table(&[(card(), 1, 2)], &[], &[]), Err(DeckFault::NotYourHalf));
        assert!(own_half(1, 3) && own_half(1, 5));
        assert!(!own_half(1, 6) && !own_half(3, 4));
    }

    #[test]
    fn two_bodies_do_not_share_a_cell() {
        let (a, b) = (card(), card());
        assert_eq!(table(&[(a, 1, 4), (b, 1, 4)], &[], &[]), Err(DeckFault::CellTaken));
    }

    /// Дублей не бывает по построению (`UNIQUE (user_id, card_id)`), но клиент
    /// построения не знает, и молча проглотить это значило бы выставить два
    /// тела с одним владением.
    #[test]
    fn one_card_stands_in_one_place() {
        let a = card();
        assert_eq!(table(&[(a, 0, 4)], &[a], &[]), Err(DeckFault::SameCardTwice));
    }

    #[test]
    fn a_card_you_do_not_own_is_refused() {
        let mine = card();
        let theirs = card();
        let owned = std::collections::HashSet::from([mine]);
        assert_eq!(
            check_deck(&[(theirs, 1, 4)], &[], &owned, &Default::default()),
            Err(DeckFault::NotYours)
        );
    }

    #[test]
    fn one_card_of_rank_five_and_no_more() {
        let (a, b) = (card(), card());
        assert_eq!(table(&[(a, 0, 4)], &[], &[(a, 5)]), Ok(()));
        assert_eq!(
            table(&[(a, 0, 4), (b, 1, 4)], &[], &[(a, 5), (b, 5)]),
            Err(DeckFault::TooManyOfRankFive)
        );
    }

    #[test]
    fn two_cards_of_rank_four_and_no_more() {
        let (a, b, c) = (card(), card(), card());
        assert_eq!(table(&[(a, 0, 4), (b, 1, 4)], &[], &[(a, 4), (b, 4)]), Ok(()));
        assert_eq!(
            table(&[(a, 0, 4), (b, 1, 4), (c, 2, 4)], &[], &[(a, 4), (b, 4), (c, 4)]),
            Err(DeckFault::TooManyOfRankFour)
        );
    }

    /// Пустой стол законен. Он не бесполезен: всё, чего гость не поставил,
    /// закрывает дом, и новый гость садится играть, ничем не владея.
    #[test]
    fn an_empty_table_is_legal() {
        assert_eq!(table(&[], &[], &[]), Ok(()));
    }

    /// Клетки по умолчанию обязаны лежать на своей половине и не совпадать —
    /// иначе дом положил бы два заёма на одно место.
    #[test]
    fn the_default_cells_are_on_your_half_and_distinct() {
        let cells: std::collections::HashSet<_> = DECK_DEFAULT_CELLS.iter().collect();
        assert_eq!(cells.len(), DECK_DEFAULT_CELLS.len());
        assert!(DECK_DEFAULT_CELLS.iter().all(|(x, y)| own_half(*x, *y)));
    }

    #[test]
    fn slug_avoids_api_doors() {
        assert_eq!(unique_slug(Some("cards"), "anything", &[]), "card-cards");
    }

    #[test]
    fn slug_walks_past_taken() {
        let taken = vec!["raven".to_string(), "raven-2".to_string()];
        assert_eq!(unique_slug(None, "Raven", &taken), "raven-3");
    }

    #[test]
    fn effect_is_cut_not_refused() {
        let long = "ы".repeat(400);
        assert_eq!(clamp_effect(Some(&long)).unwrap().chars().count(), EFFECT_MAX);
    }

    #[test]
    fn blank_text_becomes_none() {
        assert!(clamp_effect(Some("   ")).is_none());
    }

    #[test]
    fn missing_price_is_not_free() {
        assert_eq!(clamp_price(Some(-5)), None);
        assert_eq!(clamp_price(Some(0)), Some(0));
    }

    #[test]
    fn focal_is_clamped_and_broken_focus_is_dropped() {
        let out = normalize_focal(Some(r#"{"x":2.0,"y":-1.0,"zoom":9.0}"#)).unwrap();
        let back: ArtFocal = serde_json::from_str(&out).unwrap();
        assert_eq!((back.x, back.y, back.zoom), (1.0, 0.0, 3.0));
        assert!(normalize_focal(Some("not json")).is_none());
    }

    #[test]
    fn frames_always_come_back_five_deep() {
        let mut mine = painted(3, "", "Своя", "#000000", "", "#111111", "");
        mine.ink = String::new();
        let frames = normalize_frames(vec![mine]);
        assert_eq!(frames.len(), 5);
        assert_eq!(frames[2].paper, "#000000");
        assert_eq!(frames[2].name_ru, "Своя");
        // Blanks fall back rather than shipping an undressed rank.
        assert_eq!(frames[2].name_en, "Remembered");
        assert!(!frames[2].ink.is_empty());
    }

    #[test]
    fn a_frame_saved_before_photographs_keeps_working() {
        // Exactly what an older `battle_frames` setting deserialises to: every
        // new field at its zero value. Zero ratios must read as "not set".
        let old = serde_json::json!({
            "tier": 2, "nameEn": "Sturdy", "nameRu": "Крепкая",
            "paper": "#f3e9db", "ink": "#34251c", "border": "#c3ad93", "foil": ""
        });
        let frame: BattleFrame = serde_json::from_value(old).unwrap();
        let frames = normalize_frames(vec![frame]);
        assert_eq!(frames[1].aspect, DEFAULT_ASPECT);
        assert_eq!(frames[1].art_share, DEFAULT_ART_SHARE);
        // Absent, so the default — a frame written before cards had bands must
        // not come back with no header at all.
        assert_eq!(frames[1].header_share, Some(DEFAULT_HEADER_SHARE));
        assert_eq!(frames[1].foot_share, Some(DEFAULT_FOOT_SHARE));
        assert_eq!(frames[1].layout, "corners");
        // An empty mode is not a mode; a frame must know how it is worn.
        assert_eq!(frames[1].frame_mode, "overlay");
    }

    #[test]
    fn the_properties_band_always_keeps_room() {
        let mut mine = painted(2, "a", "а", "#fff", "#000", "#ccc", "");
        mine.header_share = Some(0.3);
        mine.art_share = 0.85;
        mine.foot_share = Some(0.3);
        let frames = normalize_frames(vec![mine]);
        let f = &frames[1];
        let taken = f.header_share.unwrap() + f.art_share + f.foot_share.unwrap();
        assert!(taken <= 0.9001, "three bands took {taken}");
        // Scaled in proportion, not truncated: the keeper's balance survives.
        assert!(f.art_share > f.header_share.unwrap());
    }

    #[test]
    fn a_header_may_be_nothing_at_all() {
        let mut mine = painted(1, "a", "а", "#fff", "#000", "#ccc", "");
        mine.header_share = Some(0.0);
        mine.foot_share = Some(0.0);
        let frames = normalize_frames(vec![mine]);
        // Zero is a choice here, unlike the photograph's share, where it means
        // "not set" — a card with no picture band would have nowhere to look.
        assert_eq!(frames[0].header_share, Some(0.0));
        assert_eq!(frames[0].foot_share, Some(0.0));
        assert_eq!(frames[0].art_share, DEFAULT_ART_SHARE);
    }

    #[test]
    fn a_card_becomes_a_body_the_engine_can_fight_with() {
        let mut card = sample_card();
        card.attack_channel = "magic".into();
        card.armor = 2;
        card.ward = 3;
        card.reach = 4;
        card.step = 0;
        card.mend = 5;

        let body = to_snapshot(&card);
        assert_eq!(body.name, "vedma", "в журнал едет слаг, а не заголовок");
        assert_eq!(body.channel, battle_core::Channel::Magic);
        assert_eq!(body.armor, 2);
        assert_eq!(body.ward, 3);
        assert_eq!(body.reach, 4);
        assert_eq!(body.step, 0);
        assert_eq!(body.mend, 5);
    }

    #[test]
    fn cards_from_the_archive_actually_play_a_match() {
        // Ради этого теста движок и подключали: он проходит через обе половины —
        // карта дома превращается в тело, тело воюет, партия кончается исходом.
        // Если однажды перевод разъедется с движком, упадёт здесь, а не у игрока.
        use battle_core::{Action, Cell, MatchState, Outcome, Setup, bot, reduce};

        let mut boec = sample_card();
        boec.health = 6;
        boec.power = 3;

        let mut strelok = sample_card();
        strelok.slug = "strelok".into();
        strelok.health = 4;
        strelok.power = 2;
        strelok.reach = 3;

        let setup = Setup {
            player_board: vec![
                (to_snapshot(&boec), Cell::new(1, 4).unwrap()),
                (to_snapshot(&strelok), Cell::new(0, 5).unwrap()),
            ],
            keeper_board: vec![
                (to_snapshot(&boec), Cell::new(1, 1).unwrap()),
                (to_snapshot(&strelok), Cell::new(2, 0).unwrap()),
            ],
            ..Default::default()
        };

        let mut state = MatchState::begin(setup);
        let mut journal = Vec::new();
        let mut guard = 0;
        while state.outcome.is_none() {
            let action: Action = bot::choose(&state);
            let (next, events) = reduce(&state, &action).expect("законное действие");
            state = next;
            journal.extend(events);
            guard += 1;
            assert!(guard < 2000, "партия не кончилась");
        }

        assert!(matches!(
            state.outcome,
            Some(Outcome::Player) | Some(Outcome::Keeper) | Some(Outcome::Draw)
        ));
        assert!(!journal.is_empty());
        // И журнал ложится в базу как есть — колонка JSONB его примет.
        assert!(serde_json::to_string(&journal).is_ok());
    }

    #[test]
    fn a_hand_edited_channel_falls_back_instead_of_refusing_the_match() {
        assert_eq!(to_channel("огненный"), battle_core::Channel::Physical);
        assert_eq!(to_channel("pure"), battle_core::Channel::Pure);
    }

    #[test]
    fn a_card_without_health_never_takes_the_field() {
        let mut card = sample_card();
        assert!(can_take_the_field(&card));

        card.health = 0;
        assert!(!can_take_the_field(&card), "упало бы на первом же тике");

        card.health = 6;
        card.status = "draft".into();
        assert!(!can_take_the_field(&card), "черновик не воюет");
    }

    fn sample_card() -> crate::models::BattleCardDto {
        crate::models::BattleCardDto {
            id: String::new(),
            slug: "vedma".into(),
            status: "published".into(),
            tier: 3,
            race_id: None,
            race_name_en: None,
            race_name_ru: None,
            race_icon_url: None,
            race_level_frames: None,
            type_en: None,
            type_ru: None,
            title_en: "Witch".into(),
            title_ru: "Ведьма".into(),
            effect_en: None,
            effect_ru: None,
            lore_en: None,
            lore_ru: None,
            cost: 3,
            power: 4,
            health: 6,
            mana: 0,
            traits: Vec::new(),
            kind: "unit".into(),
            armor: 0,
            ward: 0,
            attack_channel: "physical".into(),
            reach: 1,
            step: 1,
            speed: 3,
            mend: 0,
            abilities: Vec::new(),
            budget_points: None,
            balance_index: None,
            rules_version: 1,
            price_dust: Some(10),
            price_feed: None,
            level_price_dust: None,
            art_url: None,
            art_url_override: None,
            art_focal: None,
            frame_override: None,
            shelf_order: None,
            lendable: false,
            figurine_id: None,
            figurine_name: None,
            figurine_slug: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn abilities_with_an_unknown_verb_are_dropped_rather_than_stored() {
        // An unknown verb is not a card the engine could ever run, and keeping
        // it would mean the shelf shows a rule that never fires.
        let abilities = vec![
            CardAbility {
                verb: "damage".into(),
                amount: 5,
                ..blank_ability()
            },
            CardAbility {
                verb: "разбудить дракона".into(),
                amount: 5,
                ..blank_ability()
            },
        ];
        let back = read_abilities(normalize_abilities(&abilities).as_deref());
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].verb, "damage");
    }

    #[test]
    fn an_unknown_shape_or_trigger_falls_back_instead_of_refusing_the_card() {
        let abilities = vec![CardAbility {
            verb: "damage".into(),
            shape: "повсюду".into(),
            trigger: "когда-нибудь".into(),
            channel: "огненный".into(),
            ..blank_ability()
        }];
        let back = read_abilities(normalize_abilities(&abilities).as_deref());
        assert_eq!(back[0].shape, "one");
        assert_eq!(back[0].trigger, "active");
        assert_eq!(back[0].channel, "physical");
    }

    #[test]
    fn the_scales_price_a_mass_spell_by_the_book() {
        // Ураган эпического мага: 8 урона, радиус 2, дальность 3, чарный.
        // 8 × 3.2 × 1.0 × 1.05 × 1.2 = 32.256
        let hurricane = CardAbility {
            verb: "damage".into(),
            channel: "magic".into(),
            amount: 8,
            shape: "radius".into(),
            radius: 2,
            range: 3,
            ..blank_ability()
        };
        let points = ability_points(&hurricane, 5);
        assert!((points - 32.256).abs() < 0.001, "получилось {points}");
        // Одна способность уже перебирает бюджет пятого чина.
        assert!(points > tier_budget(5));
    }

    #[test]
    fn control_is_priced_against_the_rank_it_stops() {
        // Лишение действия стоит дороже на высоком чине: там действие дороже.
        let stun = CardAbility {
            verb: "control".into(),
            duration: 2,
            ..blank_ability()
        };
        assert!(ability_points(&stun, 5) > ability_points(&stun, 1));
    }

    #[test]
    fn the_verdict_moves_with_the_price_and_not_with_the_rank() {
        let points = 24.0;
        // Дорогая карта на кривой; та же сила задёшево — перегруз.
        assert!((balance_index(points, 5) - 24.0 / 22.0).abs() < 0.001);
        assert!(balance_index(points, 5) < 1.15);
        assert!(balance_index(points, 2) > 1.15);
    }

    fn blank_ability() -> CardAbility {
        CardAbility {
            id: String::new(),
            name_en: String::new(),
            name_ru: String::new(),
            verb: "damage".into(),
            channel: default_channel(),
            amount: 0,
            shape: default_shape(),
            radius: 0,
            range: 1,
            duration: 0,
            trigger: default_trigger(),
            mana_cost: 0,
            cooldown: 0,
            keywords: Vec::new(),
        }
    }

    #[test]
    fn traits_drop_the_nameless_and_cut_the_long() {
        let long = "я".repeat(400);
        let traits = vec![
            CardTrait { name_en: "Wind".into(), name_ru: "Вихрь".into(), text_en: long.clone(), text_ru: "х".into() },
            CardTrait { name_en: "  ".into(), name_ru: "".into(), text_en: "orphan".into(), text_ru: "".into() },
        ];
        let stored = normalize_traits(&traits).unwrap();
        let back = read_traits(Some(&stored));
        assert_eq!(back.len(), 1, "a nameless property is not a property");
        assert_eq!(back[0].text_en.chars().count(), TRAIT_TEXT_MAX);
        assert!(read_traits(Some("not json")).is_empty());
        assert!(normalize_traits(&[]).is_none());
    }

    #[test]
    fn an_old_picture_frame_is_still_worn_behind() {
        // Saved when a picture could only be the card's ground. Read as an
        // overlay it would cover the whole card, so it must stay behind.
        let old = serde_json::json!({
            "tier": 4, "nameEn": "Rare", "nameRu": "Редкая",
            "paper": "#e6cfb2", "ink": "#2a1a11", "border": "#6f3b24", "foil": "",
            "frameImage": "/static/images/preview/carved.jpg",
            "insetTop": 8.0, "insetRight": 9.0, "insetBottom": 8.0, "insetLeft": 9.0,
            "aspect": 0.8, "artShare": 0.5, "layout": "plaque"
        });
        let frame: BattleFrame = serde_json::from_value(old).unwrap();
        let frames = normalize_frames(vec![frame]);
        assert_eq!(frames[3].frame_mode, "behind");
        // A fresh frame with no picture keeps the modern default.
        assert_eq!(frames[0].frame_mode, "overlay");
    }

    #[test]
    fn insets_can_never_close_the_window() {
        let mut mine = painted(1, "a", "а", "#fff", "#000", "#ccc", "");
        mine.inset_top = 90.0;
        mine.inset_bottom = 90.0;
        mine.aspect = 99.0;
        mine.art_share = -3.0;
        mine.layout = "nonsense".into();
        let frames = normalize_frames(vec![mine]);
        assert!(frames[0].inset_top + frames[0].inset_bottom <= 85.0);
        assert!(frames[0].inset_top > 0.0);
        assert_eq!(frames[0].aspect, 1.4);
        // A negative share is not "set", so it falls back rather than clamping.
        assert_eq!(frames[0].art_share, DEFAULT_ART_SHARE);
        assert_eq!(frames[0].layout, "corners");
    }
}
