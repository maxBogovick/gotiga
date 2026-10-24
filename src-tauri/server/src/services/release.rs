//! Пост о релизе в Telegram-каналах.
//!
//! Заметки пишутся в `release-notes.md` перед выкладкой и встраиваются в
//! сервер. После запуска нового релиза сервер заводит черновик, а отправляет
//! его администратор кнопкой: текст поста нельзя поправить, когда он вышел, и
//! откат или повторная выкладка не должны выпускать второй пост.
//!
//! Что вышло на сайте с прошлого релиза (работы, байки, карты и этюды),
//! считается из журнала каналов, а не пишется руками: журнал замечает вещь,
//! как только она оказалась на людях.

use super::*;
use crate::channel::{self, clean_notes, release_button, release_caption, Notes, ReleaseNews};
use crate::lang::Lang;

/// Встроенный файл заметок. Лежит внутри `src-tauri/server`, потому что
/// контекст сборки образа — `src-tauri/`, и всё, что снаружи сервера, в
/// образ не попадает.
const RELEASE_NOTES: &str = include_str!("../../release-notes.md");

impl AppService {
    /// Завести черновик, если в файле заметок появился релиз, о котором ещё
    /// не заводили. Возвращает номер релиза, если черновик завёлся.
    ///
    /// Каналов нет — черновика нет: сравнивать «что вышло с прошлого раза»
    /// без журнала нечем, а поста некуда отправить.
    pub async fn prepare_release_draft(&self) -> Result<Option<String>> {
        if self.channels().is_empty() {
            return Ok(None);
        }
        let blocks = channel::parse_release_notes(RELEASE_NOTES);
        let Some(block) = blocks.into_iter().next() else {
            return Ok(None);
        };
        if self.repo.release_get(&block.id).await?.is_some() {
            return Ok(None);
        }
        let since = self.repo.release_latest().await?.map(|r| r.created_at);
        let json = serde_json::to_string(&block.notes)
            .map_err(|e| AppError::Internal(format!("release notes: {e}")))?;
        if !self.repo.release_create(&block.id, since, &json).await? {
            return Ok(None);
        }
        tracing::info!(
            "Release {} has no post yet: a draft is waiting in the admin, tab Channel",
            block.id
        );

        // Владельцу уходит записка, куда нажать: черновик, о котором не
        // знают, так и лежит.
        if !self.config.public_url_is_local() {
            if let (Some(bot), Some(chat)) = (self.channel_bot(), self.config.telegram_chat_id.as_deref()) {
                let link = format!("{}/admin#channel", self.config.public_url.trim_end_matches('/'));
                bot.tell(
                    chat,
                    &format!(
                        "✦ Release {}: the post for the channels is ready.\n<a href=\"{}\">Check and send</a>",
                        crate::telegram::esc(&block.id),
                        crate::telegram::esc(&link)
                    ),
                )
                .await;
            }
        }
        Ok(Some(block.id))
    }

    /// Всё для вкладки «Канал».
    pub async fn admin_channel_overview(&self) -> Result<AdminChannelOverview> {
        let queue = self.repo.channel_queue(CHANNEL_MAX_ATTEMPTS).await?;
        let channels = self
            .channels()
            .into_iter()
            .map(|(lang, chat)| {
                let (waiting, failed) = queue
                    .iter()
                    .find(|(l, _, _)| l == lang.code())
                    .map(|(_, w, f)| (*w, *f))
                    .unwrap_or((0, 0));
                AdminChannelInfo { lang: lang.code().to_string(), chat, waiting, failed }
            })
            .collect();
        let release = match self.repo.release_latest().await? {
            Some(row) => Some(self.release_dto(row).await?),
            None => None,
        };
        Ok(AdminChannelOverview {
            bot_ready: self.channel_bot().is_some(),
            local: self.config.public_url_is_local(),
            channels,
            release,
        })
    }

    /// Что вышло на сайте после `since`, по спискам на языке каждого канала.
    async fn release_news(
        &self,
        langs: &[Lang],
        since: Option<DateTime<Utc>>,
    ) -> Result<HashMap<Lang, ReleaseNews>> {
        let mut out: HashMap<Lang, ReleaseNews> =
            langs.iter().map(|l| (*l, ReleaseNews::default())).collect();
        let Some(since) = since else {
            return Ok(out); // первый релиз: сравнивать не с чем
        };
        let fresh = self.repo.channel_new_since(since).await?;
        if fresh.is_empty() || langs.is_empty() {
            return Ok(out);
        }
        let posts = self.channel_on_display(langs).await?;
        for (kind, id) in fresh {
            for lang in langs {
                let Some(post) = posts.get(lang).and_then(|p| p.get(&(kind.clone(), id))) else {
                    continue; // с тех пор её снова скрыли
                };
                let item = channel::NewItem { title: post.title.clone(), url: post.url.clone() };
                let news = out.get_mut(lang).expect("язык канала");
                match post.group {
                    channel::Group::Works => news.works.push(item),
                    channel::Group::Tales => news.tales.push(item),
                    channel::Group::Game => news.game.push(item),
                    channel::Group::Other => {}
                }
            }
        }
        Ok(out)
    }

    fn release_link(&self, news: &ReleaseNews, lang: Lang) -> String {
        format!(
            "{}{}?src=telegram&lang={}",
            self.config.public_url.trim_end_matches('/'),
            news.section(),
            lang.code()
        )
    }

    async fn release_dto(&self, row: ReleasePostRow) -> Result<AdminRelease> {
        let notes: Notes = serde_json::from_str(&row.notes).unwrap_or_default();
        let channels = self.channels();
        let langs: Vec<Lang> = channels.iter().map(|(l, _)| *l).collect();
        let news = self.release_news(&langs, row.since).await?;
        let sends = self.repo.release_sends(&row.id).await?;
        let mut out = Vec::new();
        for lang in &langs {
            let code = lang.code();
            let sent = sends.iter().find(|s| s.lang == code);
            let lang_news = news.get(lang).cloned().unwrap_or_default();
            // Вышедший пост показывается таким, каким вышел.
            let caption = match sent.filter(|s| s.posted_at.is_some()) {
                Some(s) => Some(s.caption.clone()),
                None => release_caption(*lang, &row.id, &notes, &lang_news),
            };
            let written_in = channel::notes_for(*lang, &notes)
                .map(|(_, l)| l)
                .filter(|l| l != lang)
                .map(|l| l.code().to_string());
            out.push(AdminReleaseChannel {
                lang: code.to_string(),
                caption,
                written_in,
                link: self.release_link(&lang_news, *lang),
                posted_at: sent.and_then(|s| s.posted_at).map(|t| t.to_rfc3339()),
                attempts: sent.map(|s| s.attempts).unwrap_or(0),
                last_error: sent.and_then(|s| s.last_error.clone()),
            });
        }
        let news_count = langs
            .first()
            .and_then(|l| news.get(l))
            .map(|n| n.works.len() + n.tales.len() + n.game.len())
            .unwrap_or(0);
        Ok(AdminRelease {
            id: row.id,
            created_at: row.created_at.to_rfc3339(),
            since: row.since.map(|t| t.to_rfc3339()),
            status: row.status,
            notes,
            channels: out,
            news_count,
        })
    }

    async fn release_row(&self, id: &str) -> Result<ReleasePostRow> {
        self.repo
            .release_get(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Release".into()))
    }

    /// Сохранить пункты черновика. Присланное приводится в порядок на сервере
    /// (`clean_notes`): форме верить нельзя.
    pub async fn admin_release_save_notes(&self, id: &str, notes: Notes) -> Result<AdminChannelOverview> {
        self.release_row(id).await?;
        let json = serde_json::to_string(&clean_notes(&notes))
            .map_err(|e| AppError::Internal(format!("release notes: {e}")))?;
        if !self.repo.release_save_notes(id, &json).await? {
            return Err(AppError::Conflict("This release post is already closed".into()));
        }
        self.admin_channel_overview().await
    }

    /// Не отправлять этот черновик.
    pub async fn admin_release_dismiss(&self, id: &str) -> Result<AdminChannelOverview> {
        let row = self.release_row(id).await?;
        if row.status == "draft" {
            self.repo.release_set_status(id, "dismissed").await?;
        }
        self.admin_channel_overview().await
    }

    /// Отправить пост о релизе в каналы. `langs` пуст — во все, куда ещё не
    /// ушло. Вышедший канал второй раз не получает, неудавшийся можно
    /// отправить снова.
    pub async fn admin_release_send(&self, id: &str, wanted: &[String]) -> Result<AdminChannelOverview> {
        let row = self.release_row(id).await?;
        if row.status != "draft" {
            return Err(AppError::Conflict("This release post is already closed".into()));
        }
        let Some(bot) = self.channel_bot() else {
            return Err(AppError::BadRequest("The notification bot is not set up".into()));
        };
        let wanted: Vec<Lang> = wanted.iter().filter_map(|c| Lang::parse(c)).collect();
        let channels = self.channels();
        let langs: Vec<Lang> = channels.iter().map(|(l, _)| *l).collect();
        let notes: Notes = serde_json::from_str(&row.notes).unwrap_or_default();
        let news = self.release_news(&langs, row.since).await?;
        let sends = self.repo.release_sends(id).await?;

        let mut anything = false;
        for (lang, chat) in &channels {
            if !wanted.is_empty() && !wanted.contains(lang) {
                continue;
            }
            let code = lang.code();
            if sends.iter().any(|s| s.lang == code && s.posted_at.is_some()) {
                continue;
            }
            let lang_news = news.get(lang).cloned().unwrap_or_default();
            let Some(caption) = release_caption(*lang, id, &notes, &lang_news) else {
                continue;
            };
            anything = true;
            self.repo.release_send_prepare(id, code, &caption).await?;
            let url = self.release_link(&lang_news, *lang);

            // Локально пост не уходит, как и объявления: вместо действия —
            // строка в журнале.
            if self.config.public_url_is_local() {
                tracing::info!("Telegram release post {code} (local, not sent): {url}\n{caption}");
                self.repo.release_send_posted(id, code, None).await?;
                continue;
            }
            match bot.post_note(chat, &caption, release_button(*lang), &url).await {
                Ok(message_id) => self.repo.release_send_posted(id, code, Some(message_id)).await?,
                Err(why) => {
                    tracing::warn!("Telegram release post {id} to {code} failed: {why}");
                    self.repo.release_send_failed(id, code, &why).await?;
                }
            }
        }
        if !anything {
            return Err(AppError::BadRequest(
                "Nothing to post: the notes are empty and nothing new came out".into(),
            ));
        }

        let sends = self.repo.release_sends(id).await?;
        let all_out = channels
            .iter()
            .all(|(l, _)| sends.iter().any(|s| s.lang == l.code() && s.posted_at.is_some()));
        if all_out {
            self.repo.release_set_status(id, "sent").await?;
            self.note_on_chart(&format!("Release post: {id}")).await;
        }
        self.admin_channel_overview().await
    }
}
