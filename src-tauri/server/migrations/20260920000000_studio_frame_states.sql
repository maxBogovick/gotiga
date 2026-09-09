-- Три слова вместо пяти.
--
-- Разбор — `STUDIO-REVIEW.md`. Коротко: в пяти словах были перепутаны три
-- разных вопроса — что автор может с работой сделать, что о ней решил хозяин и
-- участвует ли она в неделе. Из-за этого работа, побывавшая в сезоне,
-- застревала навсегда: `entered` никто не снимал, а править и выставлять
-- заново позволено только другим словам.
--
-- Теперь состояние — ТОЛЬКО про автора:
--   draft     — своя, правится, не видит никто
--   shown     — отдана хозяину: не правится, дальше решает он
--   withdrawn — снята; правится снова
--
-- Решение хозяина живёт отметками (`admitted_at`, `approved_at`), а участие —
-- только в `studio_entries`. Копии участия больше нет, расходиться нечему.

-- Забор снимается ПЕРВЫМ. Старое ограничение не знает слова `shown`, и
-- перевод под ним падает на первой же строке: сперва разрешить новое, потом
-- переводить, потом запретить старое.
ALTER TABLE studio_frames DROP CONSTRAINT IF EXISTS studio_frames_status_check;

-- Старые слова: `published`, `entered` и `approved` значат одно — «отдана
-- хозяину». Что она допущена и что взята в игру, говорят отметки.
UPDATE studio_frames SET status = 'shown'
 WHERE status IN ('published', 'entered', 'approved');

ALTER TABLE studio_frames ADD CONSTRAINT studio_frames_status_check
    CHECK (status IN ('draft', 'shown', 'withdrawn'));

DROP INDEX IF EXISTS studio_frames_published_idx;
DROP INDEX IF EXISTS studio_frames_admission_idx;
-- Очередь допуска: отдано и хозяин ещё не смотрел.
CREATE INDEX IF NOT EXISTS studio_frames_admission_idx
    ON studio_frames (updated_at) WHERE status = 'shown' AND admitted_at IS NULL;
