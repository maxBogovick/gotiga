-- Владение переезжает на ЭКЗЕМПЛЯРЫ.
--
-- `battle_owned_cards (user_id, card_id)` не может выразить ни «у меня два»,
-- ни «вот этот самый, номер 7». Пока карты только покупались у дома, разницы
-- не было; с лавкой она стала правилом, которое иначе невыразимо: продал — у
-- тебя больше нет. В одной строке `card_copies` это выполняется ФОРМОЙ
-- таблицы (продажа = смена владельца), а не проверкой, которую можно забыть.
--
-- Таблица `card_copies` заведена пустой заранее (`20260918000001`) ровно ради
-- этого дня: мигрировать пустое дёшево, живое владение — риск.
--
-- Откуда взялся экземпляр, дом ЗНАЕТ, а не угадывает: за купленное в книге
-- лежит запись с ключом `buy:{карта}`. Всё остальное выдано из рук.
--
-- Номер даётся и тем, у кого тираж не ограничен: `serial` — это «седьмой
-- отпечатанный», а не «седьмой из ста». Показывать его «№7 из 100» будет
-- только карта с тиражом, но личность у экземпляра есть всегда — иначе
-- продажа одного из двух одинаковых неотличима от продажи другого.
WITH numbered AS (
    SELECT o.*,
           ROW_NUMBER() OVER (PARTITION BY o.card_id ORDER BY o.acquired_at, o.id) AS n
      FROM battle_owned_cards o
)
INSERT INTO card_copies (card_id, owner_id, serial, level, origin, acquired_at, seen_at)
SELECT n.card_id,
       n.user_id,
       n.n,
       n.level,
       CASE
           WHEN EXISTS (
               SELECT 1 FROM battle_wallet_entries w
                WHERE w.user_id = n.user_id
                  AND w.idem_key = 'buy:' || n.card_id::text
           ) THEN 'bought'
           ELSE 'gift'
       END,
       n.acquired_at,
       n.seen_at
  FROM numbered n;

-- Счётчик отпечатанного догоняет то, что уже роздано: следующий покупатель
-- обязан получить СЛЕДУЮЩИЙ номер, а не начать нумерацию заново.
UPDATE battle_cards c
   SET minted = s.n
  FROM (SELECT card_id, COUNT(*)::int AS n FROM card_copies GROUP BY card_id) s
 WHERE s.card_id = c.id;

-- Старая таблица уходит целиком. Оставить её «на всякий случай» значило бы
-- завести второй учёт владения, который разойдётся с первым на первой же
-- продаже.
DROP TABLE IF EXISTS battle_owned_cards;
