-- Первый маршрут забега: заготовка из шести уже опубликованных боёв.
--
-- Враги и поле у каждой волны — те, что у боя-основы, в порядке полки этюдов.
-- Это ЧЕРНОВИК: подкрепления и условий победы у волн нет, порядок по
-- сложности не проверен. Автор открывает вкладку «Маршруты», правит волны,
-- запускает пробу ботами и только потом публикует (и включает режим на
-- вкладке настроек забега). Нет опубликованных боёв — маршрут не создаётся.

INSERT INTO battle_routes (slug, title_en, title_ru, title_uk, title_es, title_de, title_it, guest_waves, status)
SELECT 'first-run', 'The first run', 'Первый забег', 'Перший забіг',
       'La primera carrera', 'Der erste Durchlauf', 'La prima corsa', 2, 'draft'
 WHERE EXISTS (SELECT 1 FROM battle_challenges WHERE status = 'published')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO battle_route_waves (route_id, wave_no, challenge_id, min_cards, max_cards, reward_dust)
SELECT r.id,
       c.n::smallint,
       c.id,
       1,
       CASE WHEN c.n <= 2 THEN 3 WHEN c.n <= 4 THEN 4 ELSE 5 END,
       10 + 5 * (c.n::int - 1)
  FROM battle_routes r
  JOIN (SELECT id, ROW_NUMBER() OVER (ORDER BY sort_order NULLS LAST, created_at) AS n
          FROM battle_challenges WHERE status = 'published') c ON c.n <= 6
 WHERE r.slug = 'first-run'
   AND NOT EXISTS (SELECT 1 FROM battle_route_waves w WHERE w.route_id = r.id)
ON CONFLICT DO NOTHING;
