//! Вход через Google и Apple: правила дома поверх протокола (`crate::oauth`).
//!
//! Дорога одинакова для обоих провайдеров и устроена в три хода:
//!
//! 1. **`oauth_start`** — страница просит ссылку. Сервер заводит строку
//!    `oauth_logins` и отдаёт ссылку к провайдеру плюс *привязку* — секрет, который
//!    запоминает только эта вкладка.
//! 2. **`oauth_callback`** — провайдер вернул человека на сервер с одноразовым
//!    кодом. Сервер ничего не обменивает и никого не впускает: он смотрит, что
//!    попытка жива, и возвращает браузер на сайт, кладя код в `#`-часть адреса
//!    (до сервера она не доходит, в журналы прокси и в `Referer` не попадает).
//! 3. **`oauth_redeem`** — вкладка предъявляет привязку **вместе с кодом**.
//!    Только здесь код обменивается на личность, решается, кого впустить, и
//!    заводится сессия.
//!
//! Код и привязка должны встретиться в одном запросе, и это весь смысл порядка.
//! Привязку знает вкладка, просившая вход; код видит браузер, вернувшийся от
//! провайдера. У честного человека это одна и та же вкладка. Злоумышленник,
//! который сам получил ссылку и привязку и прислал ссылку жертве, держит
//! привязку, но кода не увидит; жертва видит код, но привязки у неё нет. Пока
//! обмен делал callback, а привязка проверялась только при выдаче, такая ссылка
//! отдавала злоумышленнику сессию жертвы (а при привязке из профиля — вешала
//! Google жертвы на аккаунт злоумышленника).
//!
//! Сессии в базе до выдачи нет вовсе: она заводится в момент ответа.

use super::*;
use crate::oauth::{self, AppleUser, Identity, Provider};

/// Потолок на запросы к провайдеру с одного адреса. Старт — это строка в базе
/// и ссылка; подряд их просят разве что для перебора.
const START_PER_HOUR: usize = 40;
const REDEEM_PER_HOUR: usize = 120;

/// Сколько знаков кода и имени Apple callback согласен нести дальше. Настоящие
/// вдвое-втрое короче; потолок нужен, чтобы адрес возврата не рос от чужой строки.
const MAX_CARRIED: usize = 2048;

/// Почему дом не впустил. Слово для страницы называет вариант, а не разбор
/// текста ошибки: прежде возраст узнавали по `BadRequest("age")`, а любой
/// `Unauthorized` читался как блокировка.
enum Turned {
    /// Имени ещё нет, а возраст не подтверждён.
    Age,
    Blocked,
    Conflict,
    Other(AppError),
}

impl From<AppError> for Turned {
    fn from(e: AppError) -> Self {
        match e {
            AppError::Conflict(_) => Turned::Conflict,
            e => Turned::Other(e),
        }
    }
}

impl Turned {
    fn word(&self) -> &'static str {
        match self {
            Turned::Age => "age",
            Turned::Blocked => "blocked",
            Turned::Conflict => "conflict",
            Turned::Other(_) => "failed",
        }
    }
}

impl AppService {
    /// Какие провайдеры включены. Страница не рисует кнопку, которой нет.
    pub fn oauth_config(&self) -> OauthConfig {
        OauthConfig {
            google: self.oauth.enabled(Provider::Google),
            apple: self.oauth.enabled(Provider::Apple),
        }
    }

    pub async fn oauth_check_start_limit(&self, ip: &str) -> Result<()> {
        self.check_rate_limit("oauth_start", ip, START_PER_HOUR, 3600).await
    }

    pub async fn oauth_check_redeem_limit(&self, ip: &str) -> Result<()> {
        self.check_rate_limit("oauth_redeem", ip, REDEEM_PER_HOUR, 3600).await
    }

    /// Куда возвращать браузер: туда, откуда он пришёл, но только если это
    /// свой адрес. Иначе `returnOrigin` стал бы открытым перенаправлением —
    /// ссылкой с нашего домена, ведущей куда угодно.
    fn oauth_return_origin(&self, asked: Option<&str>) -> String {
        let site = self.config.public_url.trim_end_matches('/').to_string();
        let Some(asked) = asked.map(|a| a.trim().trim_end_matches('/')) else {
            return site;
        };
        let mine = std::iter::once(site.as_str())
            .chain(self.config.cors_allowed_origins.iter().map(|o| o.trim_end_matches('/')))
            .any(|o| o.eq_ignore_ascii_case(asked));
        if mine { asked.to_string() } else { site }
    }

    /// Ход первый: ссылка к провайдеру и привязка для вкладки.
    pub async fn oauth_start(
        &self,
        p: Provider,
        bind_user: Option<Uuid>,
        return_origin: Option<String>,
        age_confirmed: bool,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<OauthStartResponse> {
        if !self.oauth.enabled(p) {
            return Err(AppError::BadRequest(format!("{} sign-in is not set up", p.as_str())));
        }
        let state = oauth::random_token(32);
        let nonce = oauth::random_token(24);
        let binder = oauth::random_token(32);
        let (verifier, challenge) = oauth::pkce_pair();
        let origin = self.oauth_return_origin(return_origin.as_deref());
        let ctx = self.client_context(ip, user_agent.clone());

        let row = self
            .repo
            .create_oauth_login(
                &state,
                p.as_str(),
                &nonce,
                &verifier,
                // В базу идёт отпечаток привязки, а не она сама: чтение таблицы
                // не должно давать права забрать чужой вход.
                &fingerprint(&binder),
                &origin,
                age_confirmed,
                bind_user,
                user_agent.as_deref(),
                &ctx,
            )
            .await?;

        let url = self.oauth.authorize_url(p, &state, &nonce, &challenge)?;
        Ok(OauthStartResponse {
            url,
            state,
            binder,
            expires_at: row.expires_at.to_rfc3339(),
        })
    }

    /// Ход второй: провайдер вернул человека на сервер. Возвращает адрес, куда
    /// отправить браузер, — всегда, что бы ни случилось: человека, который
    /// вернулся от провайдера, нельзя оставлять на пустой странице с JSON.
    ///
    /// Здесь ничего не обменивается и никто не впускается (см. описание
    /// модуля): код уезжает в `#`-часть адреса, и обмен делает вкладка с
    /// привязкой.
    pub async fn oauth_callback(
        &self,
        p: Provider,
        state: Option<&str>,
        code: Option<&str>,
        provider_error: Option<&str>,
        apple_user: Option<&str>,
    ) -> String {
        let site = self.config.public_url.trim_end_matches('/').to_string();

        let Some(state) = state.filter(|s| !s.is_empty() && s.len() <= 200) else {
            return oauth_page(&site, &[]);
        };
        let row = match self.repo.peek_oauth_login(state).await {
            Ok(Some(row)) => row,
            Ok(None) => return oauth_page(&site, &[]),
            Err(e) => {
                tracing::warn!("oauth callback: {e}");
                return oauth_page(&site, &[]);
            }
        };

        // Состояние, взятое у одного провайдера, у другого не принимается.
        if row.provider != p.as_str() {
            self.oauth_refuse(state, "failed").await;
            return oauth_page(&row.return_origin, &[("state", state), ("error", "failed")]);
        }
        // Человек нажал «Отмена» у провайдера, либо код не пришёл вовсе.
        let code = code.filter(|c| !c.is_empty() && c.len() <= MAX_CARRIED);
        let (true, Some(code)) = (provider_error.is_none(), code) else {
            self.oauth_refuse(state, "denied").await;
            return oauth_page(&row.return_origin, &[("state", state), ("error", "denied")]);
        };

        let mut carried = vec![("state", state), ("code", code)];
        // Имя Apple приходит из браузера, а не из подписанного токена, поэтому
        // дальше оно идёт как есть и берётся только как имя для профиля.
        if p == Provider::Apple
            && let Some(user) = apple_user.filter(|u| !u.is_empty() && u.len() <= MAX_CARRIED)
        {
            carried.push(("user", user));
        }
        oauth_page(&row.return_origin, &carried)
    }

    /// Записать отказ. Не получилось записать — это не повод молчать: строка
    /// остаётся «в работе» до уборки, и вкладка видит «остыло» без причины.
    async fn oauth_refuse(&self, state: &str, word: &str) {
        if let Err(e) = self.repo.fail_oauth_login(state, word).await {
            tracing::warn!("oauth: could not mark the sign-in as failed ({word}): {e}");
        }
    }

    /// Кого впускать: привязка к названному имени, знакомый `sub`, имя с той же
    /// подтверждённой почтой или первый приход. Возвращает того, кому заводить
    /// сессию, и «заведён только что».
    async fn oauth_admit(
        &self,
        row: &OauthLogin,
        p: Provider,
        who: &Identity,
        apple_name: Option<String>,
    ) -> std::result::Result<(User, bool), Turned> {
        let said_email = who.email.as_deref();

        // Привязка из профиля: имя уже названо, дверь к нему добавляется.
        if let Some(user_id) = row.bind_user_id {
            // Заблокированному дверь не добавляют: блокировка гасит сессии, но
            // привязка могла быть начата до неё.
            let bound = self.repo.find_user_by_id(user_id).await?.ok_or(AppError::Unauthorized)?;
            if bound.is_blocked {
                return Err(Turned::Blocked);
            }
            let user = self.repo.link_oauth_to_user(user_id, p, &who.subject, said_email).await?;
            Self::log_domain_event(
                if p == Provider::Google { "google_linked" } else { "apple_linked" },
                "user",
                user.id,
                "ok",
            );
            return Ok((user, false));
        }

        // Вернувшийся. Узнаём по `sub` — и только по нему.
        if let Some(user) = self.repo.find_user_by_oauth(p, &who.subject).await? {
            // Заблокированному эта дверь закрыта так же, как и первая.
            if user.is_blocked {
                return Err(Turned::Blocked);
            }
            // Адрес провайдера освежается, имя в профиле — никогда. Не
            // освежился — человек всё равно входит, но причина остаётся в журнале.
            let user = match self.repo.touch_oauth_email(user.id, p, said_email).await {
                Ok(fresh) => fresh,
                Err(e) => {
                    tracing::warn!("oauth {}: could not refresh the provider address: {e}", p.as_str());
                    user
                }
            };
            return Ok((user, false));
        }

        // Ящик, за который поручился провайдер. Скрытый адрес Apple — это прокси:
        // письма на него доходят не всегда, и своей почтой дом его не называет.
        let proven = said_email
            .filter(|_| who.email_verified && !who.private_email)
            .and_then(|e| normalize_email(e).ok());

        // Та же подтверждённая почта — тот же человек. Ящик подтверждён с обеих
        // сторон: хозяин имени открыл его письмом, провайдер ручается за него
        // сам. Неподтверждённый адрес не годится — его мог набрать кто угодно.
        // Имя, у которого этот провайдер уже привязан под другим `sub`, не
        // сливается: это другой человек в том же ящике.
        if let Some(email) = proven.as_deref()
            && let Some(user) = self.repo.find_user_by_confirmed_email(email).await?
            && !user.has_oauth(p)
        {
            if user.is_blocked {
                return Err(Turned::Blocked);
            }
            let user = self.repo.link_oauth_to_user(user.id, p, &who.subject, said_email).await?;
            Self::log_domain_event("oauth_linked_by_mail", "user", user.id, "ok");
            return Ok((user, false));
        }

        // Первый приход — заводится НОВОЕ имя, а значит, нужно самоподтверждение
        // возраста (политика: на каждой форме, где собираются контакты). До
        // этого места доходят только те, кого у нас ещё нет: вернувшийся,
        // привязка из профиля и слияние по почте выше уже вернулись. Имя без
        // подтверждения не создаётся вовсе — ни без почты, ни как-либо иначе.
        if !row.age_confirmed {
            return Err(Turned::Age);
        }
        let ctx = ClientContext {
            ip: row.ip.clone(),
            user_agent: row.browser.clone(),
            country_code: row.country_code.clone(),
            city: row.city.clone(),
        };
        // Имя здесь временное: то, что назвал провайдер, а если ничего — его
        // название. Страница спрашивает имя у человека сразу после первого входа.
        // Часть почты до `@` именем не служит: оно видно другим людям.
        let name = oauth_display_name(apple_name.as_deref().or(who.name.as_deref()), p);
        let made = self
            .repo
            .create_oauth_user(p, &who.subject, said_email, proven.as_deref(), &name, &ctx)
            .await?;
        // Адрес успели занять за это время (или он занят неподтверждённым
        // именем) — заводим без почты. Почту спросит первое дело, которому она
        // нужна, как у вошедшего через Telegram.
        let user = match made {
            Some(user) => user,
            None => self
                .repo
                .create_oauth_user(p, &who.subject, said_email, None, &name, &ctx)
                .await?
                .ok_or_else(|| AppError::Internal("oauth user was not created".into()))?,
        };
        Self::log_domain_event(
            if p == Provider::Google { "user_registered_google" } else { "user_registered_apple" },
            "user",
            user.id,
            "ok",
        );
        Ok((user, true))
    }

    /// Ход третий: вкладка предъявляет привязку и код и получает результат.
    /// Один раз.
    pub async fn oauth_redeem(&self, req: &OauthRedeemRequest) -> Result<OauthRedeemResponse> {
        let cold = || OauthRedeemResponse {
            state: "expired".into(),
            provider: None,
            session_token: None,
            user: None,
            is_new: false,
            failure: None,
        };
        // Неизвестное, остывшее, уже взятое и чужое (не та привязка) отвечают
        // одинаково: разница между ними нужна только тому, кто перебирает. Строка
        // берётся только с верной привязкой, поэтому чужой запрос её не сжигает.
        let state = req.state.trim();
        let Some(row) = self
            .repo
            .claim_oauth_login(state, &fingerprint(req.binder.trim()))
            .await?
        else {
            return Ok(cold());
        };

        let failed = |word: &str| OauthRedeemResponse {
            state: "failed".into(),
            provider: Some(row.provider.clone()),
            session_token: None,
            user: None,
            is_new: false,
            failure: Some(word.into()),
        };

        let code = req.code.trim();
        let Some(p) = Provider::parse(&row.provider) else {
            self.oauth_refuse(state, "failed").await;
            return Ok(failed("failed"));
        };
        if code.is_empty() || code.len() > MAX_CARRIED {
            self.oauth_refuse(state, "denied").await;
            return Ok(failed("denied"));
        }

        let who = match self.oauth.exchange(p, code, &row.pkce_verifier, &row.nonce).await {
            Ok(who) => who,
            Err(e) => {
                // В журнал идёт причина провайдера (`redirect_uri_mismatch`,
                // `invalid_client`…): по ней видно, что чинить. Секретов в ней нет.
                tracing::warn!("oauth {} exchange failed: {e}", p.as_str());
                self.oauth_refuse(state, "failed").await;
                return Ok(failed("failed"));
            }
        };

        let apple_name = if p == Provider::Apple {
            req.user
                .as_deref()
                .filter(|u| u.len() <= MAX_CARRIED)
                .and_then(AppleUser::display_name)
        } else {
            None
        };

        let (user, is_new) = match self.oauth_admit(&row, p, &who, apple_name).await {
            Ok(admitted) => admitted,
            Err(turned) => {
                if let Turned::Other(e) = &turned {
                    tracing::warn!("oauth {} admit failed: {e}", p.as_str());
                }
                self.oauth_refuse(state, turned.word()).await;
                return Ok(failed(turned.word()));
            }
        };

        // Привязка из профиля: у человека своя сессия, новой не заводят.
        let session_token = if row.bind_user_id.is_some() {
            None
        } else {
            // Обстоятельства сессии взяты у вкладки, которая просила вход.
            let ctx = ClientContext {
                ip: row.ip.clone(),
                user_agent: row.browser.clone(),
                country_code: row.country_code.clone(),
                city: row.city.clone(),
            };
            match self.open_session(user.id, &ctx).await {
                Ok(token) => Some(token),
                Err(e) => {
                    // Личность подтверждена, а сессию завести не вышло: человек
                    // увидит отказ, а не «остыло», и причина будет в журнале.
                    tracing::warn!("oauth {}: could not open a session for {}: {e}", p.as_str(), user.id);
                    self.oauth_refuse(state, "failed").await;
                    return Ok(failed("failed"));
                }
            }
        };

        if let Err(e) = self.repo.close_oauth_login(state, user.id, is_new).await {
            // Вход уже состоялся; строка досидит до уборки, повторно её не взять.
            tracing::warn!("oauth {}: could not close the sign-in: {e}", p.as_str());
        }
        Ok(OauthRedeemResponse {
            state: "ready".into(),
            provider: Some(row.provider.clone()),
            session_token,
            user: Some(UserDto::from(&user)),
            is_new,
            failure: None,
        })
    }

    /// Отвязать провайдера. Отказ, когда другой двери нет: человек, снявший
    /// единственную, запер бы себя снаружи собственной дверью. Подсчёт и
    /// снятие — одна операция под блокировкой строки (`unlink_door`).
    pub async fn oauth_unlink(&self, user: &User, p: Provider) -> Result<UserDto> {
        if !user.has_oauth(p) {
            return Ok(UserDto::from(user));
        }
        let door = if p == Provider::Google { Door::Google } else { Door::Apple };
        if !self.repo.unlink_door(user.id, door).await? {
            return Err(AppError::BadRequest(format!(
                "{} is the only way in — add another way in first",
                p.as_str()
            )));
        }
        Self::log_domain_event(
            if p == Provider::Google { "google_unlinked" } else { "apple_unlinked" },
            "user",
            user.id,
            "ok",
        );
        let fresh = self.repo.find_user_by_id(user.id).await?.ok_or(AppError::Unauthorized)?;
        Ok(UserDto::from(&fresh))
    }

    /// Уборка за входом: отработавшие и остывшие попытки.
    pub async fn prune_oauth_logins(&self) -> Result<u64> {
        self.repo.prune_oauth_logins().await
    }
}

/// Временное имя для нового профиля: то, что назвал провайдер, а если ничего —
/// название провайдера. Настоящее имя страница спрашивает у человека сразу после
/// первого входа. Часть почты до `@` не годится: имя видно другим людям, а почта
/// — нет.
fn oauth_display_name(said: Option<&str>, p: Provider) -> String {
    let clean = |s: &str| -> String {
        let one_line = s.split_whitespace().collect::<Vec<_>>().join(" ");
        let cut: String = one_line.chars().filter(|c| !c.is_control()).take(80).collect();
        cut.trim().to_string()
    };
    if let Some(name) = said.map(clean).filter(|n| !n.is_empty()) {
        return name;
    }
    match p {
        Provider::Google => "Google".to_string(),
        Provider::Apple => "Apple".to_string(),
    }
}

/// Адрес страницы возврата. Значения идут в `#`-часть, а не в строку запроса:
/// фрагмент не уходит на сервер, не попадает в журналы прокси и в `Referer`, а
/// код в нём одноразовый и без привязки ничего не открывает. Кодирование — то
/// же, что у форм (`url`), страница читает его `URLSearchParams`.
fn oauth_page(origin: &str, pairs: &[(&str, &str)]) -> String {
    let base = format!("{origin}/auth/oauth");
    if pairs.is_empty() {
        return base;
    }
    let mut scratch = reqwest::Url::parse("http://scratch.invalid/").expect("a constant address");
    scratch.query_pairs_mut().extend_pairs(pairs);
    format!("{base}#{}", scratch.query().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_is_what_the_provider_said_then_the_provider() {
        let g = Provider::Google;
        assert_eq!(oauth_display_name(Some("  Мария   Иванова "), g), "Мария Иванова");
        assert_eq!(oauth_display_name(None, g), "Google");
        assert_eq!(oauth_display_name(Some("   "), Provider::Apple), "Apple");
        assert_eq!(oauth_display_name(Some("a\u{0007}b"), g), "ab");
        assert_eq!(oauth_display_name(Some(&"я".repeat(500)), g).chars().count(), 80);
        // Обрезка по 80 знакам не оставляет пробел на конце.
        let spaced = format!("{} {}", "я".repeat(79), "ю");
        assert_eq!(oauth_display_name(Some(&spaced), g), "я".repeat(79));
    }

    #[test]
    fn the_return_address_carries_values_in_the_fragment_and_escapes_them() {
        assert_eq!(oauth_page("https://x.test", &[]), "https://x.test/auth/oauth");
        assert_eq!(
            oauth_page("https://x.test", &[("state", "Ab-_.~09"), ("code", "a b&c=d/e")]),
            "https://x.test/auth/oauth#state=Ab-_.%7E09&code=a+b%26c%3Dd%2Fe"
        );
    }
}
