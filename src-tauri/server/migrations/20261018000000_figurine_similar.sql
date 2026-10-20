-- Похожие работы: до трёх на каждую, в порядке, который выбрал автор. Их
-- показывает док на главной (три миниатюры), а выбирают в админке, во вкладке
-- «Реестр», в форме работы.
--
-- Отдельная таблица, а не три колонки и не массив: внешние ключи убирают строку
-- вместе с любой из двух работ (удалили работу — она пропала и из чужих
-- «похожих»), а `position` держит порядок. Похожесть не симметрична: «Ведьма А»
-- может указывать на «Ведьму Б», а та на другую.
CREATE TABLE IF NOT EXISTS figurine_similar (
    figurine_id UUID     NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    similar_id  UUID     NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    position    SMALLINT NOT NULL CHECK (position BETWEEN 0 AND 2),
    PRIMARY KEY (figurine_id, position),
    UNIQUE (figurine_id, similar_id),
    CHECK (figurine_id <> similar_id)
);

-- Удаление и чтение «кто указывает на эту работу» идут по второму столбцу.
CREATE INDEX IF NOT EXISTS idx_figurine_similar_target ON figurine_similar (similar_id);
