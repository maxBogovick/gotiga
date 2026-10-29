-- Недельный разбор быстрых уходов (`services/digest.rs`, `digest.rs`).
--
-- Постоянные свёртки по комнатам (`site_engagement_daily`) не знают, откуда пришёл
-- посетитель и с чего смотрел: быстрый уход «со Pinterest на телефоне» лежал только в сырых
-- событиях, которые живут 30 дней. Эта свёртка хранит тот же срез навсегда, и по ней
-- неделя сравнивается с четырьмя прежними.

CREATE TABLE IF NOT EXISTS site_engagement_source_daily (
    day              DATE    NOT NULL,
    path_group       TEXT    NOT NULL,
    -- Откуда пришёл: `utm_source`, иначе хост ссылающейся страницы, иначе вид источника
    -- (`direct`, `internal`, `search`). Свои переходы и прямые заходы — всегда одним словом.
    origin           TEXT    NOT NULL,
    device_class     TEXT    NOT NULL DEFAULT 'unknown',
    visits           INTEGER NOT NULL DEFAULT 0,
    -- Быстрый уход: присутствие короче `analytics::QUICK_EXIT_MS`.
    quick            INTEGER NOT NULL DEFAULT 0,
    -- Из быстрых — те, кто и не прокрутил (меньше 25%): страница не дала повода двигаться.
    quick_unscrolled INTEGER NOT NULL DEFAULT 0,
    -- Остались дольше 30 с.
    engaged          INTEGER NOT NULL DEFAULT 0,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (day, path_group, origin, device_class)
);

-- Один разбор на неделю (понедельник — ключ). Находки лежат готовым JSON: разбор —
-- снимок на тот день, правила потом меняются, и пересчёт задним числом показал бы
-- другое, чем было отправлено.
CREATE TABLE IF NOT EXISTS insight_digests (
    week_start     DATE        PRIMARY KEY,
    made_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    visits         INTEGER     NOT NULL DEFAULT 0,
    quick          INTEGER     NOT NULL DEFAULT 0,
    findings       JSONB       NOT NULL DEFAULT '[]'::jsonb,
    -- Ушло ли владельцу в Telegram; пока пусто, проход повторяет попытку.
    telegram_at    TIMESTAMPTZ,
    telegram_tries INTEGER     NOT NULL DEFAULT 0
);
