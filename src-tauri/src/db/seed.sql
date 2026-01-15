-- ============================================================
-- ТЕСТОВЫЕ ДАННЫЕ С РЕАЛЬНЫМИ ССЫЛКАМИ (UNSPLASH)
-- ============================================================

-- Очистка
DELETE FROM images;
DELETE FROM texts;
DELETE FROM figurines;
DELETE FROM cabinet_zones;

-- ФИГУРЫ
INSERT INTO figurines (id, name, short_text, year, status, sort_order) VALUES
('fig-001', 'Хранительница порога', 'Она стоит там, где заканчивается один дом и начинается другой.', 2023, 'available', 1),
('fig-002', 'Тот, кто слушает стены', 'В каждом доме есть тот, кто помнит все разговоры.', 2024, 'available', 2),
('fig-003', 'Собирательница пыли', 'Она никогда не подметала следы. Дом сам их забывал.', 2023, 'sold', 3),
('fig-004', 'Молчаливый гость', 'Он пришёл с севера, но никогда не говорил откуда именно.', 2024, 'available', 4);

-- ИЗОБРАЖЕНИЯ (Unsplash скульптуры/куклы)
INSERT INTO images (id, figurine_id, image_type, file_path, alt_text, sort_order) VALUES
-- fig-001
('img-001-face', 'fig-001', 'face', 'https://i.etsystatic.com/16575799/c/1014/1014/359/220/il/a0a89d/7113423980/il_300x300.7113423980_44y2.jpg', 'Лицо Хранительницы', 1),
('img-001-full', 'fig-001', 'full', 'https://i.etsystatic.com/16575799/r/il/7c76cc/7569598025/il_300x300.7569598025_7odf.jpg', 'Хранительница в полный рост', 2),
-- fig-002
('img-002-face', 'fig-002', 'face', 'https://i.etsystatic.com/16575799/c/814/814/32/38/il/769fcb/7391813374/il_300x300.7391813374_1veu.jpg', 'Лицо Слушающего', 1),
-- fig-003
('img-003-face', 'fig-003', 'face', 'https://i.etsystatic.com/16575799/r/il/015d6e/7549759072/il_300x300.7549759072_cw29.jpg', 'Лицо Собирательницы', 1),
-- fig-004
('img-004-face', 'fig-004', 'face', 'https://i.etsystatic.com/16575799/r/il/f95d87/6950951720/il_300x300.6950951720_9sk2.jpg', 'Лицо Гостя', 1);

-- АВТОРСКИЕ ТЕКСТЫ
INSERT INTO texts (id, category, content, sort_order) VALUES
('author-001', 'author', 'Я оставляю трещины. Они говорят больше, чем гладкая поверхность.', 1),
('author-002', 'author', 'Каждая фигура помнит руки, которые её создали.', 2);

-- КОНТЕНТ МАСТЕРСКОЙ (Unsplash мастерская/инструменты)
INSERT INTO texts (id, category, content, caption, image_path, sort_order) VALUES
('workshop-001', 'workshop', 'Начало', 'Первые наброски. Лицо ещё не решило, кем быть.', 'http://localhost:1420/images/workshop/master-1.jpg', 1),
('workshop-002', 'workshop', 'Форма', 'Глина помнит каждое прикосновение.', 'https://images.unsplash.com/photo-1565191999001-551c187427bb?q=80&w=1000&auto=format&fit=crop', 2);

-- ЗОНЫ КАБИНЕТА
INSERT INTO cabinet_zones (id, zone_type, x_percent, y_percent, width_percent, height_percent, target_route, sort_order) VALUES
('zone-showcase', 'showcase', 15, 20, 30, 50, '/figurines', 1),
('zone-desk', 'desk', 35, 65, 35, 30, '/workshop', 2),
('zone-note', 'note', 75, 70, 12, 12, '/author', 4);