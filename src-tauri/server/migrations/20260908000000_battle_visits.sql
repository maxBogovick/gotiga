-- Приход в комнату.
--
-- Своя таблица нужна ровно потому, что приход НИГДЕ не записан: книга кошелька
-- знает только то, за что заплатили, и «человек заходил вчера» из неё не
-- выводится.
--
-- День — по часам ДОМА (`battle_day_offset`), а не по UTC: иначе поручение
-- «зашли сегодня» обновлялось бы у москвича в три часа ночи. Первичный ключ по
-- (гость, день) и есть вся защита от двойной записи.
--
-- `visited_at` отдельно от `day` не для красоты: `day` дедуплицирует, а считают
-- по `visited_at`, и тогда счёт внутри окна — то же сравнение времени, что у
-- всех остальных условий, а не особый случай с приведением дат.
CREATE TABLE IF NOT EXISTS battle_visits (
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    day        DATE NOT NULL,
    visited_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, day)
);

CREATE INDEX IF NOT EXISTS battle_visits_user_idx
    ON battle_visits (user_id, visited_at DESC);

-- Поручение за приход.
--
-- Две пыли, а не десять: приход сам по себе не внимание, и повторяющаяся
-- награда, сравнимая с разовой, обесценит тропу за неделю
-- (`BATTLE-ERRANDS.md` §5).
--
-- СЕРИИ У НЕГО НЕТ И НЕ БУДЕТ. Ни «седьмой день подряд», ни потери
-- накопленного при пропуске: обязанностью игру делает именно серия, а не
-- повторение. Пропустили неделю — ничего не потеряли, зашли — получили две
-- пыли, как и в любой другой день.
INSERT INTO battle_errands
    (slug, title_en, title_ru, note_en, note_ru, rule, threshold, currency, amount,
     period, status, sort_order)
VALUES
    ('came-by', 'Come by', 'Зайти в комнату',
     'For looking in today. Miss a day and nothing is lost.',
     'За то, что заглянули сегодня. Пропустите день — ничего не потеряете.',
     'visits', 1, 'dust', 2, 'daily', 'published', 5)
ON CONFLICT (slug) DO NOTHING;
