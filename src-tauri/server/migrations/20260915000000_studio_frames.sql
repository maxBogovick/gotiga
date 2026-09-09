-- Студия: личный склад человека и его рамки.
--
-- Всё, что здесь заводится, — ЛИЧНОЕ. Склад дома (`battle_assets`,
-- `battle_asset_sheets`) остаётся домашним, и ни одна строка этих таблиц не
-- смешивается с ним. Причина та же, по которой проба карты не легла в
-- `battle_cards`: строка чужой непроверенной работы в домашней таблице
-- отделена от «её видно всем» одним условием в одном запросе, а условия
-- забывают.
--
-- Замер склада 7.09.2026, из которого взяты потолки (`STUDIO.md` §3):
-- вырезанная деталь — медиана 12 КБ, максимум 438 КБ; лист под разрез — 2.6 МБ.
-- Лист весит В ДВЕСТИ РАЗ больше детали, поэтому ящик считается по обоим, а
-- листов держится не больше пяти одновременно: иначе суточная квота в двадцать
-- листов забивает все 50 МБ за день, и человек не может положить готовую работу.

-- ── Лист под разрез ─────────────────────────────────────────────────────────
--
-- Хранится ЦЕЛИКОМ, в тех байтах, в которых пришёл, — как и лист хозяина:
-- разрез можно повторить с другими настройками, не спрашивая файл заново.
--
-- `harvested_at` — из листа что-то вырезали. Лист без урожая старше семи дней
-- убирает уборщик: это самый тяжёлый мусор в доме и единственный, который
-- копится сам собой.
CREATE TABLE IF NOT EXISTS studio_sheets (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 80),
    url          TEXT NOT NULL,
    width        INTEGER NOT NULL,
    height       INTEGER NOT NULL,
    bytes        INTEGER NOT NULL CHECK (bytes > 0),
    -- Настройки последнего разреза, как у хозяина (`SliceSettings`).
    settings     TEXT,
    harvested_at TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS studio_sheets_owner_idx
    ON studio_sheets (owner_id, created_at DESC);
-- Уборщику: листы, из которых ничего не вырезали.
CREATE INDEX IF NOT EXISTS studio_sheets_unharvested_idx
    ON studio_sheets (created_at) WHERE harvested_at IS NULL;

-- ── Деталь ──────────────────────────────────────────────────────────────────
--
-- Слова роли — ТЕ ЖЕ, что у склада дома (`battle_assets_role_check`), плюс
-- `paper` на бумагу. Два словаря для одного и того же однажды разошлись бы, и
-- деталь, перенесённая из студии в дом, стала бы деталью неизвестной породы.
--
-- `sheet_id` — откуда взялась, а не кто владеет: лист убрали, детали остались.
-- Тот же `ON DELETE SET NULL` и по той же причине, что у детали хозяина.
CREATE TABLE IF NOT EXISTS studio_assets (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sheet_id   UUID REFERENCES studio_sheets(id) ON DELETE SET NULL,
    name       TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 80),
    role       TEXT NOT NULL CHECK (role IN
                   ('corner', 'sideH', 'sideV', 'accent', 'art', 'paper', 'other')),
    url        TEXT NOT NULL,
    width      INTEGER NOT NULL,
    height     INTEGER NOT NULL,
    bytes      INTEGER NOT NULL CHECK (bytes > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS studio_assets_owner_idx
    ON studio_assets (owner_id, created_at DESC);
CREATE INDEX IF NOT EXISTS studio_assets_sheet_idx ON studio_assets (sheet_id);

-- ── Рамка человека ──────────────────────────────────────────────────────────
--
-- Тело — ОДИН столбец JSON, буква в букву `BattleFrame`, тот самый, которым
-- одеваются карты дома. Тридцать столбцов были бы второй схемой рамки: схема
-- рамки за год приросла слайсами, орнаментами и описью лица, и каждая
-- следующая правка требовала бы близнеца, а забытый близнец — это поле,
-- которое человек назначил, а показ молча потерял.
--
-- Отсюда же главное свойство: отрисовщик остаётся ОДИН. Рамка человека
-- рисуется тем же `BattleCard.svelte`, что и рамка дома, потому что это тот же
-- объект.
--
-- Пять слов состояния:
--   draft      — в работе, видит только автор
--   published  — выложена, ждёт допуска хозяина (`STUDIO.md` §5)
--   entered    — допущена и выставлена в сезон
--   approved   — утверждена, получает тираж лицензий
--   withdrawn  — снята (автором или хозяином)
CREATE TABLE IF NOT EXISTS studio_frames (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 60),
    body         JSONB NOT NULL,
    status       TEXT NOT NULL
                 CHECK (status IN ('draft', 'published', 'entered', 'approved', 'withdrawn'))
                 DEFAULT 'draft',
    -- Сумма веса деталей этой рамки. Не считается запросом, в отличие от
    -- ящика: ящик — это правда о складе, а тут нужен потолок на одну работу
    -- в тот момент, когда её кладут, и join ради него был бы лишним.
    bytes        INTEGER NOT NULL DEFAULT 0 CHECK (bytes >= 0),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Забор от мегабайтного тела. Рамка со всеми деталями, орнаментами и
    -- описью лица укладывается в единицы килобайт.
    CONSTRAINT studio_frames_body_size CHECK (octet_length(body::text) <= 40000)
);

CREATE INDEX IF NOT EXISTS studio_frames_owner_idx
    ON studio_frames (owner_id, updated_at DESC);
-- Очередь допуска: выложенное, чего хозяин ещё не смотрел. Самое старое сверху.
CREATE INDEX IF NOT EXISTS studio_frames_published_idx
    ON studio_frames (updated_at) WHERE status = 'published';

-- ── Библиотека дома ─────────────────────────────────────────────────────────
--
-- Детали, которые хозяин открыл людям: из них новичок собирает рамку в простом
-- сборщике, вообще ничего не рисуя. Отдельной таблицы не нужно — это те же
-- детали склада, просто открытые. Умолчание `false`: весь нынешний склад
-- остаётся закрытым, и открывается только то, что хозяин выбрал руками.
ALTER TABLE battle_assets
    ADD COLUMN IF NOT EXISTS public BOOLEAN NOT NULL DEFAULT false;

CREATE INDEX IF NOT EXISTS battle_assets_public_idx
    ON battle_assets (role, sort_order NULLS LAST) WHERE public;
