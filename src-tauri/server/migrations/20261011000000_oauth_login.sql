-- Вход через Google и Apple: ещё две двери в тот же архив.
--
-- Устроено как Telegram (20261001000000): вернувшегося узнают по ЧИСЛУ, то есть
-- по `sub` — постоянному идентификатору человека у провайдера. Почта для этого
-- не годится: её человек меняет, а у Apple она вообще может быть скрытым
-- адресом-прокси (`…@privaterelay.appleid.com`).
--
-- `*_email` хранится только чтобы напечатать в профиле «Google: маша@…» и
-- переписывается при каждом входе. Имя в профиле после создания аккаунта НЕ
-- трогается — по тому же закону, что у `telegram_username`.
ALTER TABLE users ADD COLUMN IF NOT EXISTS google_sub       TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS google_email     TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS google_linked_at TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS apple_sub        TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS apple_email      TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS apple_linked_at  TIMESTAMPTZ;

CREATE UNIQUE INDEX IF NOT EXISTS users_google_sub_idx ON users (google_sub)
    WHERE google_sub IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS users_apple_sub_idx ON users (apple_sub)
    WHERE apple_sub IS NOT NULL;

-- Одна попытка входа через провайдера.
--
-- Дорога: страница просит ссылку (`started`) → человек уходит к провайдеру →
-- провайдер возвращает его на callback сервера, и сервер ОДИН раз обменивает
-- код на личность (`working` → `done` или `failed`) → браузер возвращается на
-- сайт и забирает результат (`taken`). Сессия при этом в базу не кладётся
-- вовсе: она заводится в момент `taken`, и лежать ей негде.
--
-- `binder_hash` — защита от подсунутой ссылки. Злоумышленник проходит вход у
-- себя до самого callback, но не открывает его, а присылает ссылку жертве;
-- без привязки браузер жертвы оказался бы вошедшим в чужое имя. Привязка —
-- секрет, который знает только вкладка, просившая вход (лежит в её
-- sessionStorage), и забрать результат без него нельзя.
CREATE TABLE IF NOT EXISTS oauth_logins (
    id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- То, что провайдер вернёт нам в callback. Длинное и случайное.
    state TEXT UNIQUE NOT NULL,
    provider TEXT NOT NULL CHECK (provider IN ('google', 'apple')),

    -- Секреты самого обмена с провайдером. Живут минуты и без секрета клиента
    -- (он только в окружении сервера) ничего не открывают.
    nonce         TEXT NOT NULL,
    pkce_verifier TEXT NOT NULL,

    -- SHA-256 от привязки, которую получила вкладка.
    binder_hash TEXT NOT NULL,

    -- На какой сайт вернуть браузер. В проде это PUBLIC_URL; на стенде фронт
    -- стоит на другом порту, чем API, и вернуть надо туда, откуда пришли.
    return_origin TEXT NOT NULL,

    -- NULL — вход или первый приход. Заполнено — привязка провайдера к этому
    -- аккаунту из профиля: тот же обряд, но в конце не сессия, а вторая дверь.
    bind_user_id UUID REFERENCES users(id) ON DELETE CASCADE,

    -- started — ссылка выдана, человек у провайдера
    -- working — callback пришёл, идёт обмен (одноразово: повтор не пройдёт)
    -- done    — личность подтверждена, `user_id` назван
    -- failed  — отказ; причина в `failure`
    -- taken   — вкладка забрала результат
    status TEXT NOT NULL DEFAULT 'started'
        CHECK (status IN ('started', 'working', 'done', 'failed', 'taken')),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    -- Имя завели только что: странице это нужно, чтобы поздороваться иначе.
    is_new  BOOLEAN NOT NULL DEFAULT FALSE,
    -- denied | blocked | conflict | failed — слово для страницы, не текст ошибки.
    failure TEXT,

    -- Откуда просили вход: сессия достаётся ТОМУ браузеру, который просил.
    browser      TEXT,
    ip           TEXT,
    country_code TEXT,
    city         TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '10 minutes',
    taken_at   TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_oauth_logins_expires ON oauth_logins (expires_at);
