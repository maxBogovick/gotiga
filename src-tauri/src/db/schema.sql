-- Таблица фигур
CREATE TABLE IF NOT EXISTS figurines (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    short_text TEXT,
    full_description TEXT,
    dimensions TEXT,
    material TEXT,
    technique TEXT,
    year INTEGER,
    passport_number TEXT,
    edition TEXT,
    created_period TEXT,
    care_instructions TEXT,
    provenance_note TEXT,
    authenticity_note TEXT,
    included_items TEXT,
    ambience_path TEXT,
    video_url TEXT,
    ambience_data BLOB, -- Встроенное аудио
    video_data BLOB,    -- Встроенное видео
    secret_text TEXT,
    is_visible BOOLEAN NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'available'
        CHECK (status IN ('available', 'sold', 'reserved', 'in_progress')),
    open_from_min INTEGER,  -- "Дом просыпается": окно показа, минуты от полуночи (NULL = всегда открыто)
    open_until_min INTEGER, -- конец окна; until < from = окно через полночь (ночной зал)
    sealed_door_image TEXT, -- URL картины двери; NULL = резная дверь рисуется кодом
    showing_room_id TEXT,   -- зал показа (FK showing_rooms.id); NULL = своё окно
    display_layout TEXT,    -- раскладка страницы: specimen|showcase|codex|diptych|broadside; NULL = specimen
    display_config TEXT,    -- JSON: {background,blockOrder} для кастомизации витрины
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Индекс для сортировки
CREATE INDEX IF NOT EXISTS idx_figurines_sort
    ON figurines(sort_order);

-- Таблица изображений
CREATE TABLE IF NOT EXISTS images (
    id TEXT PRIMARY KEY,
    figurine_id TEXT NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    image_type TEXT NOT NULL
        CHECK (image_type IN ('face', 'detail', 'full')),
    file_path TEXT NOT NULL,
    original_path TEXT,
    thumb_path TEXT,
    depth_path TEXT, -- Карта глубины для 2.5D-параллакса (LivingDaguerreotype)
    parallax_intensity REAL, -- NULL = дефолт renderer-а
    focal_x REAL, -- "Замочная скважина": фокус-точка превью (0..1), NULL = центр
    focal_y REAL,
    reveal_radius REAL, -- радиус видимой области (0..1 кадра), NULL = дефолт
    darkness REAL, -- глубина затемнения (0..1), NULL = глобальная настройка темы
    data BLOB, -- Встроенное изображение
    original_data BLOB, -- Встроенный оригинал
    thumb_data BLOB, -- Встроенный thumbnail
    alt_text TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Индексы для изображений
CREATE INDEX IF NOT EXISTS idx_images_figurine
    ON images(figurine_id);
CREATE INDEX IF NOT EXISTS idx_images_sort
    ON images(figurine_id, sort_order);

-- Таблица текстов (автор, мастерская)
CREATE TABLE IF NOT EXISTS texts (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL
        CHECK (category IN ('author', 'workshop')),
    content TEXT NOT NULL,
    caption TEXT,
    image_path TEXT,
    image_data BLOB, -- Встроенное изображение мастерской
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Индекс для текстов
CREATE INDEX IF NOT EXISTS idx_texts_category
    ON texts(category, sort_order);

-- Залы показа: общее окно, на которое ссылается группа фигурок ("Дом просыпается")
CREATE TABLE IF NOT EXISTS showing_rooms (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    open_from_min INTEGER NOT NULL,
    open_until_min INTEGER NOT NULL,
    open_days_mask INTEGER, -- биты Пн..Вс; NULL = каждый день
    open_month_day TEXT,    -- "MM-DD": ежегодно в эту дату
    open_date_from TEXT,    -- "YYYY-MM-DD": разовый интервал (начало)
    open_date_until TEXT,   -- "YYYY-MM-DD": разовый интервал (конец)
    sort_order INTEGER NOT NULL DEFAULT 0
);

-- Таблица зон кабинета (интерактивные области)
CREATE TABLE IF NOT EXISTS cabinet_zones (
    id TEXT PRIMARY KEY,
    zone_type TEXT NOT NULL
        CHECK (zone_type IN ('showcase', 'desk', 'shelf', 'note')),
    x_percent REAL NOT NULL,
    y_percent REAL NOT NULL,
    width_percent REAL NOT NULL,
    height_percent REAL NOT NULL,
    target_route TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
);

-- Таблица этапов создания (Гримуар)
CREATE TABLE IF NOT EXISTS process_steps (
    id TEXT PRIMARY KEY,
    figurine_id TEXT NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    step_type TEXT NOT NULL
        CHECK (step_type IN ('sketch', 'prototype', 'modeling', 'painting', 'finish')),
    description TEXT,
    image_path TEXT NOT NULL,
    image_data BLOB, -- Встроенное изображение этапа
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Индекс для этапов
CREATE INDEX IF NOT EXISTS idx_process_steps_figurine
    ON process_steps(figurine_id, sort_order);

-- Таблица общих ресурсов приложения (фон, логотипы и т.д.)
CREATE TABLE IF NOT EXISTS app_resources (
    key TEXT PRIMARY KEY,
    file_path TEXT NOT NULL,
    data BLOB, -- Встроенное изображение
    updated_at TEXT DEFAULT (datetime('now'))
);
