//! Студия — комната, где человек делает свою рамку.
//!
//! Здесь живут только НАСТРОЙКИ комнаты: потолки склада, квоты и ворота.
//! Сама рамка — это `battles::BattleFrame`, тот же объект, которым одеваются
//! карты дома, и второго его описания в доме нет и не будет: отрисовщик один,
//! нормализация одна, схема одна.
//!
//! Все числа взяты замером живого склада 7.09.2026 (`STUDIO.md` §3), а не
//! выбраны круглыми.

use serde::{Deserialize, Serialize};

/// Редакция соглашения автора.
///
/// Меняется РУКОЙ, вместе с текстом, и это условие: человек, принявший
/// прошлую редакцию, новой не принимал, и молча считать иначе нельзя. Дом
/// спросит согласия заново — один раз, перед следующей выкладкой.
pub const AGREEMENT: &str = "2026-09-08";

/// Ворота студии: кого пускают за стол.
pub const STUDIO_GATES: &[&str] = &["all", "owners", "closed"];

/// Потолки, за которыми настройка перестаёт быть настройкой и становится
/// способом уронить сервер. Зажимаются молча — это забор от опечатки в поле
/// ввода, а не согласование с хозяином.
const BOX_BYTES_MAX: i64 = 512 * 1024 * 1024;
const ASSET_BYTES_MAX: i64 = 32 * 1024 * 1024;

/// Настройки комнаты.
///
/// `#[serde(default)]` на всей записи — условие, а не украшение: настройки уже
/// лежат в базе, и добавленное завтра поле сломало бы чтение вчерашних. Дом
/// уже говорит так везде: пустое значит «как в доме», и недостающее поле
/// падает домой, а не роняет комнату целиком.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StudioSettings {
    /// `all` — всякому с именем, `owners` — у кого есть хоть одна карта,
    /// `closed` — студия заперта.
    pub gate: String,
    /// Личный ящик на человека. 50 МБ — это 40–100 рамок по замеру.
    pub box_bytes: i64,
    /// Одна деталь. Настоящий максимум на складе дома — 438 КБ; потолок впятеро
    /// выше, то есть запас, а не забор.
    pub asset_bytes: i64,
    /// Одна рамка со всеми деталями. По замеру рамка весит 0.3–1.5 МБ.
    pub frame_bytes: i64,
    /// Сколько деталей может нести одна рамка: шесть слотов плюс орнаменты.
    pub frame_pieces: i32,
    /// Сколько рамок человек держит в работе одновременно.
    pub frames_open: i32,
    /// Листов на разрез в сутки.
    pub sheets_per_day: i32,
    /// Листов ОДНОВРЕМЕННО. Второй потолок, и он не лишний: лист весит 2.6 МБ
    /// против 12 КБ у детали — в двести раз больше. Без него суточная квота
    /// забивает весь ящик за один день, и человек, нарезавший вчера, сегодня не
    /// может положить готовую работу.
    pub sheets_at_once: i32,
    /// Сколько дней лежит лист, из которого ничего не вырезали.
    pub sheet_days: i32,
    /// Сколько оценок человек раздаёт за сезон.
    pub ratings_per_season: i64,
    /// Ниже этого числа оценок работа в тройку не проходит.
    ///
    /// Настройкой, а не только константой: первые сезоны идут при десятке
    /// участников, и порог, рассчитанный на живой дом, тогда не пропустит
    /// никого — то есть первые недели пройдут впустую. Смягчить его на старте
    /// должен хозяин, не программист.
    pub ratings_trusted: i64,
    /// Сколько работ уходит хозяину.
    pub to_keeper: i64,
    /// Сколько незаконченных КАРТ человек держит одновременно. Своим числом,
    /// а не рамочным: карта весит не то, что рама, и общая ручка однажды
    /// подвинула бы обе разом.
    pub cards_open: i64,
    /// Коридор цены в лавке. Без потолка первый же автор поставит миллион и
    /// лавка встанет; без пола её зальют мусором по единице.
    pub price_floor: i32,
    pub price_ceil: i32,
    /// Доля дома со сделки, в сотых. СГОРАЕТ: это единственный сток валюты, и
    /// без него за месяц цены перестанут что-то значить.
    pub commission_percent: i32,
}

impl Default for StudioSettings {
    fn default() -> Self {
        Self {
            gate: "owners".into(),
            box_bytes: 50 * 1024 * 1024,
            asset_bytes: 2 * 1024 * 1024,
            frame_bytes: 6 * 1024 * 1024,
            frame_pieces: 20,
            frames_open: 10,
            sheets_per_day: 20,
            sheets_at_once: 5,
            sheet_days: 7,
            ratings_per_season: RATINGS_PER_SEASON,
            ratings_trusted: RATINGS_TRUSTED,
            to_keeper: TO_KEEPER as i64,
            cards_open: 10,
            price_floor: 50,
            price_ceil: 5000,
            commission_percent: 10,
        }
    }
}

/// Настройки, приведённые в чувство. Неизвестные ворота — домашние: слово,
/// которого дом не знает, не должно запирать студию и не должно открывать её
/// настежь.
pub fn normalize_studio_settings(mut s: StudioSettings) -> StudioSettings {
    let home = StudioSettings::default();
    if !STUDIO_GATES.contains(&s.gate.trim()) {
        s.gate = home.gate.clone();
    } else {
        s.gate = s.gate.trim().to_string();
    }
    s.box_bytes = s.box_bytes.clamp(1024 * 1024, BOX_BYTES_MAX);
    s.asset_bytes = s.asset_bytes.clamp(64 * 1024, ASSET_BYTES_MAX);
    // Рамка не может быть тяжелее ящика: потолок, которого нельзя достичь,
    // не потолок, а обещание, что место найдётся.
    s.frame_bytes = s.frame_bytes.clamp(256 * 1024, s.box_bytes);
    s.frame_pieces = s.frame_pieces.clamp(1, 64);
    s.frames_open = s.frames_open.clamp(1, 100);
    s.cards_open = s.cards_open.clamp(1, 100);
    s.sheets_per_day = s.sheets_per_day.clamp(0, 200);
    s.sheets_at_once = s.sheets_at_once.clamp(1, 50);
    s.sheet_days = s.sheet_days.clamp(1, 90);
    s.ratings_per_season = s.ratings_per_season.clamp(1, 100);
    // Ноль голосов доверия — это «в тройку попадает то, чего никто не видел».
    s.ratings_trusted = s.ratings_trusted.clamp(1, 100);
    s.to_keeper = s.to_keeper.clamp(1, 10);
    s.price_floor = s.price_floor.clamp(1, 1_000_000);
    // Потолок ниже пола — это лавка, в которой нельзя выставить ничего.
    s.price_ceil = s.price_ceil.clamp(s.price_floor, 1_000_000);
    // Сотня процентов — это «дом забирает всё»; продавать тогда незачем.
    s.commission_percent = s.commission_percent.clamp(0, 50);
    s
}

// ── Сезон ────────────────────────────────────────────────────────────────────

/// Числа сезона. Все они — настройки комнаты, но живут здесь, а не в
/// `StudioSettings`, потому что их не крутят: это правила счёта, и меняют их
/// вместе с объяснением людям, а не ползунком в среду.

/// Сколько оценок человек раздаёт за сезон. Десять — чтобы смотрели, а не
/// проставляли всем подряд.
pub const RATINGS_PER_SEASON: i64 = 10;

/// Сколько работ уходит хозяину.
pub const TO_KEEPER: usize = 3;

/// Ниже этого числа оценок работа в тройку не проходит.
///
/// Без порога тройку займут те, кого никто не смотрел: у работы с одной
/// пятёркой средний балл выше, чем у работы с пятьюдесятью по 4.8.
pub const RATINGS_TRUSTED: i64 = 10;

/// Поправка Байеса: `m` голосов по `C` баллов, приписанных каждой работе.
///
/// Пока оценок мало, итог притянут к середине; чем больше живых голосов, тем
/// меньше влияние поправки. Одна формула вместо среднего, и объясняется она
/// людям одной строкой: «пока работу мало кто видел, оценка осторожная».
pub const BAYES_M: f64 = 10.0;
pub const BAYES_C: f64 = 3.0;

/// Сколько раз одну раму можно выставить.
pub const ENTRIES_PER_FRAME: i16 = 2;

/// Сколько дней ждёт деталь, не попавшая ни в одну раму.
///
/// Месяц, а не неделя: деталь режут впрок и ставят в раму через две недели, и
/// уборка раньше срока — это выброшенная работа, а не убранный мусор. С листом
/// иначе: он нужен, пока идёт разрез.
pub const ORPHAN_DAYS: i32 = 30;

/// Плата за сезон, в пыли.
///
/// Платят НЕ ТОЛЬКО тройке: из двадцати авторов побеждают трое, и без платы
/// остальным семнадцать второго сезона не будет. За доведённую до сезона работу
/// — всем; тем, чей итог выше среднего по сезону, — добавка. Не за место: место
/// одно, а понравиться могут многим.
pub const PAY_ENTERED: i32 = 5;
pub const PAY_LIKED: i32 = 10;

/// Вес голоса.
///
/// Регистрация в доме дешёвая, и голос, весящий у всех одинаково, отдаёт исход
/// тому, кто завёл больше почтовых ящиков. Вес не отсекает никого — новый
/// человек голосует с первого дня, — но накрутка становится дорогой: пять
/// мультиаккаунтов весят меньше одного игрока.
pub const WEIGHT_BASE: f64 = 0.2;
pub const WEIGHT_AGED: f64 = 0.3;
pub const WEIGHT_OWNS: f64 = 0.25;
pub const WEIGHT_PLAYED: f64 = 0.25;
/// С какого возраста аккаунт считается не вчерашним.
pub const WEIGHT_AGE_DAYS: i64 = 14;

pub fn voter_weight(aged: bool, owns: bool, played: bool) -> f64 {
    let mut w = WEIGHT_BASE;
    if aged {
        w += WEIGHT_AGED;
    }
    if owns {
        w += WEIGHT_OWNS;
    }
    if played {
        w += WEIGHT_PLAYED;
    }
    w.min(1.0)
}

/// Байесовский средний балл. `sum` — сумма «оценка × вес», `weight` — сумма
/// весов.
pub fn bayes_score(sum: f64, weight: f64) -> f64 {
    (sum + BAYES_M * BAYES_C) / (weight + BAYES_M)
}

/// Роли деталей студии. Те же слова, что у склада дома, плюс бумага.
/// Два словаря для одного и того же однажды разошлись бы.
pub const STUDIO_ASSET_ROLES: &[&str] = &[
    "corner", "sideH", "sideV", "accent", "art", "paper", "other",
];

pub fn clamp_asset_role(role: &str) -> String {
    let r = role.trim();
    if STUDIO_ASSET_ROLES.contains(&r) {
        r.to_string()
    } else {
        "other".to_string()
    }
}

/// Состояние работы — ТОЛЬКО про автора: что он может с ней сделать.
///
/// Три слова, а не пять. Решение хозяина живёт отметками (`admitted_at`,
/// `approved_at`), участие в неделе — в `studio_entries`. Когда всё это было
/// свалено в один столбец, работа, побывавшая в сезоне, застревала навсегда:
/// слово `entered` никто не снимал, а править и выставлять заново позволено
/// было только другим словам (`STUDIO-REVIEW.md`).
pub const STUDIO_FRAME_STATUSES: &[&str] = &["draft", "shown", "withdrawn"];

/// Можно ли править. Отданное хозяину — нельзя: он смотрит одно, а на люди
/// вышло бы другое. Снятое — можно: это и есть способ починить и показать
/// снова.
pub fn frame_may_edit(status: &str) -> bool {
    matches!(status, "draft" | "withdrawn")
}

/// Можно ли отдать хозяину.
pub fn frame_may_publish(status: &str) -> bool {
    matches!(status, "draft" | "withdrawn")
}

/// Почему работа не идёт в неделю — или ничего, если идёт.
///
/// Правило вынесено из службы СЮДА нарочно. Пока оно жило тремя `if` внутри
/// запроса к базе, проверить его было нечем: любая проверка требовала базы,
/// сессии и открытого сезона. Именно в этих трёх `if` и сидел тупик, из-за
/// которого работа, побывавшая в сезоне, не выставлялась больше никогда.
pub fn why_not_in_season(
    status: &str,
    admitted: bool,
    entries_used: i16,
    cap: i16,
) -> Option<&'static str> {
    if !admitted {
        return Some("notAdmitted");
    }
    if status != "shown" {
        return Some("notReady");
    }
    if entries_used >= cap {
        return Some("frameSpent");
    }
    None
}

/// Что работа может сделать из этого состояния. Список действий, а не текст:
/// по нему проверяется, что из каждого состояния есть выход.
pub fn moves_from(status: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    if frame_may_edit(status) {
        out.push("edit");
    }
    if frame_may_publish(status) {
        out.push("show");
    }
    if why_not_in_season(status, true, 0, ENTRIES_PER_FRAME).is_none() {
        out.push("enter");
    }
    out
}

// ── Свои карты ──────────────────────────────────────────────────────────────

/// Поля карты, которые назначает ДОМ, а не человек.
///
/// Не «то, что сложно» и не «то, что сломает», а хозяйство полки и оформление:
/// цена — дело лавки, слуг уникален и по нему на карту ссылаются испытания,
/// наряд несёт весь дом и одет сразу на все карты своего чина. Содержимое —
/// имя, числа, черты, способности, картинка — человека.
///
/// Числа НЕ отнимаются, и `tier` в том числе: чин здесь бюджет
/// (`tier_budget = 8 + 6·(чин−1)`), а не награда, и взявший пятый получил не
/// сильную карту, а разрешение весить 32 очка, которое надо ещё чем-то
/// заполнить.
pub const HOUSE_CARD_FIELDS: &[&str] = &[
    "slug",
    "status",
    "priceDust",
    "priceFeed",
    "levelPriceDust",
    "figurineId",
    "lendable",
    "frameOverride",
    "motionWear",
    "shelfOrder",
    "editionSize",
];

/// Тело работы, из которого вырезано домовое.
///
/// Вырезает СЕРВЕР, а не страница: клиент не забор. И вырезает ДВАЖДЫ — при
/// записи и ещё раз при утверждении, — потому что запись могла лечь до
/// последней правки этого списка, а утверждение последний рубеж.
///
/// Чистая функция над `serde_json::Value`, чтобы проверялась без базы: пока
/// вырезание жило внутри запроса, проверить его можно было только подняв базу
/// и сессию, то есть никогда.
pub fn guest_body(body: &serde_json::Value) -> serde_json::Value {
    let mut out = body.clone();
    if let Some(map) = out.as_object_mut() {
        for field in HOUSE_CARD_FIELDS {
            map.remove(*field);
        }
    }
    out
}

/// Тело работы, ГОТОВОЕ ЧИТАТЬСЯ как запрос карты.
///
/// Вырезав домовое, мы вырезали и `status` — единственное домовое поле, у
/// которого нет умолчания в разборе, — и тело перестало читаться вовсе. Класть
/// сюда «черновик» правильно, а не удобно: работа человека и ЕСТЬ черновик
/// карты, пока хозяин не сказал иного, и назначить ей «опубликована» не может
/// никто, кроме него.
///
/// Одной функцией, потому что читается тело в двух местах — при записи и при
/// утверждении, — и подставить умолчание в одном, забыв о другом, значит
/// получить работу, которую нельзя утвердить.
pub fn guest_card_json(body: &serde_json::Value) -> serde_json::Value {
    let mut kept = guest_body(body);
    if let Some(map) = kept.as_object_mut() {
        map.insert("status".into(), serde_json::Value::String("draft".into()));
    }
    kept
}

/// Пыль автору за утверждённую карту.
///
/// Платят за ПРИНЕСЁННОЕ, а не за то, сколько раз карту купили: плата за
/// покупки превратила бы студию в ферму — тот же довод, по которому пыль
/// даётся за испытание, а не за победу.
pub const PAY_CARD_APPROVED: i32 = 50;

/// Сколько длится торг по умолчанию.
///
/// Неделя, как и сезон: дом живёт неделями, и вторая мерка времени в нём была
/// бы вторым календарём, который надо помнить.
pub const AUCTION_DAYS: i64 = 7;

/// Насколько ставка должна перебить прежнюю.
///
/// Не про доход, а про то, чтобы торг не превращался в лестницу по пылинке:
/// сто ставок с шагом в единицу — это не торг, а очередь.
pub const BID_STEP: i32 = 10;

/// Ставка в последние минуты продлевает торг на столько же.
///
/// Иначе весь аукцион решается в последнюю секунду, и выигрывает не тот, кто
/// дал больше, а тот, у кого быстрее рука.
pub const SNIPE_MINUTES: i64 = 10;

/// Сколько вещей кладут в одну мену. Не про баланс, а про опечатку и про то,
/// чтобы список на странице оставался списком.
pub const TRADE_ITEMS_MAX: usize = 6;

pub fn default_lang() -> String {
    "ru".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Домовое поле, присланное человеком, в работу не попадает.
    ///
    /// Проверяется не «функция удаляет ключ», а свойство, ради которого она
    /// есть: из ЛЮБОГО тела, каким бы полным оно ни пришло, не остаётся ни
    /// одного домового поля — и при этом остаётся всё содержимое.
    #[test]
    fn nothing_of_the_house_survives_a_guest_body() {
        let mut sent = serde_json::json!({
            "titleRu": "Ведьма",
            "cost": 4,
            "power": 5,
            "abilities": [{"verb": "damage"}],
        });
        // Кладём В ТЕЛО все домовые поля разом — ровно то, что сделал бы
        // человек, отправивший запрос мимо страницы.
        for field in HOUSE_CARD_FIELDS {
            sent[*field] = serde_json::json!("подсунуто");
        }

        let kept = guest_body(&sent);
        for field in HOUSE_CARD_FIELDS {
            assert!(
                kept.get(*field).is_none(),
                "домовое поле {field} осталось в работе"
            );
        }
        assert_eq!(kept["titleRu"], "Ведьма", "имя человека на месте");
        assert_eq!(kept["cost"], 4, "числа человека на месте");
        assert!(kept["abilities"].is_array(), "способности на месте");
    }

    /// Цена — не «одно из полей», а то, ради чего список вообще заведён:
    /// человек, назначивший своей карте цену в одну пылинку, обнёс бы лавку.
    #[test]
    fn the_price_is_never_the_guests_to_name() {
        let sent = serde_json::json!({"titleRu": "Даром", "priceDust": 1, "priceFeed": 0});
        let kept = guest_body(&sent);
        assert!(kept.get("priceDust").is_none());
        assert!(kept.get("priceFeed").is_none());
    }

    /// Тупик, который стоил недели: работа, побывавшая в сезоне, застревала
    /// навсегда — `entered` не снимал никто, а править и выставлять позволено
    /// было только другим словам.
    ///
    /// Проверяется не «слово А ведёт к слову Б», а свойство: ИЗ КАЖДОГО
    /// состояния есть выход. Такой тест переживает переименования и ловит
    /// именно то, что случилось.
    #[test]
    fn no_state_a_work_cannot_leave() {
        for status in STUDIO_FRAME_STATUSES {
            let moves = moves_from(status);
            assert!(
                !moves.is_empty(),
                "из состояния «{status}» работа не может сделать ничего — это тупик"
            );
        }
    }

    /// Вторая попытка обещана числом `ENTRIES_PER_FRAME`. Пока правило жило в
    /// службе, обещание было недостижимо, и никто этого не видел.
    #[test]
    fn a_work_that_ran_in_a_season_can_run_again() {
        assert_eq!(why_not_in_season("shown", true, 1, ENTRIES_PER_FRAME), None);
        assert_eq!(
            why_not_in_season("shown", true, ENTRIES_PER_FRAME, ENTRIES_PER_FRAME),
            Some("frameSpent")
        );
        assert_eq!(why_not_in_season("shown", false, 0, 2), Some("notAdmitted"));
        assert_eq!(why_not_in_season("draft", true, 0, 2), Some("notReady"));
    }

    /// Слова состояния должны совпадать с тем, что разрешает схема. Читается
    /// сама миграция: рассинхрон кода и `CHECK` — это отказ базы в проде, а не
    /// красная строка в тесте.
    #[test]
    fn the_words_match_what_the_schema_allows() {
        let sql = include_str!("../migrations/20260920000000_studio_frame_states.sql");
        let check = sql
            .split("CHECK (status IN (")
            .nth(1)
            .expect("в миграции нет CHECK по состояниям");
        for word in STUDIO_FRAME_STATUSES {
            assert!(
                check.contains(&format!("'{word}'")),
                "схема не знает слова «{word}»"
            );
        }
    }

    /// Настройки уже лежат в базе. Поле, добавленное завтра, не должно ломать
    /// чтение вчерашней записи — а сломало: комната отвечала ошибкой на всё,
    /// пока на записи не появился `serde(default)`.
    #[test]
    fn settings_saved_before_a_field_existed_still_read() {
        let old = r#"{"gate":"all","boxBytes":52428800}"#;
        let read: StudioSettings = serde_json::from_str(old).expect("старая запись не прочиталась");
        assert_eq!(read.gate, "all");
        assert_eq!(read.ratings_trusted, RATINGS_TRUSTED, "новое поле упало домой");
    }

    /// Числа настроек — забор от опечатки, а не от умысла. Проверяется то, что
    /// невозможно увидеть глазами: обещание, которого нельзя достичь.
    #[test]
    fn a_frame_can_never_be_heavier_than_the_box() {
        let s = normalize_studio_settings(StudioSettings {
            box_bytes: 4 * 1024 * 1024,
            frame_bytes: 64 * 1024 * 1024,
            ..Default::default()
        });
        assert_eq!(s.frame_bytes, s.box_bytes);
    }

    /// Незнакомое слово ворот не должно ни запирать студию, ни открывать её
    /// настежь: и то и другое — тихая беда.
    #[test]
    fn unknown_gate_falls_home_rather_than_locking_or_opening() {
        let s = normalize_studio_settings(StudioSettings {
            gate: "everyone-please".into(),
            ..Default::default()
        });
        assert_eq!(s.gate, "owners");
    }

    /// Ноль голосов доверия значит «в тройку попадает то, чего никто не
    /// видел». Это не настройка, это отказ от порога.
    #[test]
    fn trust_can_be_softened_but_never_turned_off() {
        let s = normalize_studio_settings(StudioSettings {
            ratings_trusted: 0,
            to_keeper: 0,
            ..Default::default()
        });
        assert!(s.ratings_trusted >= 1);
        assert!(s.to_keeper >= 1);
    }

    /// Вся причина, по которой здесь не среднее: одна пятёрка не должна
    /// обгонять пятьдесят по 4.8.
    #[test]
    fn one_five_never_beats_fifty_high_marks() {
        let lone = bayes_score(5.0, 1.0);
        let many = bayes_score(4.8 * 50.0, 50.0);
        assert!(many > lone, "одна пятёрка обогнала пятьдесят по 4.8: {many} < {lone}");
    }

    /// Накрутка мультиаккаунтами должна проигрывать живым игрокам — ради этого
    /// и заведён вес голоса.
    #[test]
    fn a_hundred_ones_from_strangers_weigh_less_than_twenty_fours_from_players() {
        let strangers = voter_weight(false, false, false) * 100.0;
        let players = voter_weight(true, true, true) * 20.0;
        assert!(bayes_score(4.0 * players, players) > bayes_score(strangers, strangers));
        // И вес не должен ни превысить единицу, ни упасть в ноль: нулевой вес —
        // это отнятый голос, а не ослабленный.
        assert_eq!(voter_weight(true, true, true), 1.0);
        assert!(voter_weight(false, false, false) > 0.0);
    }
}
