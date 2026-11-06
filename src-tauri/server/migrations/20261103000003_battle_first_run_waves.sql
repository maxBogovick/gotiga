-- Подкрепления для волн первого маршрута, подобранные пробой ботами.
--
-- Проба (5 начал; слабейший / средний / сильнейший отряд): слабейший проходит
-- одну волну из шести, средний (четыре сильнейшие карты рангов 1–2) — две,
-- сильнейший выигрывает 3 из 5. Проба не даёт наград между волнами (лечение,
-- замена), поэтому в настоящем забеге средний отряд уйдёт дальше. Цель «3–4
-- волны» проверяется игрой, см. раздел 7 в TASKS-BATTLE-RUN.md.
--
-- Волна правится, только если на ней стоит ожидаемый бой-основа, у неё ещё нет
-- подкреплений (автор их не трогал) и все карты расписания есть на полке.
-- Иначе волна остаётся как есть и её правят в админке.

UPDATE battle_route_waves w
   SET reinforcements = '[{"round":3,"x":1,"y":0,"card":"miner"}]'
  FROM battle_routes r, battle_challenges c
 WHERE r.id = w.route_id AND r.slug = 'first-run'
   AND c.id = w.challenge_id AND c.slug = 'blessing'
   AND w.wave_no = 2
   AND w.reinforcements = '[]'
   AND (SELECT COUNT(*) FROM battle_cards WHERE slug IN ('miner')) = 1;

UPDATE battle_route_waves w
   SET reinforcements = '[{"round":2,"x":0,"y":0,"card":"hunchback"},{"round":4,"x":2,"y":0,"card":"gnome-axeman"}]'
  FROM battle_routes r, battle_challenges c
 WHERE r.id = w.route_id AND r.slug = 'first-run'
   AND c.id = w.challenge_id AND c.slug = 'ravines'
   AND w.wave_no = 3
   AND w.reinforcements = '[]'
   AND (SELECT COUNT(*) FROM battle_cards WHERE slug IN ('gnome-axeman','hunchback')) = 2;

UPDATE battle_route_waves w
   SET reinforcements = '[{"round":2,"x":2,"y":0,"card":"hunchback"},{"round":4,"x":1,"y":0,"card":"dark-vampire"}]'
  FROM battle_routes r, battle_challenges c
 WHERE r.id = w.route_id AND r.slug = 'first-run'
   AND c.id = w.challenge_id AND c.slug = 'high-ground'
   AND w.wave_no = 4
   AND w.reinforcements = '[]'
   AND (SELECT COUNT(*) FROM battle_cards WHERE slug IN ('dark-vampire','hunchback')) = 2;

UPDATE battle_route_waves w
   SET reinforcements = '[{"round":2,"x":1,"y":0,"card":"gnome-axeman"},{"round":3,"x":0,"y":0,"card":"red-vampire"},{"round":5,"x":2,"y":0,"card":"forest-gnome"}]'
  FROM battle_routes r, battle_challenges c
 WHERE r.id = w.route_id AND r.slug = 'first-run'
   AND c.id = w.challenge_id AND c.slug = 'mire'
   AND w.wave_no = 5
   AND w.reinforcements = '[]'
   AND (SELECT COUNT(*) FROM battle_cards WHERE slug IN ('forest-gnome','gnome-axeman','red-vampire')) = 3;

UPDATE battle_route_waves w
   SET reinforcements = '[{"round":2,"x":2,"y":0,"card":"red-vampire"},{"round":3,"x":0,"y":0,"card":"forest-gnome"},{"round":4,"x":2,"y":0,"card":"forest-gnome"}]'
  FROM battle_routes r, battle_challenges c
 WHERE r.id = w.route_id AND r.slug = 'first-run'
   AND c.id = w.challenge_id AND c.slug = 'the-healer'
   AND w.wave_no = 6
   AND w.reinforcements = '[]'
   AND (SELECT COUNT(*) FROM battle_cards WHERE slug IN ('forest-gnome','red-vampire')) = 2;
