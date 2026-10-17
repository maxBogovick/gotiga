-- Расстановка гостя на поле одной встречи.
--
-- Прежде гость собирал один стол на половине 3 × 3, и сервер подгонял его к
-- полю встречи (`deck_cell_on`). Встречи бывают шириной 3–4 и глубиной 3–5, с
-- местностью и строениями, поэтому карты вставали не там, где человек их
-- видел, а стена под подогнанной картой молча пропадала. Теперь карты
-- расставляют прямо перед боем, на поле этой встречи, и расстановка
-- запоминается для неё: повторная партия открывается с прежней.
--
-- Форма `layout` та же, что у `battle_decks` (`DeckLayout`), но клетки — это
-- клетки поля встречи. Состав схемой не проверяется по той же причине, что и у
-- стола: карту могли снять с полки после сохранения, и законность проверяется
-- в момент начала партии.
CREATE TABLE IF NOT EXISTS battle_lineups (
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    challenge_id UUID NOT NULL REFERENCES battle_challenges(id) ON DELETE CASCADE,
    layout       TEXT NOT NULL CHECK (char_length(layout) <= 4000),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, challenge_id)
);
