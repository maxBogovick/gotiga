-- Одноразовое слово для входа через Telegram.
--
-- Устроено как вход на телевизоре: страница показывает СЛОВО, бот называет то
-- же слово, и человек сверяет их сам. Без слова схема ломается одним приёмом —
-- злоумышленник открывает вход у себя, присылает свою ссылку жертве («нажмите,
-- получите скидку»), жертва жмёт ЗАПУСТИТЬ, и чужой архив открыт. Сверять
-- жертве не с чем: своего экрана со словом у неё нет.
--
-- Поэтому записка бота называет и слово, и откуда пришёл запрос (браузер,
-- город): «Chrome, Москва» человеку в Вологде говорит достаточно.
CREATE TABLE IF NOT EXISTS telegram_login_codes (
    id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Секрет в ссылке `t.me/бот?start=<code>`. Длинный и случайный: по нему
    -- находят строку, и подобрать его должно быть нельзя.
    code TEXT UNIQUE NOT NULL,
    -- То, что сверяет человек. Короткое и читаемое вслух.
    word TEXT NOT NULL,

    -- waiting  — ссылка выдана, бота ещё не открывали
    -- asked    — бот получил /start и показал записку, ждём кнопку
    -- confirmed— нажали «это я», сессия лежит в session_token
    -- refused  — нажали «не я»
    -- taken    — страница забрала сессию; строка больше ничего не отдаёт
    state TEXT NOT NULL DEFAULT 'waiting'
        CHECK (state IN ('waiting', 'asked', 'confirmed', 'refused', 'taken')),

    -- Откуда просили вход — только чтобы бот назвал это человеку.
    browser      TEXT,
    country_code TEXT,
    city         TEXT,
    ip           TEXT,

    -- NULL — вход или первый приход. Заполнено — привязка Telegram к этому
    -- аккаунту из профиля: то же слово, но в конце не сессия, а вторая дверь.
    bind_user_id UUID REFERENCES users(id) ON DELETE CASCADE,

    -- Что пришло из Telegram. `telegram_id` — то самое число, по которому
    -- узнают вернувшегося; имя и `@имя` только для записки и профиля.
    telegram_id         BIGINT,
    telegram_username   TEXT,
    telegram_first_name TEXT,
    -- Куда отвечать и какую записку править после нажатия кнопки.
    chat_id    BIGINT,
    message_id BIGINT,

    -- Выдаётся странице ОДИН раз, после чего строка уходит в `taken`.
    session_token TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '5 minutes',
    taken_at   TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_telegram_login_codes_expires
    ON telegram_login_codes (expires_at);
