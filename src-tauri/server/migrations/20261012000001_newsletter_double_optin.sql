-- Книга дома: подтверждение адреса письмом (double opt-in).
--
-- Прежде подпись действовала сразу, и любой мог вписать в книгу чужой адрес:
-- человек получал письма, которых не просил, а дом — жалобы и репутацию
-- отправителя. Теперь адрес становится подписчиком, когда по ссылке из письма
-- перешёл его хозяин.
--
-- «Действующий подписчик» = confirmed_at IS NOT NULL AND unsubscribed_at IS NULL.
ALTER TABLE newsletter_subscribers
    ADD COLUMN IF NOT EXISTS confirmed_at     TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS confirm_hash     TEXT,
    ADD COLUMN IF NOT EXISTS confirm_sent_at  TIMESTAMPTZ;

-- Уже вписанные остаются подписчиками: прежняя запись была согласием по
-- правилам того времени, а переспрашивать всю книгу значило бы потерять её.
UPDATE newsletter_subscribers SET confirmed_at = created_at WHERE confirmed_at IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_subscribers_confirm_hash
    ON newsletter_subscribers (confirm_hash) WHERE confirm_hash IS NOT NULL;
