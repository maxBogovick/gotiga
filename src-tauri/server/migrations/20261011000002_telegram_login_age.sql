-- Самоподтверждение «мне 16 или больше» и для входа через Telegram.
--
-- Тот же закон, что у Google и Apple (20261011000001): отметка едет вместе с
-- попыткой входа и проверяется в единственный миг — когда заводится НОВОЕ имя.
-- Вернувшегося и привязку из профиля она не касается.
ALTER TABLE telegram_login_codes ADD COLUMN IF NOT EXISTS age_confirmed BOOLEAN NOT NULL DEFAULT FALSE;

-- Новое состояние `age`: имени ещё нет, а возраст не подтверждён. Странице
-- нужно слово, чтобы отправить человека на регистрацию, а не сказать «вход
-- отменён в Telegram» — он ничего не отменял.
ALTER TABLE telegram_login_codes DROP CONSTRAINT IF EXISTS telegram_login_codes_state_check;
ALTER TABLE telegram_login_codes ADD CONSTRAINT telegram_login_codes_state_check
    CHECK (state IN ('waiting', 'asked', 'confirmed', 'refused', 'taken', 'age'));
