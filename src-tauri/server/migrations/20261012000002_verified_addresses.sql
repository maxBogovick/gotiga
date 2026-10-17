-- Письма уходят только на адреса, чей хозяин доказал ящик.
--
-- 1. Просьбы о письме по байкам («хочу продолжение», голос «о ком следующая»),
--    оставленные до проверки адреса. Адрес без доказательства — не обязательно
--    адрес просящего, а письмо дома уходило бы любому постороннему. Адрес
--    стирается из просьбы, сама просьба и голос остаются: стол по-прежнему
--    видит, чего ждут, просто писать некуда. Доказанный адрес — подтверждённый
--    подписчик книги или подтверждённая почта имени.
UPDATE tale_sequel_wishes w SET email = NULL
 WHERE w.email IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM newsletter_subscribers s
                    WHERE lower(s.email) = lower(w.email)
                      AND s.confirmed_at IS NOT NULL AND s.unsubscribed_at IS NULL)
   AND NOT EXISTS (SELECT 1 FROM users u
                    WHERE lower(u.email) = lower(w.email) AND u.email_confirmed_at IS NOT NULL);

UPDATE tale_poll_votes v SET email = NULL
 WHERE v.email IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM newsletter_subscribers s
                    WHERE lower(s.email) = lower(v.email)
                      AND s.confirmed_at IS NOT NULL AND s.unsubscribed_at IS NULL)
   AND NOT EXISTS (SELECT 1 FROM users u
                    WHERE lower(u.email) = lower(v.email) AND u.email_confirmed_at IS NOT NULL);

-- Письма, уже разложенные по таким адресам, но не ушедшие. Повод `new`
-- проверяется в миг отправки (подтверждённый подписчик), поэтому не трогается.
DELETE FROM tale_letters l
 WHERE l.sent_at IS NULL AND l.reason IN ('sequel', 'chosen')
   AND NOT EXISTS (SELECT 1 FROM newsletter_subscribers s
                    WHERE lower(s.email) = lower(l.email)
                      AND s.confirmed_at IS NOT NULL AND s.unsubscribed_at IS NULL)
   AND NOT EXISTS (SELECT 1 FROM users u
                    WHERE lower(u.email) = lower(l.email) AND u.email_confirmed_at IS NOT NULL);

-- 2. Слежение за эскизом: письмо «работа готова» уходит позже и само, поэтому
--    адрес подтверждается тем же способом, что и книга дома.
ALTER TABLE gazette_watches
    ADD COLUMN IF NOT EXISTS confirmed_at    TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS confirm_hash    TEXT,
    ADD COLUMN IF NOT EXISTS confirm_sent_at TIMESTAMPTZ;

-- Прежние слежения подтверждены, только если адрес доказан иначе. Остальные
-- письма не получат; автор по-прежнему видит их в сводке и может написать сам.
UPDATE gazette_watches w SET confirmed_at = NOW()
 WHERE w.confirmed_at IS NULL
   AND (EXISTS (SELECT 1 FROM newsletter_subscribers s
                 WHERE lower(s.email) = lower(w.email)
                   AND s.confirmed_at IS NOT NULL AND s.unsubscribed_at IS NULL)
        OR EXISTS (SELECT 1 FROM users u
                    WHERE lower(u.email) = lower(w.email) AND u.email_confirmed_at IS NOT NULL));

CREATE UNIQUE INDEX IF NOT EXISTS idx_gazette_watches_confirm_hash
    ON gazette_watches (confirm_hash) WHERE confirm_hash IS NOT NULL;
