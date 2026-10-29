-- Мастер заявки: способы связи, пожелание, выбранные признаки, размер, срок, бюджет.
-- Почта больше не обязательна: достаточно любого способа связи в `contacts`.

ALTER TABLE commissions ALTER COLUMN requester_email DROP NOT NULL;

ALTER TABLE commissions
    ADD COLUMN contacts        JSONB NOT NULL DEFAULT '[]',
    ADD COLUMN wish            TEXT,
    ADD COLUMN choices         JSONB NOT NULL DEFAULT '{}',
    ADD COLUMN size_key        TEXT,
    ADD COLUMN deadline_kind   TEXT,
    ADD COLUMN budget_key      TEXT,
    ADD COLUMN budget_currency TEXT;

ALTER TABLE commissions
    ADD CONSTRAINT commissions_wish_check
        CHECK (wish IS NULL OR wish IN ('similar', 'new')),
    ADD CONSTRAINT commissions_size_key_check
        CHECK (size_key IS NULL OR size_key IN ('xs', 's', 'm', 'l', 'xl', 'unsure')),
    ADD CONSTRAINT commissions_deadline_kind_check
        CHECK (deadline_kind IS NULL OR deadline_kind IN ('none', 'date', 'asap')),
    ADD CONSTRAINT commissions_budget_key_check
        CHECK (budget_key IS NULL OR budget_key IN ('lt100', '100_200', '200_400', 'gt400', 'discuss')),
    ADD CONSTRAINT commissions_budget_currency_check
        CHECK (budget_currency IS NULL OR budget_currency IN ('USD', 'EUR'));

-- Фотографии, которые гость загрузил до отправки заявки. Фото, которое никто не
-- прикрепил к заявке (`claimed_at IS NULL`), суточная уборка стирает через сутки.
CREATE TABLE commission_uploads (
    url        TEXT PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    claimed_at TIMESTAMPTZ
);

CREATE INDEX commission_uploads_unclaimed_idx
    ON commission_uploads (created_at) WHERE claimed_at IS NULL;
