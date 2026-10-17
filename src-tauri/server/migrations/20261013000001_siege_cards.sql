-- Штурм города (BATTLE-GENRE-RESEARCH.md § 7.5).
--
-- 'structure' — вид карты для стен и башен: сторона, у которой остались одни
-- строения, побеждена. Это вид, а не статус, потому что о карте он говорит,
-- ЧТО она такое в бою, — тем же полем, что 'unit'.
--
-- 'etude' — статус «только для этюдов»: карта встаёт в этюд, но не стоит на
-- полке и не продаётся. Стена, башня и таран — снаряжение этюда, а не работа,
-- которую можно взять. Это статус, а не вид, потому что о карте он говорит,
-- ГДЕ она бывает, — тем же полем, что 'published'.
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_kind_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_kind_check
    CHECK (kind IN ('unit', 'spell', 'relic', 'structure'));

ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_status_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_status_check
    CHECK (status IN ('draft', 'published', 'retired', 'etude'));
