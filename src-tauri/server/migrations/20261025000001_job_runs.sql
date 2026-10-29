-- Журнал фоновых проходов: когда каждый шёл в последний раз и чем кончился.
--
-- Проходов больше десятка (объявления в канале, письма, поиск, уборка, разбор недели), и их
-- сбой был виден только в журнале сервера, который никто не читает. Строка на проход, а не
-- на запуск: растёт число проходов, а не таблица. Пишется в конце каждого запуска
-- (`services/jobs.rs`), поэтому зависший проход виден тем, что `started_at` стареет.
CREATE TABLE IF NOT EXISTS job_runs (
    job          TEXT        PRIMARY KEY,
    -- Как часто проход должен идти, секунды; 0 — один раз при запуске сервера.
    every_secs   INTEGER     NOT NULL DEFAULT 0,
    started_at   TIMESTAMPTZ NOT NULL,
    finished_at  TIMESTAMPTZ NOT NULL,
    duration_ms  INTEGER     NOT NULL DEFAULT 0,
    ok           BOOLEAN     NOT NULL,
    -- Последний удачный запуск и последний раз, когда проход что-то сделал (не вхолостую).
    ok_at        TIMESTAMPTZ,
    did_at       TIMESTAMPTZ,
    -- Что сделал в последний раз, когда делал; пустой запуск её не стирает.
    note         TEXT,
    error        TEXT,
    runs         BIGINT      NOT NULL DEFAULT 0,
    failures     BIGINT      NOT NULL DEFAULT 0,
    in_a_row     INTEGER     NOT NULL DEFAULT 0
);
