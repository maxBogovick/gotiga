-- Адрес автора.
--
-- Зал авторов — публичная страница, на которую ссылаются и которую индексируют;
-- `uuid` в адресе такую страницу делает нечитаемой и непередаваемой словами.
--
-- Слуг назначается ЛЕНИВО — когда первую работу человека допустили на люди. До
-- этого адреса у него нет и быть не должно: страницы, на которой ничего нет,
-- в доме не заводят.
ALTER TABLE users ADD COLUMN IF NOT EXISTS studio_slug TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS users_studio_slug_idx ON users (studio_slug)
    WHERE studio_slug IS NOT NULL;
