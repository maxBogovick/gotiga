//! Недельный разбор быстрых уходов: правила, без базы и без сети.
//!
//! Вход — свёртка `site_engagement_source_daily` за неделю и за четыре прежних, плюс
//! скорость и ошибки (они только объясняют найденное). Выход — список находок и текст
//! для Telegram. Нейросети здесь нет: находка — это арифметика над счётчиками, и по ней
//! можно проверить каждое число. Почему человек ушёл, разбор не знает; он называет
//! приметы рядом (`Hint`), и слова в тексте — «возможно», а не «причина».
//!
//! Числа порогов — не замер, а осторожный старт: сайт небольшой, и десять человек в
//! группе — это шум. Их правят здесь, в одном месте.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Меньше стольких визитов в группе (комната × источник × устройство) — не судим.
pub const MIN_CELL_VISITS: i64 = 30;
/// Для комнаты целиком порог выше: она сравнивается с собой же за прошлые недели.
pub const MIN_ROOM_VISITS: i64 = 50;
/// Сколько визитов должно быть в том, с чем сравниваем, иначе сравнение ничего не стоит.
pub const MIN_REFERENCE_VISITS: i64 = 60;
/// Доля быстрых уходов, ниже которой не говорим о проблеме, как бы ни выросла.
pub const MIN_RATE: f64 = 0.40;
/// Насколько доля должна быть выше той, с чем сравниваем (доля, не процентные пункты).
pub const MIN_GAP: f64 = 0.15;
/// Отличие в «сигмах» биномиального шума: два с лишним — уже не случай.
pub const MIN_Z: f64 = 2.0;
/// Находок в одном разборе: больше человек всё равно не прочтёт.
pub const MAX_FINDINGS: usize = 8;
/// LCP (мс) хуже порога «хорошо» Core Web Vitals.
pub const SLOW_LCP_MS: f64 = 2500.0;
/// Замеров скорости, меньше которых по скорости не судим.
pub const MIN_VITALS_SAMPLES: i64 = 10;
/// Ошибок на комнате за неделю, с которых они считаются приметой.
pub const MIN_ERRORS: i64 = 3;
/// Быстро ушедших, меньше которых «не прокрутили» ничего не говорит.
pub const MIN_QUICK_FOR_SCROLL: i64 = 15;
/// Доля не прокрутивших среди быстро ушедших, с которой это примета.
pub const NO_SCROLL_SHARE: f64 = 0.8;

/// Строка свёртки за один день, сложенная по неделе.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CellCount {
    pub path_group: String,
    pub origin: String,
    pub device_class: String,
    pub visits: i64,
    pub quick: i64,
    pub quick_unscrolled: i64,
}

/// Скорость загрузки комнаты на устройстве (LCP, 75-й процентиль, взвешенно по дням).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SlowLoad {
    pub path_group: String,
    pub device_class: String,
    pub samples: i64,
    pub lcp_p75: f64,
}

/// Ошибки на комнате за неделю.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrokenRoom {
    pub path_group: String,
    pub events: i64,
}

/// С чем сравнивали долю быстрых уходов.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Against {
    /// С той же группой за четыре прежние недели.
    Past,
    /// С остальными посетителями той же комнаты на этой неделе.
    Room,
}

/// Примета рядом с находкой. Не причина: то, что замерено и совпало по месту.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum Hint {
    /// Страница грузится медленно.
    SlowLoad {
        #[serde(rename = "lcpMs")]
        lcp_ms: i64,
    },
    /// На комнате были ошибки.
    Errors { events: i64 },
    /// Быстро ушедшие почти не прокручивали: первый экран не дал повода остаться.
    NoScroll { share: f64 },
    /// Пришли из приложения соцсети: во встроенном браузере уходят быстрее всегда.
    InAppBrowser,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub path_group: String,
    /// `None` — все источники.
    pub origin: Option<String>,
    /// `None` — все устройства.
    pub device_class: Option<String>,
    pub visits: i64,
    pub quick: i64,
    /// Доля быстрых уходов этой недели, 0..1.
    pub rate: f64,
    pub reference_rate: f64,
    pub against: Against,
    /// Насколько больше людей ушло быстро, чем дала бы доля сравнения.
    pub extra: f64,
    pub hints: Vec<Hint>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Digest {
    pub visits: i64,
    pub quick: i64,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Copy, Default)]
struct Totals {
    visits: i64,
    quick: i64,
    unscrolled: i64,
}

impl Totals {
    fn rate(&self) -> f64 {
        if self.visits == 0 {
            0.0
        } else {
            self.quick as f64 / self.visits as f64
        }
    }
}

type Key = (String, Option<String>, Option<String>);

/// Каждая строка идёт в четыре счёта: комната, комната × источник, комната × устройство,
/// комната × источник × устройство. Так проблема находится на том уровне, где она есть:
/// «Pinterest вообще» или «Pinterest на телефоне».
fn tally(rows: &[CellCount]) -> HashMap<Key, Totals> {
    let mut map: HashMap<Key, Totals> = HashMap::new();
    for r in rows {
        let keys = [
            (r.path_group.clone(), None, None),
            (r.path_group.clone(), Some(r.origin.clone()), None),
            (r.path_group.clone(), None, Some(r.device_class.clone())),
            (
                r.path_group.clone(),
                Some(r.origin.clone()),
                Some(r.device_class.clone()),
            ),
        ];
        for key in keys {
            let t = map.entry(key).or_default();
            t.visits += r.visits;
            t.quick += r.quick;
            t.unscrolled += r.quick_unscrolled;
        }
    }
    map
}

/// Выше ли доля `now` доли сравнения настолько, что это не шум. Возвращает «лишних»
/// быстро ушедших.
fn judge(now: &Totals, reference: f64) -> Option<f64> {
    let rate = now.rate();
    if rate < MIN_RATE || rate - reference < MIN_GAP {
        return None;
    }
    // Дисперсия снизу ограничена: при сравнении с долей около 0 или 1 знаменатель
    // иначе обнулил бы шум и любое отличие стало бы «значимым».
    let variance = (reference * (1.0 - reference)).max(0.01);
    let z = (rate - reference) / (variance / now.visits as f64).sqrt();
    if z < MIN_Z {
        return None;
    }
    Some((rate - reference) * now.visits as f64)
}

/// Источники, в которых страница открывается во встроенном браузере приложения.
fn is_in_app(origin: &str) -> bool {
    const MARKS: [&str; 8] = [
        "telegram",
        "t.me",
        "instagram",
        "facebook",
        "fb.",
        "pinterest",
        "tiktok",
        "vk.com",
    ];
    let o = origin.to_ascii_lowercase();
    MARKS.iter().any(|m| o.contains(m))
}

fn hints_for(
    f: &Finding,
    unscrolled: i64,
    slow: &[SlowLoad],
    broken: &[BrokenRoom],
) -> Vec<Hint> {
    let mut out = Vec::new();
    let worst = slow
        .iter()
        .filter(|s| s.path_group == f.path_group && s.samples >= MIN_VITALS_SAMPLES)
        .filter(|s| f.device_class.as_ref().is_none_or(|d| *d == s.device_class))
        .map(|s| s.lcp_p75)
        .fold(None, |acc: Option<f64>, v| Some(acc.map_or(v, |a| a.max(v))));
    if let Some(lcp) = worst
        && lcp > SLOW_LCP_MS
    {
        out.push(Hint::SlowLoad { lcp_ms: lcp.round() as i64 });
    }
    if let Some(b) = broken
        .iter()
        .find(|b| b.path_group == f.path_group && b.events >= MIN_ERRORS)
    {
        out.push(Hint::Errors { events: b.events });
    }
    if f.quick >= MIN_QUICK_FOR_SCROLL {
        let share = unscrolled as f64 / f.quick as f64;
        if share >= NO_SCROLL_SHARE {
            out.push(Hint::NoScroll { share });
        }
    }
    if f.origin.as_deref().is_some_and(is_in_app) {
        out.push(Hint::InAppBrowser);
    }
    out
}

/// А — шире Б: та же комната, и каждое названное А измерение совпадает с Б.
fn covers(a: &Finding, b: &Finding) -> bool {
    a.path_group == b.path_group
        && (a.origin.is_some() || a.device_class.is_some())
        && a.origin.as_ref().is_none_or(|o| Some(o) == b.origin.as_ref())
        && a.device_class
            .as_ref()
            .is_none_or(|d| Some(d) == b.device_class.as_ref())
        && (a.origin != b.origin || a.device_class != b.device_class)
}

/// Собрать разбор. `current` — неделя, `past` — четыре недели перед ней.
pub fn build(
    current: &[CellCount],
    past: &[CellCount],
    slow: &[SlowLoad],
    broken: &[BrokenRoom],
) -> Digest {
    let now = tally(current);
    let before = tally(past);
    let mut found: Vec<(Finding, i64)> = Vec::new();

    for (key, t) in &now {
        let is_room = key.1.is_none() && key.2.is_none();
        let min = if is_room { MIN_ROOM_VISITS } else { MIN_CELL_VISITS };
        if t.visits < min || t.rate() < MIN_RATE {
            continue;
        }
        let mut verdict: Option<(f64, f64, Against)> = None;

        if let Some(prev) = before.get(key)
            && prev.visits >= MIN_REFERENCE_VISITS
            && let Some(extra) = judge(t, prev.rate())
        {
            verdict = Some((extra, prev.rate(), Against::Past));
        }
        // Всегда плохая группа с прошлым не сравнится: сравниваем с остальной комнатой.
        if verdict.is_none()
            && !is_room
            && let Some(room) = now.get(&(key.0.clone(), None, None))
        {
            let rest = Totals {
                visits: room.visits - t.visits,
                quick: room.quick - t.quick,
                unscrolled: room.unscrolled - t.unscrolled,
            };
            if rest.visits >= MIN_REFERENCE_VISITS
                && let Some(extra) = judge(t, rest.rate())
            {
                verdict = Some((extra, rest.rate(), Against::Room));
            }
        }

        if let Some((extra, reference_rate, against)) = verdict {
            let finding = Finding {
                path_group: key.0.clone(),
                origin: key.1.clone(),
                device_class: key.2.clone(),
                visits: t.visits,
                quick: t.quick,
                rate: t.rate(),
                reference_rate,
                against,
                extra,
                hints: Vec::new(),
            };
            found.push((finding, t.unscrolled));
        }
    }

    // Те же самые люди, названные по-разному («Pinterest» и «телефон», когда с телефона
    // пришёл только Pinterest), — одна находка. Остаётся самая широкая, а между источником
    // и устройством — источник: он говорит о ссылке, которую можно поправить.
    found.sort_by_key(|(f, _)| {
        (
            f.origin.is_some() as u8 + f.device_class.is_some() as u8,
            f.origin.is_none(),
        )
    });
    let mut same: std::collections::HashSet<(String, i64, i64)> = std::collections::HashSet::new();
    found.retain(|(f, _)| same.insert((f.path_group.clone(), f.visits, f.quick)));

    // Узкая находка, уже названная более широкой («Pinterest на телефоне» внутри
    // «Pinterest»), второй строкой не нужна.
    let wide: Vec<Finding> = found.iter().map(|(f, _)| f.clone()).collect();
    found.retain(|(f, _)| !wide.iter().any(|w| covers(w, f)));

    found.sort_by(|a, b| {
        b.0.extra
            .partial_cmp(&a.0.extra)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.path_group.cmp(&b.0.path_group))
            .then_with(|| a.0.origin.cmp(&b.0.origin))
            .then_with(|| a.0.device_class.cmp(&b.0.device_class))
    });
    found.truncate(MAX_FINDINGS);

    let findings = found
        .into_iter()
        .map(|(mut f, unscrolled)| {
            f.hints = hints_for(&f, unscrolled, slow, broken);
            f
        })
        .collect();

    Digest {
        visits: current.iter().map(|r| r.visits).sum(),
        quick: current.iter().map(|r| r.quick).sum(),
        findings,
    }
}

/// Понедельник и воскресенье последней законченной недели.
pub fn last_full_week(today: NaiveDate) -> (NaiveDate, NaiveDate) {
    use chrono::{Datelike, Duration};
    let this_monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    (this_monday - Duration::days(7), this_monday - Duration::days(1))
}

// ── Слова для Telegram ───────────────────────────────────────────────────

fn room_name(key: &str) -> &str {
    match key {
        "home" => "Главная",
        "archive" => "Все работы",
        "work" => "Страница работы",
        "passport" => "Паспорт работы",
        "author" => "Автор",
        "workshop" => "Мастерская",
        "tale" => "Небылицы",
        "gazette" => "Вестник",
        "battles" => "Игра",
        "studio" => "Студия",
        "acquire" => "Как приобрести работу",
        "commission" => "Заказ работы",
        "other" => "Неизвестные адреса и 404",
        other => other,
    }
}

fn device_name(key: &str) -> &str {
    match key {
        "mobile" => "телефон",
        "desktop" => "компьютер",
        "tablet" => "планшет",
        _ => "устройство не определено",
    }
}

fn origin_name(key: &str) -> String {
    match key {
        "direct" => "прямой заход".into(),
        "internal" => "переход со страниц сайта".into(),
        "search" => "поиск".into(),
        other => other.to_string(),
    }
}

fn percent(x: f64) -> String {
    format!("{}%", (x * 100.0).round() as i64)
}

fn hint_text(h: &Hint) -> String {
    match h {
        Hint::SlowLoad { lcp_ms } => format!(
            "страница грузится медленно (главная картинка за {} с)",
            format!("{:.1}", *lcp_ms as f64 / 1000.0).replace('.', ",")
        ),
        Hint::Errors { events } => format!("на странице были ошибки ({events})"),
        Hint::NoScroll { share } => format!(
            "{} быстро ушедших не прокрутили страницу",
            percent(*share)
        ),
        Hint::InAppBrowser => {
            "переход из приложения соцсети: во встроенном браузере уходят быстрее".into()
        }
    }
}

fn month_name(m: u32) -> &'static str {
    [
        "января", "февраля", "марта", "апреля", "мая", "июня", "июля", "августа", "сентября",
        "октября", "ноября", "декабря",
    ][(m as usize - 1).min(11)]
}

/// Сообщение владельцу: простой текст без разметки, чтобы нечего было экранировать.
pub fn telegram_text(week_start: NaiveDate, week_end: NaiveDate, d: &Digest, admin_url: &str) -> String {
    use chrono::Datelike;
    let span = if week_start.month() == week_end.month() {
        format!(
            "{}–{} {}",
            week_start.day(),
            week_end.day(),
            month_name(week_end.month())
        )
    } else {
        format!(
            "{} {} – {} {}",
            week_start.day(),
            month_name(week_start.month()),
            week_end.day(),
            month_name(week_end.month())
        )
    };
    let mut out = format!("Разбор быстрых уходов, {span}\n");
    if d.visits == 0 {
        out.push_str("\nЗа неделю в свёртке нет визитов. Она копится с выкладки этого разбора.\n");
    } else {
        out.push_str(&format!(
            "Визитов: {}. Ушли быстрее 10 секунд: {}.\n",
            d.visits,
            percent(d.quick as f64 / d.visits as f64)
        ));
    }
    if d.findings.is_empty() {
        if d.visits > 0 {
            out.push_str(&format!(
                "\nНичего не выбивается. Группы меньше {MIN_CELL_VISITS} визитов не оцениваются, \
                 поэтому на малом числе посетителей разбор молчит.\n"
            ));
        }
    } else {
        out.push_str("\nЧто выбивается:\n");
        for (i, f) in d.findings.iter().enumerate() {
            let mut who = vec![room_name(&f.path_group).to_string()];
            if let Some(o) = &f.origin {
                who.push(origin_name(o));
            }
            if let Some(dev) = &f.device_class {
                who.push(device_name(dev).to_string());
            }
            let against = match f.against {
                Against::Past => "за прошлые четыре недели было",
                Against::Room => "у остальных на этой странице",
            };
            out.push_str(&format!(
                "{}. {} — быстро ушли {} из {} ({}); {} {}.\n",
                i + 1,
                who.join(" · "),
                f.quick,
                f.visits,
                percent(f.rate),
                against,
                percent(f.reference_rate),
            ));
            if !f.hints.is_empty() {
                let list: Vec<String> = f.hints.iter().map(hint_text).collect();
                out.push_str(&format!("   Возможно: {}.\n", list.join("; ")));
            }
        }
        out.push_str(
            "\nЭто совпадения, а не причины: разбор видит, где и с чем рядом уходят, \
             но не знает, почему.\n",
        );
    }
    out.push_str(&format!("\nПодробности: {admin_url}"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pg: &str, origin: &str, dev: &str, visits: i64, quick: i64) -> CellCount {
        CellCount {
            path_group: pg.into(),
            origin: origin.into(),
            device_class: dev.into(),
            visits,
            quick,
            quick_unscrolled: quick,
        }
    }

    #[test]
    fn small_groups_are_not_judged() {
        let now = [row("tale", "pinterest", "mobile", 20, 19), row("tale", "direct", "desktop", 200, 40)];
        let d = build(&now, &[], &[], &[]);
        assert!(d.findings.iter().all(|f| f.origin.as_deref() != Some("pinterest")));
    }

    #[test]
    fn a_bad_source_stands_out_from_the_rest_of_the_room() {
        let now = [
            row("tale", "pinterest", "mobile", 60, 48),
            row("tale", "direct", "desktop", 200, 60),
        ];
        let d = build(&now, &[], &[], &[]);
        let f = d.findings.first().expect("a finding");
        assert_eq!(f.origin.as_deref(), Some("pinterest"));
        assert_eq!(f.against, Against::Room);
        assert!(f.rate > 0.75 && f.reference_rate < 0.35);
    }

    #[test]
    fn the_narrow_finding_is_dropped_under_the_wide_one() {
        // Весь Pinterest плох, и телефон внутри него — тот же самый факт.
        let now = [
            row("tale", "pinterest", "mobile", 60, 48),
            row("tale", "direct", "desktop", 200, 60),
        ];
        let d = build(&now, &[], &[], &[]);
        let names: Vec<_> = d
            .findings
            .iter()
            .map(|f| (f.origin.clone(), f.device_class.clone()))
            .collect();
        assert!(names.contains(&(Some("pinterest".into()), None)));
        assert!(!names.contains(&(Some("pinterest".into()), Some("mobile".into()))));
    }

    #[test]
    fn growth_against_the_past_is_found_even_when_the_room_is_uniform() {
        let now = [row("home", "direct", "desktop", 120, 84)];
        let past = [row("home", "direct", "desktop", 400, 120)];
        let d = build(&now, &past, &[], &[]);
        let f = d.findings.iter().find(|f| f.origin.is_none() && f.device_class.is_none()).unwrap();
        assert_eq!(f.against, Against::Past);
        assert!((f.reference_rate - 0.3).abs() < 1e-9);
    }

    #[test]
    fn the_usual_rate_is_not_a_finding() {
        let now = [row("home", "direct", "desktop", 120, 60)];
        let past = [row("home", "direct", "desktop", 400, 190)];
        assert!(build(&now, &past, &[], &[]).findings.is_empty());
    }

    #[test]
    fn a_small_rise_inside_the_noise_is_not_a_finding() {
        // 53% против 45% на 60 визитах: выше порога по разнице? нет, 8 пунктов < 15.
        let now = [row("home", "direct", "desktop", 60, 32)];
        let past = [row("home", "direct", "desktop", 400, 180)];
        assert!(build(&now, &past, &[], &[]).findings.is_empty());
    }

    #[test]
    fn hints_name_what_stands_next_to_the_finding() {
        let now = [
            row("tale", "telegram", "mobile", 60, 50),
            row("tale", "direct", "desktop", 200, 50),
        ];
        let slow = [SlowLoad { path_group: "tale".into(), device_class: "mobile".into(), samples: 14, lcp_p75: 3900.0 }];
        let broken = [BrokenRoom { path_group: "tale".into(), events: 5 }];
        let d = build(&now, &[], &slow, &broken);
        let f = d.findings.iter().find(|f| f.origin.as_deref() == Some("telegram")).unwrap();
        assert!(f.hints.contains(&Hint::SlowLoad { lcp_ms: 3900 }));
        assert!(f.hints.contains(&Hint::Errors { events: 5 }));
        assert!(f.hints.contains(&Hint::InAppBrowser));
    }

    #[test]
    fn slow_load_on_another_device_is_not_a_hint_for_this_one() {
        // Находка про компьютер; медленно только на телефоне.
        let now = [
            row("tale", "direct", "desktop", 60, 50),
            row("tale", "direct", "mobile", 200, 50),
        ];
        let slow = [SlowLoad { path_group: "tale".into(), device_class: "mobile".into(), samples: 40, lcp_p75: 5000.0 }];
        let d = build(&now, &[], &slow, &[]);
        let f = d
            .findings
            .iter()
            .find(|f| f.device_class.as_deref() == Some("desktop"))
            .expect("the desktop finding");
        assert!(!f.hints.iter().any(|h| matches!(h, Hint::SlowLoad { .. })));
    }

    #[test]
    fn at_most_eight_findings_and_biggest_first() {
        let mut now = vec![row("home", "direct", "desktop", 2000, 600)];
        for i in 0..12 {
            now.push(row("tale", &format!("site{i}.example"), "mobile", 40 + i, 38 + i));
        }
        let d = build(&now, &[], &[], &[]);
        assert!(d.findings.len() <= MAX_FINDINGS);
        assert!(d.findings.windows(2).all(|w| w[0].extra >= w[1].extra));
    }

    #[test]
    fn the_week_is_the_last_full_monday_to_sunday() {
        // 2026-10-25 — воскресенье; законченная неделя — 12–18 октября.
        let (s, e) = last_full_week(NaiveDate::from_ymd_opt(2026, 10, 25).unwrap());
        assert_eq!((s.to_string(), e.to_string()), ("2026-10-12".into(), "2026-10-18".into()));
        // В понедельник законченной стала прошедшая неделя.
        let (s, e) = last_full_week(NaiveDate::from_ymd_opt(2026, 10, 26).unwrap());
        assert_eq!((s.to_string(), e.to_string()), ("2026-10-19".into(), "2026-10-25".into()));
    }

    #[test]
    fn text_says_the_truth_when_there_is_nothing_to_say() {
        let s = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let e = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
        let empty = telegram_text(s, e, &Digest::default(), "https://x/admin");
        assert!(empty.contains("19–25 октября"));
        assert!(empty.contains("нет визитов"));
        let quiet = telegram_text(s, e, &Digest { visits: 300, quick: 90, findings: vec![] }, "https://x/admin");
        assert!(quiet.contains("Ничего не выбивается"));
        assert!(quiet.contains("30%"));
    }

    #[test]
    fn text_names_the_finding_in_plain_words() {
        let s = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let e = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
        let now = [
            row("tale", "pinterest", "mobile", 60, 48),
            row("tale", "direct", "desktop", 200, 60),
        ];
        let d = build(&now, &[], &[], &[]);
        let t = telegram_text(s, e, &d, "https://x/admin");
        assert!(t.contains("Небылицы · pinterest"));
        assert!(t.contains("48 из 60"));
        assert!(t.contains("совпадения, а не причины"));
    }
}
