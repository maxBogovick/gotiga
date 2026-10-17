-- Пределы «столько-то в час» переезжают из памяти процесса в базу.
--
-- В памяти предел сбрасывался каждым перезапуском (а выкладка перезапускает
-- сервер) и не делился между двумя копиями на одной базе. Одна строка — одно
-- принятое обращение; отказанные не записываются, поэтому строк на ключ не
-- больше его предела.
CREATE TABLE IF NOT EXISTS rate_limit_hits (
    key    TEXT        NOT NULL,
    hit_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_rate_limit_hits_key ON rate_limit_hits (key, hit_at);
CREATE INDEX IF NOT EXISTS idx_rate_limit_hits_at ON rate_limit_hits (hit_at);
