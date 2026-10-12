-- Замер: время визита, откуда пришли и куда ушли, скорость, сбои, поступки.
--
-- Всё идёт в тот же поток событий, а не в отдельные таблицы: суточный хеш
-- посетителя, фильтр ботов, DNT и очередь записи уже есть, и второй поток
-- разошёлся бы с первым в первом же правиле.

-- ── Новые поля ────────────────────────────────────────────────────────────
-- `name` + `detail` + `value` — общая форма «поступка» (interaction) и сбоя
-- (client_error): что (name), о чём (detail), сколько (value). Отдельные
-- колонки, а не `cta_type`: счётчики призывов считают именно `cta_type`, и
-- поступок «поиск» не должен попадать в «нажатий на призыв».
ALTER TABLE figurine_analytics_events
    ADD COLUMN IF NOT EXISTS name         TEXT,
    ADD COLUMN IF NOT EXISTS detail       TEXT,
    ADD COLUMN IF NOT EXISTS value        INTEGER,
    -- Был ли этот браузер на сайте в один из прошлых дней. Флаг, а не номер:
    -- NULL значит «не знаем» (хранилище закрыто), и в долях он не считается.
    ADD COLUMN IF NOT EXISTS is_returning BOOLEAN,
    -- Куда ушли: путь следующей комнаты, `ext:хост` для чужого сайта, `hide`
    -- когда вкладку свернули или закрыли.
    ADD COLUMN IF NOT EXISTS exit_to      TEXT,
    -- Последний заголовок, до которого дочитали: где на странице оборвалось.
    ADD COLUMN IF NOT EXISTS last_section TEXT,
    -- Скорость на устройстве посетителя (`web_vitals`), миллисекунды;
    -- CLS — в тысячных долях (0.1 → 100), чтобы остаться целым числом.
    ADD COLUMN IF NOT EXISTS vital_lcp    INTEGER,
    ADD COLUMN IF NOT EXISTS vital_inp    INTEGER,
    ADD COLUMN IF NOT EXISTS vital_cls    INTEGER,
    ADD COLUMN IF NOT EXISTS vital_ttfb   INTEGER,
    ADD COLUMN IF NOT EXISTS conn         TEXT,
    ADD COLUMN IF NOT EXISTS viewport_w   INTEGER;

ALTER TABLE figurine_analytics_events DROP CONSTRAINT figurine_analytics_event_type_chk;
ALTER TABLE figurine_analytics_events ADD CONSTRAINT figurine_analytics_event_type_chk CHECK (
    event_type IN (
        'figurine_view', 'figurine_engaged', 'figurine_cta_click',
        'page_view', 'page_engaged',
        'interaction', 'client_error', 'not_found', 'web_vitals'
    )
);

-- ── Одна строка на визит ──────────────────────────────────────────────────
-- Время и прокрутка теперь отправляются несколько раз за визит (вкладку
-- свернули, вернулись, прочитали ещё), и последнее слово остаётся за
-- большими значениями. Уникальный индекс — это и есть правило «одна строка»:
-- ON CONFLICT в записи сливает повтор в ту же строку, а запросы медиан не
-- обязаны выбирать «последнее событие визита».
--
-- Старые дубли (если они были) убираются до индекса, иначе он не построится.
DELETE FROM figurine_analytics_events a
USING figurine_analytics_events b
WHERE a.page_view_id IS NOT NULL
  AND a.page_view_id = b.page_view_id
  AND a.event_type = b.event_type
  AND a.event_type IN ('page_engaged', 'figurine_engaged', 'web_vitals')
  AND a.id < b.id;

CREATE UNIQUE INDEX IF NOT EXISTS uq_fae_one_per_visit
    ON figurine_analytics_events (page_view_id, event_type)
    WHERE page_view_id IS NOT NULL
      AND event_type IN ('page_engaged', 'figurine_engaged', 'web_vitals');

CREATE INDEX IF NOT EXISTS idx_fae_interaction
    ON figurine_analytics_events (event_date DESC, name)
    WHERE event_type = 'interaction';
CREATE INDEX IF NOT EXISTS idx_fae_errors
    ON figurine_analytics_events (event_date DESC)
    WHERE event_type IN ('client_error', 'not_found');
CREATE INDEX IF NOT EXISTS idx_fae_vitals
    ON figurine_analytics_events (event_date DESC)
    WHERE event_type = 'web_vitals';

-- ── Постоянные свёртки ────────────────────────────────────────────────────
-- Сырые события живут 30 дней (`analytics::RETENTION_DAYS`). Всё, что нужно
-- сравнивать с прошлым кварталом, лежит здесь и не стирается.

-- Распределение времени и прокрутки по комнате: корзины, а не среднее, —
-- среднее прячет два горба «ушёл сразу» и «читал долго».
CREATE TABLE IF NOT EXISTS site_engagement_daily (
    day            DATE    NOT NULL,
    path_group     TEXT    NOT NULL,
    visits         INTEGER NOT NULL DEFAULT 0,
    dur_lt10       INTEGER NOT NULL DEFAULT 0,   -- < 10 с
    dur_10_30      INTEGER NOT NULL DEFAULT 0,
    dur_30_60      INTEGER NOT NULL DEFAULT 0,
    dur_60_180     INTEGER NOT NULL DEFAULT 0,
    dur_ge180      INTEGER NOT NULL DEFAULT 0,   -- >= 3 мин
    scroll_lt25    INTEGER NOT NULL DEFAULT 0,
    scroll_25_49   INTEGER NOT NULL DEFAULT 0,
    scroll_50_74   INTEGER NOT NULL DEFAULT 0,
    scroll_ge75    INTEGER NOT NULL DEFAULT 0,
    new_visits       INTEGER NOT NULL DEFAULT 0,
    returning_visits INTEGER NOT NULL DEFAULT 0,
    -- Те же визиты, но только вернувшиеся/новые, дольше 30 с: «остаются ли».
    new_long         INTEGER NOT NULL DEFAULT 0,
    returning_long   INTEGER NOT NULL DEFAULT 0,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (day, path_group)
);

-- Куда уходят из комнаты. `exit_to` — комната-назначение (`to:work`),
-- `ext:хост` или `hide`.
CREATE TABLE IF NOT EXISTS site_exits_daily (
    day        DATE    NOT NULL,
    path_group TEXT    NOT NULL,
    exit_to    TEXT    NOT NULL,
    visits     INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (day, path_group, exit_to)
);

CREATE TABLE IF NOT EXISTS site_interactions_daily (
    day         DATE    NOT NULL,
    name        TEXT    NOT NULL,
    detail      TEXT    NOT NULL DEFAULT '',
    events      INTEGER NOT NULL DEFAULT 0,
    visitors    INTEGER NOT NULL DEFAULT 0,
    -- События с value = 0: у поиска это «ничего не нашлось».
    zero_events INTEGER NOT NULL DEFAULT 0,
    value_sum   BIGINT  NOT NULL DEFAULT 0,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (day, name, detail)
);

CREATE TABLE IF NOT EXISTS site_errors_daily (
    day        DATE    NOT NULL,
    kind       TEXT    NOT NULL,
    path_group TEXT    NOT NULL,
    detail     TEXT    NOT NULL DEFAULT '',
    events     INTEGER NOT NULL DEFAULT 0,
    visitors   INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (day, kind, path_group, detail)
);

-- Скорость: 75-й процентиль и доли «хорошо» по порогам Core Web Vitals
-- (LCP ≤ 2500 мс, INP ≤ 200 мс, CLS ≤ 0.1).
CREATE TABLE IF NOT EXISTS site_vitals_daily (
    day          DATE NOT NULL,
    path_group   TEXT NOT NULL,
    device_class TEXT NOT NULL DEFAULT 'unknown',
    samples      INTEGER NOT NULL DEFAULT 0,
    lcp_n        INTEGER NOT NULL DEFAULT 0,
    lcp_p75      INTEGER,
    lcp_good     INTEGER NOT NULL DEFAULT 0,
    inp_n        INTEGER NOT NULL DEFAULT 0,
    inp_p75      INTEGER,
    inp_good     INTEGER NOT NULL DEFAULT 0,
    cls_n        INTEGER NOT NULL DEFAULT 0,
    cls_p75      INTEGER,
    cls_good     INTEGER NOT NULL DEFAULT 0,
    ttfb_p75     INTEGER,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (day, path_group, device_class)
);

-- Автоматические заметки на графике (выкладка, выход байки, письма, пост в
-- канале) отличаются от ручных, чтобы их можно было отфильтровать и чтобы
-- повтор в тот же день не множил строки.
ALTER TABLE analytics_annotations ADD COLUMN IF NOT EXISTS auto BOOLEAN NOT NULL DEFAULT FALSE;
CREATE UNIQUE INDEX IF NOT EXISTS uq_analytics_annotations_auto
    ON analytics_annotations (day, label) WHERE auto;
