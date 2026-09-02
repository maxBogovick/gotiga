-- Названия заданий — обычными словами.
--
-- Засеянные тексты были написаны голосом сайта: «оглядеться», «прочесть
-- небылицу», «довести этюд до конца», «разложить стол». Игрок, впервые попавший
-- в игру, из них не понимает ни что от него требуется, ни куда идти. Голос
-- сайта остаётся на сайте; интерфейс игры говорит прямо.
--
-- Обновляется ТОЛЬКО то, что ещё в точности совпадает с засеянным текстом:
-- задание, которое хранитель уже переписал по-своему, эта миграция не трогает.
UPDATE battle_errands SET title_ru = 'Посмотреть работы', title_en = 'View some works'
 WHERE slug = 'look-around' AND title_ru = 'Оглядеться';

UPDATE battle_errands
   SET title_ru = 'Отметить понравившуюся работу', title_en = 'Like a work',
       note_ru = 'Отметьте сердечком работу, которая понравилась.',
       note_en = 'Put a heart on a work you liked.'
 WHERE slug = 'closer-look' AND title_ru = 'Приглядеться';

UPDATE battle_errands
   SET title_ru = 'Прочитать историю', title_en = 'Read a story',
       note_ru = 'Дочитайте одну историю до конца.',
       note_en = 'Read one story to the end.'
 WHERE slug = 'read-a-tale' AND title_ru = 'Прочесть небылицу';

UPDATE battle_errands
   SET title_ru = 'Оставить отзыв о работе', title_en = 'Leave a review',
       note_ru = 'Напишите, что вы думаете о работе. Засчитается, когда автор его одобрит.',
       note_en = 'Write what you think of a work. Counts once the author approves it.'
 WHERE slug = 'leave-impression' AND title_ru = 'Оставить впечатление';

UPDATE battle_errands
   SET title_ru = 'Собрать колоду', title_en = 'Assemble a deck',
       note_ru = 'Шесть карт: три на поле, три в руке.',
       note_en = 'Six cards: three on the field, three in hand.'
 WHERE slug = 'lay-the-table' AND title_ru = 'Разложить стол';

UPDATE battle_errands
   SET title_ru = 'Доиграть бой до конца', title_en = 'Finish a battle',
       note_ru = 'Победа или поражение — важно доиграть до конца.',
       note_en = 'Win or lose, what matters is finishing.'
 WHERE slug = 'finish-a-study' AND title_ru = 'Довести этюд до конца';

UPDATE battle_errands
   SET title_ru = 'Повысить уровень карты', title_en = 'Raise a card level',
       note_ru = 'Уровень не влияет на бой — это отметка о том, что карта давно у вас.',
       note_en = 'A level does not affect battles. It marks how long you have had the card.'
 WHERE slug = 'raise-a-card' AND title_ru = 'Поднять карту на ступень';

UPDATE battle_errands SET title_ru = 'Зайти в игру', title_en = 'Come back'
 WHERE slug = 'came-by' AND title_ru = 'Зайти в комнату';

UPDATE battle_errands
   SET title_ru = 'Оставить отзыв, который понравится автору',
       title_en = 'Leave a review the author values',
       note_ru = 'Награду выдаёт автор, если сочтёт нужным.',
       note_en = 'The author decides whether to award it.'
 WHERE slug = 'an-impression' AND title_ru = 'Впечатление, которое автор оставит';

UPDATE battle_errands
   SET note_ru = 'Самая редкая награда и самая крупная.',
       note_en = 'The rarest of the three, and the largest.'
 WHERE slug = 'a-commission' AND note_ru = 'Самое редкое из трёх — и самое крупное.';
