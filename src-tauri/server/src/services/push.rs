//! Уведомления в браузере: подписки и рассылка о новом на сайте.
//!
//! Что нового, решает та же выборка, что у Telegram-канала (`channel_on_display`), а не
//! события «опубликовано»: запланированный лист выходит сам, работа — когда с неё сняли
//! «в работе», и ни у одного из путей нет своей ручки. Проход раз в пять минут сверяет то,
//! что на людях сейчас, с журналом `push_posts`.
//!
//! Шифрование и подпись — `crate::push`; здесь база, выбор получателей и тексты.

use super::*;
use crate::db::PushSubscriber;
use crate::lang::Lang;
use crate::push::{self, Vapid};

/// Ключ настройки: пара ключей сервера (PKCS#8, base64). Заводится сама при первом обращении.
const VAPID_KEY: &str = "push_vapid";
/// Ключ настройки: «когда рассылку подключили». Всё, что уже стоит на сайте, к этому дню
/// отмечается ушедшим: иначе первая же подписка получила бы архив.
const PUSH_SINCE_KEY: &str = "push_since";
/// Выдержка: работу показывают и ещё правят, а уведомление не поправить.
const PUSH_GRACE_SECS: i64 = 600;
/// Вещей за проход: три уведомления подряд — это уже поток.
const PUSH_PER_TICK: usize = 3;
/// Сколько ждущих вещей смотрим за проход: больше, чем отправляем (см. `announce_push_from`).
const PUSH_DUE_WINDOW: i64 = 200;
/// Сколько проходов пытаемся доставить вещь, до которой не дотянулась связь.
const PUSH_MAX_ATTEMPTS: i32 = 4;
/// Вещь, замеченная давно и не ушедшая (сервер лежал), уже не новость.
const PUSH_STALE_DAYS: i64 = 3;
/// Новых подписок в час с одного адреса.
const PUSH_NEW_PER_HOUR: usize = 30;
/// Потолок подписок на сайте.
const PUSH_MAX_SUBSCRIPTIONS: i64 = 100_000;
/// Запросов к узлам рассылки одновременно.
const PUSH_PARALLEL: usize = 16;
/// Длина вводки в уведомлении: дальше браузеры обрезают сами.
const PUSH_LEAD_CHARS: usize = 140;

/// Темы, о которых просят: ровно те, что в `CHECK` таблицы.
const TOPICS: [&str; 3] = ["works", "tales", "gazette"];

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Delivered,
    /// Узел рассылки назвал адрес несуществующим (404/410): подписку стираем.
    Gone,
    /// Отказ, который говорит об этой подписке или о нашей подписи: считается.
    Refused,
    /// Сеть или узел рассылки не ответили: подписчик ни при чём.
    Unreached,
}

fn topic_of(group: crate::channel::Group) -> Option<&'static str> {
    use crate::channel::Group;
    match group {
        Group::Works => Some("works"),
        Group::Tales => Some("tales"),
        Group::Other => Some("gazette"),
        // Игра в уведомлениях не обещана: о ней канал и пост о релизе.
        Group::Game => None,
    }
}

fn clip(text: &str, max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max).collect();
    // Резать по слову: если на границе слово не оборвано, оно остаётся целым.
    let at_word_end = text.chars().nth(max).is_some_and(char::is_whitespace);
    let cut = if at_word_end {
        cut.as_str()
    } else {
        cut.rsplit_once(char::is_whitespace).map_or(cut.as_str(), |(head, _)| head)
    };
    format!("{}…", cut.trim_end_matches(|c: char| c.is_ascii_punctuation()))
}

impl AppService {
    /// Пара ключей сервера. Её нет в окружении намеренно: поставить второй раз руками
    /// значило бы однажды разойтись с тем, что запомнили браузеры подписчиков, и тогда
    /// уведомления молча перестали бы приходить. Заводится при первом обращении и лежит
    /// в базе; две копии сервера, заводящие одновременно, получают один ключ.
    async fn push_vapid(&self) -> Result<Arc<Vapid>> {
        let cell = &self.push_vapid;
        let vapid = cell
            .get_or_try_init(|| async {
                let engine = base64::engine::general_purpose::STANDARD;
                if self.repo.get_setting(VAPID_KEY).await?.is_none() {
                    let pkcs8 = Vapid::generate_pkcs8()
                        .map_err(|e| AppError::Internal(format!("push keys: {e}")))?;
                    self.repo.setting_insert_if_absent(VAPID_KEY, &engine.encode(pkcs8)).await?;
                }
                let stored = self
                    .repo
                    .get_setting(VAPID_KEY)
                    .await?
                    .ok_or_else(|| AppError::Internal("push keys were not saved".into()))?;
                let pkcs8 = engine
                    .decode(stored.trim())
                    .map_err(|e| AppError::Internal(format!("push keys: {e}")))?;
                let vapid = Vapid::from_pkcs8(&pkcs8)
                    .map_err(|e| AppError::Internal(format!("push keys: {e}")))?;
                Ok::<_, AppError>(Arc::new(vapid))
            })
            .await?;
        Ok(vapid.clone())
    }

    /// Открытый ключ сервера: его браузер просит при подписке.
    pub async fn push_public_key(&self) -> Result<String> {
        Ok(self.push_vapid().await?.public_key())
    }

    /// Записать или обновить подписку устройства. Адрес проверяется по списку узлов
    /// рассылки: сервер потом сам идёт по нему запросом.
    ///
    /// Пределы «в час с адреса» и потолок числа подписок касаются только **новых** подписок.
    /// Обновить уже известную (сменить темы, язык, подтвердить запись) может лишь тот, кто
    /// знает её адрес — это секрет устройства, — и за общим адресом (школа, оператор связи)
    /// такие обновления не должны съедать лимит, оставляя других без подписки.
    pub async fn push_subscribe(&self, req: PushSubscribeRequest, ip: &str) -> Result<()> {
        let endpoint = req.endpoint.trim();
        if push::endpoint_audience(endpoint).is_none() {
            return Err(AppError::BadRequest("This push endpoint is not supported.".into()));
        }
        // Ключи проверяются тем же разбором, что и при отправке: подписка, которую
        // нечем зашифровать, только копила бы неудачи.
        let (public, secret) = push::decode_keys(&req.keys.p256dh, &req.keys.auth)
            .map_err(|_| AppError::BadRequest("The push keys are not valid.".into()))?;
        // Длина и первый байт ключа ещё не значат, что он лежит на кривой: пробное шифрование
        // отсеивает выдуманные ключи сейчас, а не после пяти проходов рассылки.
        push::encrypt(&public, &secret, b"x")
            .map_err(|_| AppError::BadRequest("The push keys are not valid.".into()))?;

        if self.repo.push_find(endpoint).await?.is_none() {
            self.check_rate_limit("push_subscribe", ip, PUSH_NEW_PER_HOUR, 3600).await?;
            // Подписка без входа: потолок общего числа не даёт копить мёртвые строки без конца.
            if self.repo.push_count().await? >= PUSH_MAX_SUBSCRIPTIONS {
                return Err(AppError::BadRequest("Too many subscriptions.".into()));
            }
        }

        // Браузер сам выдал новую подписку взамен старой (`pushsubscriptionchange`): Service
        // Worker не знает ни тем, ни языка читателя, поэтому они переезжают из прежней записи.
        let old = req.old_endpoint.as_deref().map(str::trim).filter(|o| !o.is_empty());
        let prior = match old {
            Some(o) => self.repo.push_find(o).await?,
            None => None,
        };
        let Some((lang, topics)) = resolve_choice(&req.topics, &req.lang, prior) else {
            return Err(AppError::BadRequest("Choose at least one topic.".into()));
        };
        // Ключи сервера заводятся здесь же: подписка без них не могла бы получить ни одного
        // сообщения, а без вызова `push_public_key` их никто бы не завёл.
        self.push_vapid().await?;
        self.repo
            .push_upsert(endpoint, &req.keys.p256dh, &req.keys.auth, lang.code(), &topics, old)
            .await
    }

    pub async fn push_unsubscribe(&self, endpoint: &str, ip: &str) -> Result<()> {
        // Запись стирает только тот, кто знает адрес; предел — от потока пустых запросов.
        self.check_rate_limit("push_unsubscribe", ip, 120, 3600).await?;
        self.repo.push_remove(endpoint.trim()).await?;
        Ok(())
    }

    /// Проход рассылок за один тик: Telegram-каналы и уведомления в браузере. Что сейчас на
    /// людях, читается один раз и на оба прохода: это самое тяжёлое чтение сайта (все работы
    /// и пятьсот листов вестника), и два чтения подряд платили бы за него дважды каждые пять
    /// минут. Сбой одного прохода другому не мешает. Возвращает число постов в каналах и
    /// число уведомлений.
    pub async fn announce_news(&self) -> Result<(usize, usize)> {
        // Не прочиталось — рассылка уведомлений в этот тик пропускается, а Telegram-каналы идут
        // как шли: у них с уведомлениями общее только чтение «что на людях». Пустым список
        // подписчиков заменять нельзя: проход счёл бы, что получателей нет, и отметил бы вещи
        // ушедшими.
        let subs = match self.repo.push_active().await {
            Ok(subs) => Some(subs),
            Err(e) => {
                tracing::warn!("Push subscribers could not be read, the push pass is skipped: {e}");
                None
            }
        };
        let no_subs: Vec<PushSubscriber> = Vec::new();
        let mut langs: Vec<Lang> = self.channels().into_iter().map(|(l, _)| l).collect();
        for l in Lang::ALL {
            if subs.as_ref().unwrap_or(&no_subs).iter().any(|s| s.lang == l.code()) {
                langs.push(l);
            }
        }
        // Английский нужен всегда: по нему читается список вещей на людях (ключи у всех
        // языков одни и те же).
        langs.push(Lang::En);
        langs.sort_by_key(|l| l.code());
        langs.dedup();
        let display = self.channel_on_display(&langs).await?;
        let posted = match self.announce_channels(&display).await {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!("Telegram channel round failed: {e}");
                0
            }
        };
        let pushed = match &subs {
            Some(subs) => match self.announce_push_from(&display, subs).await {
                Ok(n) => n,
                Err(e) => {
                    tracing::warn!("Push round failed: {e}");
                    0
                }
            },
            None => 0,
        };
        Ok((posted, pushed))
    }

    /// Разослать то, что появилось на сайте: журнал `push_posts`, выдержка, получатели по
    /// темам и языкам.
    async fn announce_push_from(
        &self,
        display: &HashMap<Lang, HashMap<(String, Uuid), ChannelPost>>,
        subs: &[PushSubscriber],
    ) -> Result<usize> {
        let Some(on_display) = display.get(&Lang::En) else { return Ok(0) };

        let ids_of = |kind: &str| -> Vec<Uuid> {
            on_display
                .iter()
                .filter(|((k, _), post)| k == kind && topic_of(post.group).is_some())
                .map(|((_, id), _)| *id)
                .collect()
        };
        let kinds = ["work", "leaf"];

        // Первый проход после подключения: всё, что уже на сайте, считается ушедшим.
        if self.repo.get_setting(PUSH_SINCE_KEY).await?.is_none() {
            let mut n = 0;
            for kind in kinds {
                n += self.repo.push_note_seen(kind, &ids_of(kind), true).await?;
            }
            self.repo.upsert_setting(PUSH_SINCE_KEY, &Utc::now().to_rfc3339()).await?;
            tracing::info!(
                "Push notifications connected: {n} works and leaves already on the site are left \
                 unannounced; everything that appears from now on goes out"
            );
            return Ok(0);
        }
        for kind in kinds {
            self.repo.push_note_seen(kind, &ids_of(kind), false).await?;
        }

        // Берём с запасом, а не ровно «вещей за проход»: снятая с сайта вещь остаётся ждать,
        // и три таких, самые старые, занимали бы все места, не пуская новые.
        let due = self.repo.push_due(PUSH_GRACE_SECS, PUSH_DUE_WINDOW).await?;
        if due.is_empty() {
            return Ok(0);
        }
        let local = self.config.public_url_is_local();
        let vapid = self.push_vapid().await?;
        // Без переходов: узел рассылки отвечает сам, а перенаправление ушло бы на адрес,
        // которого нет в списке доверенных.
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        let base = self.config.public_url.trim_end_matches('/').to_string();
        let mut sent = 0;
        let mut worked = 0;

        for (kind, id, first_seen) in due {
            if worked >= PUSH_PER_TICK {
                break;
            }
            let Some(en_post) = on_display.get(&(kind.clone(), id)) else {
                continue; // снова снята с сайта: вернут — выйдет тогда
            };
            let stale = Utc::now() - first_seen > chrono::Duration::days(PUSH_STALE_DAYS);
            let Some(topic) = topic_of(en_post.group) else {
                self.repo.push_mark_sent(&kind, id).await?;
                continue;
            };
            let audience: Vec<&PushSubscriber> =
                subs.iter().filter(|s| s.topics.iter().any(|t| t == topic)).collect();
            if stale || audience.is_empty() {
                self.repo.push_mark_sent(&kind, id).await?;
                continue;
            }
            worked += 1;

            // Текст — на языке каждого подписчика; собирается один раз на язык.
            let mut payloads: HashMap<String, Arc<Vec<u8>>> = HashMap::new();
            for sub in &audience {
                if payloads.contains_key(&sub.lang) {
                    continue;
                }
                let lang = Lang::parse(&sub.lang).unwrap_or(Lang::En);
                let post = display
                    .get(&lang)
                    .and_then(|m| m.get(&(kind.clone(), id)))
                    .unwrap_or(en_post);
                let path = post.url.strip_prefix(&base).unwrap_or(&post.url).replace("src=telegram", "src=push");
                let mut body = post.title.clone();
                if let Some(lead) = post.lead.as_deref().filter(|l| !l.trim().is_empty()) {
                    body = format!("{body}\n{}", clip(lead, PUSH_LEAD_CHARS));
                }
                let json = serde_json::json!({
                    "title": post.label,
                    "body": body,
                    "url": path,
                    "tag": format!("{kind}:{id}"),
                    "image": post.image,
                    "lang": lang.code(),
                });
                payloads.insert(sub.lang.clone(), Arc::new(json.to_string().into_bytes()));
            }

            if local {
                // На своей машине в чужие браузеры не шлём: база стенда своя, а узлы рассылки
                // настоящие. Закон `say_or_send` (§ 16.4): дом называет, что отправил бы.
                tracing::info!(
                    "Push (local, not sent): {kind} {id} would go to {} subscribers on topic {topic}",
                    audience.len()
                );
                self.repo.push_mark_sent(&kind, id).await?;
                continue;
            }

            let subject = base.clone();
            let gate = Arc::new(tokio::sync::Semaphore::new(PUSH_PARALLEL));
            let mut set = tokio::task::JoinSet::new();
            for sub in audience {
                let payload = payloads[&sub.lang].clone();
                let (client, vapid, subject, gate, sub) =
                    (client.clone(), vapid.clone(), subject.clone(), gate.clone(), sub.clone());
                set.spawn(async move {
                    let _slot = gate.acquire().await;
                    let outcome = deliver(&client, &vapid, &subject, &sub, &payload).await;
                    (sub.id, outcome)
                });
            }
            let (mut ok, mut gone, mut refused, mut unreached) =
                (Vec::new(), Vec::new(), Vec::new(), 0usize);
            while let Some(done) = set.join_next().await {
                match done {
                    Ok((sid, Outcome::Delivered)) => ok.push(sid),
                    Ok((sid, Outcome::Gone)) => gone.push(sid),
                    Ok((sid, Outcome::Refused)) => refused.push(sid),
                    Ok((_, Outcome::Unreached)) | Err(_) => unreached += 1,
                }
            }
            self.repo.push_record(&ok, &gone, &refused).await?;
            sent += ok.len();
            tracing::info!(
                "Push {kind} {id}: {} delivered, {} gone, {} refused, {unreached} not reached",
                ok.len(),
                gone.len(),
                refused.len()
            );
            // Никому не дошло, и причина — связь (сеть сервера, узлы рассылки заняты): вещь
            // остаётся ждать следующего прохода, но не бесконечно. Иначе короткий сбой связи
            // в минуту выхода байки лишал бы её уведомления навсегда.
            if unreached > 0 && ok.is_empty() {
                let attempts = self.repo.push_attempt(&kind, id).await?;
                if keep_waiting(ok.len(), unreached, attempts) {
                    continue;
                }
            }
            self.repo.push_mark_sent(&kind, id).await?;
        }
        Ok(sent)
    }
}

/// Одно уведомление одному подписчику.
async fn deliver(
    client: &reqwest::Client,
    vapid: &Vapid,
    subject: &str,
    sub: &PushSubscriber,
    payload: &[u8],
) -> Outcome {
    let Some((audience, _host)) = push::endpoint_audience(&sub.endpoint) else {
        return Outcome::Gone;
    };
    let Ok((public, secret)) = push::decode_keys(&sub.p256dh, &sub.auth) else {
        return Outcome::Gone;
    };
    let body = match push::encrypt(&public, &secret, payload) {
        Ok(body) => body,
        // Ключ подписки не лежит на кривой: зашифровать нечем, и так будет всегда.
        Err(push::Error::BadKeys) => return Outcome::Gone,
        Err(_) => return Outcome::Refused,
    };
    let now = Utc::now().timestamp().max(0) as u64;
    let Ok(authorization) = vapid.authorization(&audience, subject, now) else {
        return Outcome::Refused;
    };
    let response = client
        .post(&sub.endpoint)
        .header("Authorization", authorization)
        .header("Content-Encoding", "aes128gcm")
        .header("Content-Type", "application/octet-stream")
        // Сутки: уведомление о новой байке не теряет смысла за ночь, но неделю спустя его
        // уже не показывают.
        .header("TTL", "86400")
        .header("Urgency", "normal")
        .body(body)
        .send()
        .await;
    match response {
        Ok(r) => {
            let outcome = classify_status(r.status().as_u16());
            if outcome == Outcome::Refused {
                tracing::warn!(
                    "Push refused by {}: {}",
                    sub.endpoint.split('/').nth(2).unwrap_or("?"),
                    r.status()
                );
            }
            outcome
        }
        Err(_) => Outcome::Unreached,
    }
}

/// Что значит ответ узла рассылки для подписки.
fn classify_status(status: u16) -> Outcome {
    match status {
        200..=299 => Outcome::Delivered,
        404 | 410 => Outcome::Gone,
        400 | 401 | 403 | 413 => Outcome::Refused,
        // 429 и 5xx: узел рассылки занят, подписчик ни при чём.
        _ => Outcome::Unreached,
    }
}

/// Остаётся ли вещь ждать следующего прохода: никому не дошло, а причина — связь, и попыток
/// ещё не исчерпано. Отказы и мёртвые адреса повода повторять не дают.
fn keep_waiting(delivered: usize, unreached: usize, attempts: i32) -> bool {
    delivered == 0 && unreached > 0 && attempts < PUSH_MAX_ATTEMPTS
}

/// Темы и язык, которые запишутся: из запроса, а если тем в нём нет (браузер сам сменил
/// подписку, и Service Worker их не знает) — из прежней записи. `None` — тем нет нигде.
fn resolve_choice(
    asked_topics: &[String],
    asked_lang: &str,
    prior: Option<(String, Vec<String>)>,
) -> Option<(Lang, Vec<String>)> {
    let known = |from: &[String]| -> Vec<String> {
        TOPICS
            .iter()
            .filter(|t| from.iter().any(|x| x == *t))
            .map(|t| t.to_string())
            .collect()
    };
    let mut topics = known(asked_topics);
    if topics.is_empty() {
        topics = known(prior.as_ref().map(|(_, t)| t.as_slice()).unwrap_or(&[]));
    }
    if topics.is_empty() {
        return None;
    }
    let lang = if asked_lang.trim().is_empty() {
        prior.as_ref().and_then(|(l, _)| Lang::parse(l))
    } else {
        Lang::parse(asked_lang)
    };
    Some((lang.unwrap_or(Lang::En), topics))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lead_is_cut_on_a_word() {
        assert_eq!(clip("short", 20), "short");
        let cut = clip("one two three four five six", 13);
        assert_eq!(cut, "one two three…");
        assert!(clip("  padded  ", 20) == "padded");
    }

    #[test]
    fn every_status_has_one_meaning() {
        assert_eq!(classify_status(201), Outcome::Delivered);
        assert_eq!(classify_status(204), Outcome::Delivered);
        // Адрес стёрт у узла рассылки: подписку стираем сразу.
        assert_eq!(classify_status(404), Outcome::Gone);
        assert_eq!(classify_status(410), Outcome::Gone);
        // Отказ об этой подписке или о нашей подписи: считается.
        for refused in [400, 401, 403, 413] {
            assert_eq!(classify_status(refused), Outcome::Refused, "{refused}");
        }
        // Узел занят или лежит: подписчик ни при чём.
        for busy in [429, 500, 502, 503, 301] {
            assert_eq!(classify_status(busy), Outcome::Unreached, "{busy}");
        }
    }

    #[test]
    fn only_a_lost_wire_keeps_an_item_waiting() {
        // Никому не дошло из-за связи — ждём, пока не исчерпаны попытки.
        assert!(keep_waiting(0, 3, 1));
        assert!(keep_waiting(0, 1, PUSH_MAX_ATTEMPTS - 1));
        assert!(!keep_waiting(0, 3, PUSH_MAX_ATTEMPTS), "попытки кончились");
        // Дошло хоть до одного — вещь ушла.
        assert!(!keep_waiting(1, 5, 1));
        // Одни отказы и мёртвые адреса: повторять нечего.
        assert!(!keep_waiting(0, 0, 1));
    }

    fn topics(of: &[&str]) -> Vec<String> {
        of.iter().map(|t| t.to_string()).collect()
    }

    #[test]
    fn choice_comes_from_the_request_or_the_previous_record() {
        // Из запроса; чужая тема отбрасывается, порядок — как в таблице.
        let (lang, got) = resolve_choice(&topics(&["gazette", "games", "works"]), "ru", None).unwrap();
        assert_eq!(lang, Lang::Ru);
        assert_eq!(got, topics(&["works", "gazette"]));
        // Смена подписки: запрос пуст, берётся прежняя запись.
        let prior = Some(("de".to_string(), topics(&["tales"])));
        let (lang, got) = resolve_choice(&[], "", prior.clone()).unwrap();
        assert_eq!((lang, got), (Lang::De, topics(&["tales"])));
        // Своё слово в запросе сильнее прежней записи.
        let (lang, got) = resolve_choice(&topics(&["works"]), "it", prior.clone()).unwrap();
        assert_eq!((lang, got), (Lang::It, topics(&["works"])));
        // Нигде нет тем — принимать нечего.
        assert!(resolve_choice(&[], "en", None).is_none());
        assert!(resolve_choice(&topics(&["games"]), "en", prior.map(|(l, _)| (l, vec![]))).is_none());
        // Непонятный язык — английский.
        assert_eq!(resolve_choice(&topics(&["works"]), "xx", None).unwrap().0, Lang::En);
    }

    #[test]
    fn game_things_are_not_announced() {
        use crate::channel::Group;
        assert_eq!(topic_of(Group::Works), Some("works"));
        assert_eq!(topic_of(Group::Tales), Some("tales"));
        assert_eq!(topic_of(Group::Other), Some("gazette"));
        assert_eq!(topic_of(Group::Game), None);
    }
}
