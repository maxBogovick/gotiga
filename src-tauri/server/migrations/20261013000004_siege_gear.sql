-- Снаряжение штурма: стена, сторожевая башня, таран (BATTLE-GENRE-RESEARCH.md
-- § 7). Карты «только для этюдов» без работы автора: их ставит в город
-- кнопка «Сделать городом» на столе поля, и потому они обязаны быть на
-- каждом сервере, а не только там, где их завели руками.
--
-- Картинки встроены в сервер и кладутся на склад при запуске
-- (`siege::lay_gear_art`) под теми же именами, что здесь.
--
-- ON CONFLICT DO NOTHING: где снаряжение уже заведено руками (с этими же
-- слугами), оно остаётся как есть — правленное хранителем не затирается.
INSERT INTO battle_cards (
    slug, status, kind, tier, type_en, type_ru, title_en, title_ru, effect_en, effect_ru,
    cost, power, health, armor, attack_channel, reach, step, shoots, art_url
) VALUES
    ('fortress-wall', 'etude', 'structure', 2, 'Structure', 'Строение',
     'Fortress wall', 'Крепостная стена',
     'Does not move or strike. Holds the way while it stands.',
     'Не ходит и не бьёт. Держит проход, пока цела.',
     1, 0, 8, 3, 'none', 1, 0, FALSE, '/static/frames/siege-fortress-wall.webp'),
    ('watchtower', 'etude', 'structure', 2, 'Structure', 'Строение',
     'Watchtower', 'Сторожевая башня',
     'Does not move. Shoots up to three cells away, over walls.',
     'Не ходит. Стреляет на три клетки, через стены.',
     1, 2, 8, 0, 'physical', 3, 0, TRUE, '/static/frames/siege-watchtower.webp'),
    ('siege-ram', 'etude', 'unit', 2, 'Siege', 'Осада',
     'Battering ram', 'Таран',
     'Strikes past armour.', 'Бьёт мимо брони.',
     2, 4, 9, 0, 'pure', 1, 1, FALSE, '/static/frames/siege-ram.webp')
ON CONFLICT (slug) DO NOTHING;
