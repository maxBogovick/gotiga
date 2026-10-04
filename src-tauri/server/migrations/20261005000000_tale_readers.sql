-- Читатель небылиц после последней строки: «хочу продолжение», голос «о ком
-- записать следующую» и письмо, когда новая байка вышла.
--
-- Всё это сходится в одном месте — в письме о байке. Поэтому очередь писем
-- одна на три повода, а не три рассылки: человек, который и просил
-- продолжение, и вписан в книгу дома, получает об одной байке одно письмо.

-- Байка может продолжать другую. Колонка на листе, как `shelf_order` и
-- `author`: байка — это лист газеты. Начало удалили — продолжение остаётся
-- байкой, поэтому SET NULL.
ALTER TABLE gazette_leaves
    ADD COLUMN IF NOT EXISTS sequel_of UUID REFERENCES gazette_leaves(id) ON DELETE SET NULL;

-- Когда по байке разложены письма. NULL — ещё не разложены.
ALTER TABLE gazette_leaves
    ADD COLUMN IF NOT EXISTS letters_at TIMESTAMPTZ;

-- Всё, что уже побывало на людях, отмечается разложенным: иначе первый же
-- проход разослал бы книге дома всю полку разом. Архивная байка тоже там
-- побывала, а отличить её от никогда не выходившей по `published_at` нельзя:
-- архивирование обнуляет его. Без неё в этом списке возвращённая на полку
-- архивная байка ушла бы всей книге как «новая небылица».
UPDATE gazette_leaves
   SET letters_at = NOW()
 WHERE kind = 'tale'
   AND letters_at IS NULL
   AND (status IN ('published', 'archived')
        OR (status = 'scheduled' AND scheduled_at IS NOT NULL AND scheduled_at <= NOW()));

CREATE INDEX IF NOT EXISTS gazette_leaves_sequel_idx
    ON gazette_leaves (sequel_of)
    WHERE sequel_of IS NOT NULL;

-- «Хочу продолжение»: одна строка на читателя и байку. Почта необязательна —
-- просьба без почты всё равно говорит дому, чего ждут; с почтой ещё и
-- приносит письмо, когда продолжение выйдет.
-- `by_telegram` — вошедший через Telegram попросил сообщить туда. Номер чата
-- здесь не хранится: он берётся из имени в миг отправки (`tale_notes`).
CREATE TABLE IF NOT EXISTS tale_sequel_wishes (
    tale_id       UUID NOT NULL REFERENCES gazette_leaves(id) ON DELETE CASCADE,
    visitor_token VARCHAR(64) NOT NULL,
    user_id       UUID REFERENCES users(id) ON DELETE SET NULL,
    email         TEXT CHECK (email IS NULL OR char_length(email) <= 200),
    by_telegram   BOOLEAN NOT NULL DEFAULT FALSE,
    lang          TEXT NOT NULL DEFAULT 'en' CHECK (lang IN ('en', 'ru')),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (tale_id, visitor_token)
);

-- Голосование «о ком записать следующую небылицу». Кандидатов назначает
-- хозяин, поэтому любой исход годится: это решение, а не замер, и три
-- голоса уже решают. Открыто одно голосование за раз.
CREATE TABLE IF NOT EXISTS tale_polls (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    state              TEXT NOT NULL DEFAULT 'open' CHECK (state IN ('open', 'closed')),
    winner_figurine_id UUID REFERENCES figurines(id) ON DELETE SET NULL,
    -- Байка, которой обещание исполнено. Пока пусто — полка говорит «пишется».
    fulfilled_leaf_id  UUID REFERENCES gazette_leaves(id) ON DELETE SET NULL,
    opened_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at          TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS tale_polls_one_open
    ON tale_polls ((state))
    WHERE state = 'open';

CREATE TABLE IF NOT EXISTS tale_poll_candidates (
    poll_id     UUID NOT NULL REFERENCES tale_polls(id) ON DELETE CASCADE,
    figurine_id UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    position    SMALLINT NOT NULL,
    PRIMARY KEY (poll_id, figurine_id)
);

-- Один голос на читателя в голосовании, и он же меняется, пока голосование
-- открыто. Голос ссылается на пару (голосование, кандидат): отдать его
-- работе, которую не выставляли, схема не даст.
CREATE TABLE IF NOT EXISTS tale_poll_votes (
    poll_id       UUID NOT NULL,
    visitor_token VARCHAR(64) NOT NULL,
    user_id       UUID REFERENCES users(id) ON DELETE SET NULL,
    figurine_id   UUID NOT NULL,
    email         TEXT CHECK (email IS NULL OR char_length(email) <= 200),
    lang          TEXT NOT NULL DEFAULT 'en' CHECK (lang IN ('en', 'ru')),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (poll_id, visitor_token),
    FOREIGN KEY (poll_id, figurine_id)
        REFERENCES tale_poll_candidates (poll_id, figurine_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tale_poll_votes_choice
    ON tale_poll_votes (poll_id, figurine_id);

-- Очередь писем о байке: одна строка на байку и адрес. Повод записан в
-- строке, и первым раскладывается самый личный — продолжение, потом выбор
-- читателей, потом книга дома, — поэтому совпавший адрес получает то письмо,
-- которое сам просил.
--
-- Ссылки отписки здесь нет: состоит ли адрес в книге, спрашивается в миг
-- отправки, а не раскладки. Письмо может ждать в очереди до недели (почта
-- лежала), и отписавшийся за это время не должен получить «новую небылицу».
--
-- `claimed_at` — письмо взято на отправку. Две копии сервера на одной базе
-- (выкладка внахлёст) иначе выбрали бы одни и те же строки и отправили
-- каждое письмо дважды. Взятое и не отмеченное ушедшим возвращается в очередь
-- по истечении срока взятия — если процесс упал посреди прохода.
CREATE TABLE IF NOT EXISTS tale_letters (
    tale_id    UUID NOT NULL REFERENCES gazette_leaves(id) ON DELETE CASCADE,
    email      TEXT NOT NULL,
    lang       TEXT NOT NULL DEFAULT 'en' CHECK (lang IN ('en', 'ru')),
    reason     TEXT NOT NULL CHECK (reason IN ('sequel', 'chosen', 'new')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    claimed_at TIMESTAMPTZ,
    sent_at    TIMESTAMPTZ,
    attempts   INTEGER NOT NULL DEFAULT 0,
    last_error TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS tale_letters_one_per_address
    ON tale_letters (tale_id, lower(email));

CREATE INDEX IF NOT EXISTS idx_tale_letters_due
    ON tale_letters (created_at)
    WHERE sent_at IS NULL;

-- Записки в Telegram о вышедшем продолжении — вторая дорога того же
-- уведомления, для вошедших через Telegram. Пишет бот входа: человек сам
-- начинал с ним разговор, а бот уведомлений ему не знаком. Устроена так же,
-- как очередь писем (взятие, попытки, неделя срока), но ключ — имя, а не
-- адрес: номер чата берётся из имени в миг отправки, и отвязавший Telegram
-- записки не получает.
CREATE TABLE IF NOT EXISTS tale_notes (
    tale_id    UUID NOT NULL REFERENCES gazette_leaves(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    lang       TEXT NOT NULL DEFAULT 'en' CHECK (lang IN ('en', 'ru')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    claimed_at TIMESTAMPTZ,
    sent_at    TIMESTAMPTZ,
    attempts   INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    PRIMARY KEY (tale_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_tale_notes_due
    ON tale_notes (created_at)
    WHERE sent_at IS NULL;
