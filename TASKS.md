# Living Cabinet of Curiosities — Задачи на разработку

## Философия проекта (ПРОЧИТАТЬ ОБЯЗАТЕЛЬНО)

**Это НЕ software. Это сохранённое присутствие художника.**

Ключевые слова эстетики: камерность, тишина, пыль, ткань, старый дом.
**Это не готика и не хоррор. Это интимная странность.**

### Главное правило
Если функция:
- разрушает тишину
- ускоряет восприятие
- делает проект похожим на магазин

➡️ **она не входит в MVP**.

### Критерии успеха MVP
- Пользователь проводит >3 минут
- Не возникает желания "пролистать"
- Фигурки запоминаются

---

## Технологический стек

| Слой | Технология | Примечание |
|------|------------|------------|
| Desktop shell | Tauri 2.x | минимальный размер, Rust-backend |
| Backend | Rust + tokio | Clean Architecture |
| База данных | SQLite (rusqlite) | локальная, в папке приложения |
| Frontend | SvelteKit (SPA) | static adapter для Tauri |
| Стили | Tailwind CSS | кастомная тема проекта |
| Сериализация | serde | JSON для IPC |

---

## Задача 1: Инициализация проекта Tauri + SvelteKit

### Контекст
Проект начинается с нуля. Создаём desktop-приложение с Tauri 2.x и SvelteKit.

### Цель
Рабочий каркас приложения, который запускается и показывает пустую страницу.

### Пошаговая инструкция

#### Шаг 1.1: Структура директорий
Убедиться, что структура соответствует:
```
gotiga/
├── src/                    # SvelteKit frontend
├── src-tauri/              # Rust backend
│   ├── capabilities/       # <--- ВАЖНО: Папка для прав доступа
│   │   └── default.json
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── db/
│   │   ├── models/
│   │   └── commands/
│   ├── Cargo.toml
│   └── tauri.conf.json
├── static/                 # статические файлы
│   ├── fonts/              # <--- ВАЖНО: Папка для шрифтов
│   └── images/
├── data/                   # данные приложения (БД, контент)
```

#### Шаг 1.3: Настройка прав доступа (Capabilities)
Критично для Tauri 2.0. Создать файл src-tauri/capabilities/default.json:
Файл `svelte.config.js`:
```json
{
   "$schema": "../gen/schemas/desktop-schema.json",
   "identifier": "default",
   "description": "Capability for the main window",
   "windows": ["main"],
   "permissions": [
      "core:default",
      "fs:default",
      "fs:allow-app-write-recursive",
      "fs:allow-resource-read-recursive",
      "shell:open"
   ]
}
```

#### Шаг 1.4: Настройка tauri.conf.json
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Cabinet of Curiosities",
  "version": "0.1.0",
  "identifier": "com.cabinet.curiosities",
  "build": {
    "frontendDist": "../build"
  },
  "app": {
    "windows": [
      {
        "title": "Cabinet of Curiosities",
        "width": 1200,
        "height": 800,
        "minWidth": 800,
        "minHeight": 600,
        "resizable": true,
        "fullscreen": false,
        "decorations": true,
        "center": true
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

#### Шаг 1.5: Зависимости Rust (Cargo.toml)
```toml
[package]
name = "gotiga"
version = "0.1.0"
edition = "2021"

[lib]
name = "cabinet_lib"
crate-type = ["lib", "cdylib", "staticlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
rusqlite = { version = "0.31", features = ["bundled"] }
uuid = { version = "1", features = ["v4"] }
thiserror = "1"
```

#### Шаг 1.6: Проверка запуска
```bash
npm install
cargo tauri dev
```

### Критерии готовности
- [ ] `cargo tauri dev` запускается без ошибок
- [ ] Открывается окно с заголовком "Cabinet of Curiosities"
- [ ] Размер окна 1200x800
- [ ] Минимальный размер 800x600 работает
- [ ] Hot reload при изменении Svelte-файлов работает

### Возможные ошибки и решения
| Ошибка | Решение |
|--------|---------|
| `failed to bundle project` | Проверить adapter-static установлен |
| `window not found` | Проверить tauri.conf.json синтаксис |
| `rusqlite build failed` | Добавить feature "bundled" |

---

## Задача 2: Настройка Tailwind CSS с темой проекта

### Контекст
Tailwind CSS уже установлен в проекте. Нужно настроить кастомную тему, соответствующую эстетике проекта.

### Цель
Полностью настроенная тема Tailwind с цветами, шрифтами и утилитами проекта.

### ВАЖНО: Эстетические требования
- **Белый цвет НЕ используется НИКОГДА**
- **Bold/жирный текст НЕ используется НИКОГДА**
- Все цвета — тёплые, приглушённые
- Всё медленное, плавное

### Пошаговая инструкция

#### Шаг 2.2: Конфигурация tailwind.config.js
```javascript
/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    // Полностью переопределяем цвета (НЕ extend!)
    colors: {
      // Основные цвета проекта
      'cabinet': {
        'bg': '#2E2B28',           // тёмный тёплый фон
        'wood': '#5A524C',         // дерево
        'fabric': '#8C7E73',       // состаренная ткань
        'bone': '#CFC6B8',         // кость / кожа (основной текст)
        'dust': '#A39B91',         // пыль (вторичный текст)
      },
      // Акценты (использовать ОЧЕНЬ редко)
      'accent': {
        'red': '#7A2E2E',          // выцветший бордовый
        'olive': '#6A705F',        // пыльно-оливковый
      },
      // Служебные
      'transparent': 'transparent',
      'current': 'currentColor',
      // Чёрный для теней
      'black': '#000000',
    },

    // Шрифты
    fontFamily: {
      'display': ['"Cormorant Garamond"', 'serif'],  // заголовки, имена
      'body': ['"Source Serif 4"', 'serif'],         // основной текст
    },

    // ВАЖНО: Убираем все font-weight кроме normal
    fontWeight: {
      'normal': '400',
    },

    // Размеры текста
    fontSize: {
      'xs': ['0.75rem', { lineHeight: '1.5' }],
      'sm': ['0.875rem', { lineHeight: '1.6' }],
      'base': ['1rem', { lineHeight: '1.8' }],
      'lg': ['1.25rem', { lineHeight: '1.6' }],
      'xl': ['1.5rem', { lineHeight: '1.4' }],
      '2xl': ['2rem', { lineHeight: '1.3' }],
      '3xl': ['2.5rem', { lineHeight: '1.2' }],
      '4xl': ['3rem', { lineHeight: '1.1' }],
    },

    // Расширяем стандартные значения
    extend: {
      // Анимации (всё медленное!)
      transitionDuration: {
        '400': '400ms',
        '600': '600ms',
        '800': '800ms',
        '1000': '1000ms',
      },

      // Timing functions
      transitionTimingFunction: {
        'cabinet': 'cubic-bezier(0.25, 0.1, 0.25, 1)',
      },

      // Тени (мягкие, тёплые)
      boxShadow: {
        'cabinet': '0 4px 20px rgba(0, 0, 0, 0.3)',
        'cabinet-lg': '0 8px 40px rgba(0, 0, 0, 0.4)',
        'cabinet-inner': 'inset 0 2px 10px rgba(0, 0, 0, 0.2)',
      },

      // Прозрачности для overlay
      opacity: {
        '15': '0.15',
        '85': '0.85',
      },

      // Размытие для эффектов
      backdropBlur: {
        'xs': '2px',
      },

      // Анимации
      animation: {
        'fade-in': 'fadeIn 500ms cubic-bezier(0.25, 0.1, 0.25, 1) forwards',
        'fade-in-slow': 'fadeIn 800ms cubic-bezier(0.25, 0.1, 0.25, 1) forwards',
        'scale-in': 'scaleIn 500ms cubic-bezier(0.25, 0.1, 0.25, 1) forwards',
        'slide-up': 'slideUp 600ms cubic-bezier(0.25, 0.1, 0.25, 1) forwards',
      },

      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        scaleIn: {
          '0%': { opacity: '0', transform: 'scale(0.98)' },
          '100%': { opacity: '1', transform: 'scale(1)' },
        },
        slideUp: {
          '0%': { opacity: '0', transform: 'translateY(10px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
      },

      // Поворот для "разбросанных" элементов
      rotate: {
        '1': '1deg',
        '2': '2deg',
        '3': '3deg',
        '-1': '-1deg',
        '-2': '-2deg',
        '-3': '-3deg',
      },
    },
  },
  plugins: [],
}
```

#### Шаг 2.3: Глобальные стили src/app.css
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

/* Подключение шрифтов Google */
@import url('https://fonts.googleapis.com/css2?family=Cormorant+Garamond:wght@400&family=Source+Serif+4:wght@400&display=swap');

@layer base {
  /* Глобальный сброс */
  * {
    @apply font-normal; /* Принудительно убираем bold везде */
  }

  html {
    @apply bg-cabinet-bg text-cabinet-bone antialiased;
    font-feature-settings: "liga" 1, "kern" 1;
  }

  body {
    @apply font-body text-base min-h-screen;
    /* Отключаем выделение текста для атмосферности */
    user-select: none;
  }

  /* Заголовки */
  h1, h2, h3, h4, h5, h6 {
    @apply font-display font-normal tracking-wide;
  }

  h1 { @apply text-4xl; }
  h2 { @apply text-3xl; }
  h3 { @apply text-2xl; }

  /* Ссылки */
  a {
    @apply text-cabinet-bone border-b border-transparent transition-colors duration-300;
  }

  a:hover {
    @apply border-cabinet-fabric;
  }

  /* Убираем outline при фокусе (для атмосферы) */
  *:focus {
    @apply outline-none;
  }

  /* Скроллбар в стиле проекта */
  ::-webkit-scrollbar {
    @apply w-2;
  }

  ::-webkit-scrollbar-track {
    @apply bg-cabinet-bg;
  }

  ::-webkit-scrollbar-thumb {
    @apply bg-cabinet-wood rounded-full;
  }
}

@layer components {
  /* Кнопка в стиле проекта */
  .btn-cabinet {
    @apply px-6 py-3
           border border-cabinet-fabric
           text-cabinet-bone font-display
           bg-transparent
           transition-all duration-300 ease-cabinet
           hover:bg-cabinet-wood/20;
  }

  /* Интерактивная зона (невидимая) */
  .hit-area {
    @apply absolute cursor-pointer
           transition-all duration-300 ease-cabinet
           hover:bg-cabinet-bone/5;
  }

  /* Карточка/контейнер */
  .cabinet-card {
    @apply bg-cabinet-wood/30
           shadow-cabinet
           transition-all duration-400 ease-cabinet;
  }

  /* Текст-цитата (авторский стиль) */
  .cabinet-quote {
    @apply font-display text-lg text-cabinet-dust
           italic leading-relaxed;
  }

  /* Изображение с эффектом */
  .cabinet-image {
    @apply transition-all duration-600 ease-cabinet
           hover:scale-[1.02] hover:shadow-cabinet-lg;
  }
}

@layer utilities {
  /* Задержки анимации */
  .animation-delay-100 { animation-delay: 100ms; }
  .animation-delay-200 { animation-delay: 200ms; }
  .animation-delay-300 { animation-delay: 300ms; }
  .animation-delay-400 { animation-delay: 400ms; }
  .animation-delay-500 { animation-delay: 500ms; }
}
```

#### Шаг 2.4: Глобальные стили src/app.css
```css
/* Подключение ЛОКАЛЬНЫХ шрифтов */
@font-face {
  font-family: 'Cormorant Garamond';
  src: url('/fonts/CormorantGaramond-Regular.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Source Serif 4';
  src: url('/fonts/SourceSerif4-Regular.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

@layer base {
  /* Глобальный сброс */
  * {
    @apply font-normal;
    /* Убираем стандартный курсор, наследуем кастомный */
    cursor: inherit;
  }

  html {
    @apply bg-cabinet-bg text-cabinet-bone antialiased;
    font-feature-settings: "liga" 1, "kern" 1;
    
    /* Кастомный курсор (опционально, если есть картинка) */
    /* cursor: url('/images/cursor.png'), auto; */
    
    /* Если картинки нет — просто системный, но не pointer по умолчанию */
    cursor: default;
  }

  body {
    @apply font-body text-base min-h-screen;
    user-select: none;
  }
   /* Интерактивная зона — меняем курсор */
   .hit-area {
      @apply absolute cursor-pointer;
      /* cursor: url('/images/cursor-pointer.png'), pointer; */
   }

   button, a {
      cursor: pointer;
   }
}
```
### Шпаргалка по классам Tailwind для проекта

#### Цвета фона
```
bg-cabinet-bg      — основной фон (#2E2B28)
bg-cabinet-wood    — дерево (#5A524C)
bg-cabinet-fabric  — ткань (#8C7E73)
bg-cabinet-bone    — светлый (#CFC6B8)
bg-accent-red      — РЕДКО! (#7A2E2E)
bg-accent-olive    — РЕДКО! (#6A705F)
```

#### Цвета текста
```
text-cabinet-bone  — основной текст
text-cabinet-dust  — вторичный текст
text-cabinet-fabric — приглушённый текст
```

#### Шрифты
```
font-display — Cormorant Garamond (заголовки, имена)
font-body    — Source Serif 4 (основной текст)
```

#### Анимации
```
animate-fade-in      — появление 500ms
animate-fade-in-slow — появление 800ms
animate-scale-in     — появление с масштабом
animate-slide-up     — появление снизу
```

#### Переходы
```
transition-all duration-300 ease-cabinet — стандартный
transition-all duration-600 ease-cabinet — медленный
```

### Критерии готовности
- [ ] Tailwind работает, классы применяются
- [ ] Шрифты Cormorant Garamond и Source Serif 4 загружаются
- [ ] Фон страницы цвета #2E2B28 (тёмный тёплый)
- [ ] Текст цвета #CFC6B8 (кость)
- [ ] Нигде нет белого цвета
- [ ] Нигде нет жирного текста
- [ ] Скроллбар стилизован

### ЗАПРЕЩЕНО использовать
```
bg-white, text-white, border-white  — НИКОГДА
font-bold, font-semibold, font-medium — НИКОГДА
```

---

## Задача 3: База данных SQLite — схема и инициализация

### Контекст
Приложение хранит данные локально в SQLite. БД создаётся при первом запуске.

### Цель
Создать схему БД, миграции и код инициализации на Rust.

### Пошаговая инструкция

#### Шаг 3.1: Создать модуль БД src-tauri/src/db/mod.rs
```rust
pub mod schema;
pub mod repository;

use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    /// Создать или открыть БД
    pub fn new(db_path: PathBuf) -> Result<Self> {
        // Создать директорию если не существует
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;

        // Включить foreign keys
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        // Применить миграции
        db.migrate()?;

        Ok(db)
    }

    /// Применить миграции
    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(include_str!("schema.sql"))?;

        Ok(())
    }
}
```

#### Шаг 3.2: Создать схему src-tauri/src/db/schema.sql
```sql
-- Таблица фигур
CREATE TABLE IF NOT EXISTS figurines (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    short_text TEXT,
    year INTEGER,
    status TEXT NOT NULL DEFAULT 'available'
        CHECK (status IN ('available', 'sold', 'reserved')),
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
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
    alt_text TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
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
    caption TEXT,  -- для workshop: подпись к фото
    image_path TEXT,  -- для workshop: путь к изображению
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Индекс для текстов
CREATE INDEX IF NOT EXISTS idx_texts_category
    ON texts(category, sort_order);

-- Таблица зон кабинета (интерактивные области)
CREATE TABLE IF NOT EXISTS cabinet_zones (
    id TEXT PRIMARY KEY,
    zone_type TEXT NOT NULL
        CHECK (zone_type IN ('showcase', 'desk', 'shelf', 'note')),
    x_percent REAL NOT NULL,  -- позиция X в процентах
    y_percent REAL NOT NULL,  -- позиция Y в процентах
    width_percent REAL NOT NULL,
    height_percent REAL NOT NULL,
    target_route TEXT NOT NULL,  -- куда ведёт клик
    sort_order INTEGER NOT NULL DEFAULT 0
);
```

#### Шаг 3.3: Создать репозиторий src-tauri/src/db/repository.rs
```rust
use rusqlite::{params, Connection, Result};
use crate::models::*;

pub struct Repository<'a> {
    conn: &'a Connection,
}

impl<'a> Repository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // === FIGURINES ===

    pub fn get_all_figurines(&self) -> Result<Vec<Figurine>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, short_text, year, status, sort_order
             FROM figurines
             ORDER BY sort_order"
        )?;

        let iter = stmt.query_map([], |row| {
            Ok(Figurine {
                id: row.get(0)?,
                name: row.get(1)?,
                short_text: row.get(2)?,
                year: row.get(3)?,
                status: FigurineStatus::from_str(&row.get::<_, String>(4)?),
                sort_order: row.get(5)?,
            })
        })?;

        iter.collect()
    }

    pub fn get_figurine_by_id(&self, id: &str) -> Result<Option<Figurine>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, short_text, year, status, sort_order
             FROM figurines
             WHERE id = ?"
        )?;

        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Figurine {
                id: row.get(0)?,
                name: row.get(1)?,
                short_text: row.get(2)?,
                year: row.get(3)?,
                status: FigurineStatus::from_str(&row.get::<_, String>(4)?),
                sort_order: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    // === IMAGES ===

    pub fn get_images_for_figurine(&self, figurine_id: &str) -> Result<Vec<Image>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, figurine_id, image_type, file_path, alt_text, sort_order
             FROM images
             WHERE figurine_id = ?
             ORDER BY sort_order"
        )?;

        let iter = stmt.query_map(params![figurine_id], |row| {
            Ok(Image {
                id: row.get(0)?,
                figurine_id: row.get(1)?,
                image_type: ImageType::from_str(&row.get::<_, String>(2)?),
                file_path: row.get(3)?,
                alt_text: row.get(4)?,
                sort_order: row.get(5)?,
            })
        })?;

        iter.collect()
    }

    // === TEXTS ===

    pub fn get_texts_by_category(&self, category: &str) -> Result<Vec<Text>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category, content, caption, image_path, sort_order
             FROM texts
             WHERE category = ?
             ORDER BY sort_order"
        )?;

        let iter = stmt.query_map(params![category], |row| {
            Ok(Text {
                id: row.get(0)?,
                category: TextCategory::from_str(&row.get::<_, String>(1)?),
                content: row.get(2)?,
                caption: row.get(3)?,
                image_path: row.get(4)?,
                sort_order: row.get(5)?,
            })
        })?;

        iter.collect()
    }

    // === CABINET ZONES ===

    pub fn get_cabinet_zones(&self) -> Result<Vec<CabinetZone>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, zone_type, x_percent, y_percent, width_percent, height_percent, target_route
             FROM cabinet_zones
             ORDER BY sort_order"
        )?;

        let iter = stmt.query_map([], |row| {
            Ok(CabinetZone {
                id: row.get(0)?,
                zone_type: row.get(1)?,
                x_percent: row.get(2)?,
                y_percent: row.get(3)?,
                width_percent: row.get(4)?,
                height_percent: row.get(5)?,
                target_route: row.get(6)?,
            })
        })?;

        iter.collect()
    }
}
```

#### Шаг 3.4: Интеграция в main.rs
```rust
mod db;
mod models;
mod commands;

use db::Database;
use std::path::PathBuf;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Путь к БД в директории данных приложения
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to get app data dir");

            let db_path = app_data_dir.join("cabinet.db");

            // Инициализация БД
            let db = Database::new(db_path)
                .expect("Failed to initialize database");

            // Сохранить в состоянии приложения
            app.manage(db);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_all_figurines,
            commands::get_figurine,
            commands::get_author_texts,
            commands::get_workshop_content,
            commands::get_cabinet_zones,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Критерии готовности
- [ ] БД создаётся автоматически при первом запуске
- [ ] Путь к БД: `{app_data_dir}/cabinet.db`
- [ ] Все таблицы создаются без ошибок
- [ ] Foreign keys работают (CASCADE удаление)
- [ ] Индексы созданы для оптимизации

---

## Задача 4: Rust-модели и DTO

### Контекст
Нужны типизированные структуры для работы с данными и передачи на frontend.

### Цель
Создать модели, DTO и преобразования между ними.

### Файл src-tauri/src/models/mod.rs
```rust
use serde::{Deserialize, Serialize};

// ============================================================
// ВНУТРЕННИЕ МОДЕЛИ (для работы с БД)
// ============================================================

#[derive(Debug, Clone)]
pub struct Figurine {
    pub id: String,
    pub name: String,
    pub short_text: Option<String>,
    pub year: Option<i32>,
    pub status: FigurineStatus,
    pub sort_order: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FigurineStatus {
    Available,
    Sold,
    Reserved,
}

impl FigurineStatus {
    pub fn from_str(s: &str) -> Self {
        match s {
            "sold" => Self::Sold,
            "reserved" => Self::Reserved,
            _ => Self::Available,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Sold => "sold",
            Self::Reserved => "reserved",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    pub id: String,
    pub figurine_id: String,
    pub image_type: ImageType,
    pub file_path: String,
    pub alt_text: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImageType {
    Face,   // крупный план лица
    Detail, // детали
    Full,   // полный вид
}

impl ImageType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "face" => Self::Face,
            "detail" => Self::Detail,
            _ => Self::Full,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Face => "face",
            Self::Detail => "detail",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Text {
    pub id: String,
    pub category: TextCategory,
    pub content: String,
    pub caption: Option<String>,
    pub image_path: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextCategory {
    Author,
    Workshop,
}

impl TextCategory {
    pub fn from_str(s: &str) -> Self {
        match s {
            "author" => Self::Author,
            _ => Self::Workshop,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CabinetZone {
    pub id: String,
    pub zone_type: String,
    pub x_percent: f64,
    pub y_percent: f64,
    pub width_percent: f64,
    pub height_percent: f64,
    pub target_route: String,
}

// ============================================================
// DTO (для передачи на frontend, сериализуемые)
// ============================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FigurineDto {
    pub id: String,
    pub name: String,
    pub short_text: Option<String>,
    pub year: Option<i32>,
    pub status: String,
    pub images: Vec<ImageDto>,
}

impl FigurineDto {
    pub fn from_figurine(figurine: Figurine, images: Vec<Image>) -> Self {
        Self {
            id: figurine.id,
            name: figurine.name,
            short_text: figurine.short_text,
            year: figurine.year,
            status: figurine.status.as_str().to_string(),
            images: images.into_iter().map(ImageDto::from).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDto {
    pub id: String,
    pub image_type: String,
    pub url: String,  // путь для frontend (asset://)
    pub alt_text: Option<String>,
}

impl From<Image> for ImageDto {
    fn from(image: Image) -> Self {
        Self {
            id: image.id,
            image_type: image.image_type.as_str().to_string(),
            // Конвертируем путь в URL для Tauri
            url: format!("asset://localhost/{}", image.file_path),
            alt_text: image.alt_text,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FigurineListItemDto {
    pub id: String,
    pub name: String,
    pub status: String,
    pub face_image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDto {
    pub id: String,
    pub content: String,
}

impl From<Text> for TextDto {
    fn from(text: Text) -> Self {
        Self {
            id: text.id,
            content: text.content,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopItemDto {
    pub id: String,
    pub content: String,
    pub caption: Option<String>,
    pub image_url: Option<String>,
}

impl From<Text> for WorkshopItemDto {
    fn from(text: Text) -> Self {
        Self {
            id: text.id,
            content: text.content,
            caption: text.caption,
            image_url: text.image_path.map(|p| format!("asset://localhost/{}", p)),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CabinetZoneDto {
    pub id: String,
    pub zone_type: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub target_route: String,
}

impl From<CabinetZone> for CabinetZoneDto {
    fn from(zone: CabinetZone) -> Self {
        Self {
            id: zone.id,
            zone_type: zone.zone_type,
            x: zone.x_percent,
            y: zone.y_percent,
            width: zone.width_percent,
            height: zone.height_percent,
            target_route: zone.target_route,
        }
    }
}
```

### Критерии готовности
- [ ] Все структуры компилируются
- [ ] JSON сериализация работает (camelCase)
- [ ] Преобразования Model → DTO реализованы
- [ ] URL изображений формируются корректно

---

## Задача 5: Tauri-команды (API для frontend)

### Контекст
Frontend вызывает backend через `invoke()`. Нужно реализовать все необходимые команды.

### Цель
Создать Tauri-команды для всех операций с данными.

### Файл src-tauri/src/commands/mod.rs
```rust
use tauri::State;
use crate::db::Database;
use crate::db::repository::Repository;
use crate::models::*;

/// Получить список всех фигур (для витрины)
#[tauri::command]
pub async fn get_all_figurines(
    db: State<'_, Database>
) -> Result<Vec<FigurineListItemDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);

    let figurines = repo.get_all_figurines()
        .map_err(|e| format!("Database error: {}", e))?;

    let mut result = Vec::new();

    for fig in figurines {
        // Получить face-изображение
        let images = repo.get_images_for_figurine(&fig.id)
            .map_err(|e| format!("Database error: {}", e))?;

        let face_image = images.iter()
            .find(|img| img.image_type == ImageType::Face)
            .map(|img| format!("asset://localhost/{}", img.file_path));

        result.push(FigurineListItemDto {
            id: fig.id,
            name: fig.name,
            status: fig.status.as_str().to_string(),
            face_image_url: face_image,
        });
    }

    Ok(result)
}

/// Получить детальную информацию о фигуре
#[tauri::command]
pub async fn get_figurine(
    id: String,
    db: State<'_, Database>
) -> Result<Option<FigurineDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);

    let figurine = repo.get_figurine_by_id(&id)
        .map_err(|e| format!("Database error: {}", e))?;

    match figurine {
        Some(fig) => {
            let images = repo.get_images_for_figurine(&id)
                .map_err(|e| format!("Database error: {}", e))?;

            Ok(Some(FigurineDto::from_figurine(fig, images)))
        }
        None => Ok(None)
    }
}

/// Получить авторские тексты
#[tauri::command]
pub async fn get_author_texts(
    db: State<'_, Database>
) -> Result<Vec<TextDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);

    let texts = repo.get_texts_by_category("author")
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(texts.into_iter().map(TextDto::from).collect())
}

/// Получить контент мастерской
#[tauri::command]
pub async fn get_workshop_content(
    db: State<'_, Database>
) -> Result<Vec<WorkshopItemDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);

    let texts = repo.get_texts_by_category("workshop")
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(texts.into_iter().map(WorkshopItemDto::from).collect())
}

/// Получить интерактивные зоны кабинета
#[tauri::command]
pub async fn get_cabinet_zones(
    db: State<'_, Database>
) -> Result<Vec<CabinetZoneDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);

    let zones = repo.get_cabinet_zones()
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(zones.into_iter().map(CabinetZoneDto::from).collect())
}
```

### TypeScript типы для frontend src/lib/types/api.ts
```typescript
// Типы данных от backend

export interface FigurineListItem {
  id: string;
  name: string;
  status: 'available' | 'sold' | 'reserved';
  faceImageUrl: string | null;
}

export interface Figurine {
  id: string;
  name: string;
  shortText: string | null;
  year: number | null;
  status: 'available' | 'sold' | 'reserved';
  images: FigurineImage[];
}

export interface FigurineImage {
  id: string;
  imageType: 'face' | 'detail' | 'full';
  url: string;
  altText: string | null;
}

export interface AuthorText {
  id: string;
  content: string;
}

export interface WorkshopItem {
  id: string;
  content: string;
  caption: string | null;
  imageUrl: string | null;
}

export interface CabinetZone {
  id: string;
  zoneType: 'showcase' | 'desk' | 'shelf' | 'note';
  x: number;
  y: number;
  width: number;
  height: number;
  targetRoute: string;
}
```

### API-обёртка src/lib/api.ts
```typescript
import { invoke } from '@tauri-apps/api/core';
import type {
  FigurineListItem,
  Figurine,
  AuthorText,
  WorkshopItem,
  CabinetZone
} from './types/api';

export const api = {
  /**
   * Получить список всех фигур
   */
  async getAllFigurines(): Promise<FigurineListItem[]> {
    return invoke('get_all_figurines');
  },

  /**
   * Получить детальную информацию о фигуре
   */
  async getFigurine(id: string): Promise<Figurine | null> {
    return invoke('get_figurine', { id });
  },

  /**
   * Получить авторские тексты
   */
  async getAuthorTexts(): Promise<AuthorText[]> {
    return invoke('get_author_texts');
  },

  /**
   * Получить контент мастерской
   */
  async getWorkshopContent(): Promise<WorkshopItem[]> {
    return invoke('get_workshop_content');
  },

  /**
   * Получить зоны кабинета
   */
  async getCabinetZones(): Promise<CabinetZone[]> {
    return invoke('get_cabinet_zones');
  },
};
```

### Критерии готовности
- [ ] Все команды зарегистрированы в `main.rs`
- [ ] Frontend может вызывать `api.getAllFigurines()` и получать данные
- [ ] Ошибки БД возвращаются как понятные сообщения
- [ ] TypeScript типы соответствуют DTO

---

## Задача 6: Структура frontend и маршрутизация

### Контекст
SvelteKit в режиме SPA. Навигация только через клики по интерактивным элементам.

### Цель
Создать структуру страниц и систему навигации.

### Структура маршрутов
```
src/routes/
├── +layout.svelte          # общий layout с переходами
├── +page.svelte            # Cabinet Room (главная)
├── figurines/
│   ├── +page.svelte        # список фигур (полка)
│   └── [id]/
│       └── +page.svelte    # Figurine View (детали)
├── workshop/
│   └── +page.svelte        # Workshop (мастерская)
└── author/
    └── +page.svelte        # Author Presence
```

### Файл src/routes/+layout.svelte
```svelte
<script lang="ts">
  import '../app.css';
  import { fly, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { page } from '$app/stores';

  // Ключ для анимации переходов
  $: key = $page.url.pathname;
</script>

<div class="min-h-screen bg-cabinet-bg">
  {#key key}
    <main
      class="min-h-screen"
      in:fade={{ duration: 400, delay: 200, easing: cubicOut }}
      out:fade={{ duration: 300, easing: cubicOut }}
    >
      <slot />
    </main>
  {/key}
</div>
```

### Файл src/routes/+page.svelte (заглушка Cabinet Room)
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import type { CabinetZone } from '$lib/types/api';

  let zones: CabinetZone[] = [];
  let loaded = false;

  onMount(async () => {
    zones = await api.getCabinetZones();
    loaded = true;
  });

  function handleZoneClick(zone: CabinetZone) {
    goto(zone.targetRoute);
  }
</script>

<div class="relative w-full h-screen overflow-hidden">
  <!-- Фоновое изображение комнаты -->
  <div
    class="absolute inset-0 bg-cover bg-center"
    class:animate-fade-in-slow={loaded}
    style="background-image: url('/images/cabinet-room.jpg');"
  >
    <!-- Интерактивные зоны -->
    {#each zones as zone}
      <button
        class="hit-area"
        style="
          left: {zone.x}%;
          top: {zone.y}%;
          width: {zone.width}%;
          height: {zone.height}%;
        "
        on:click={() => handleZoneClick(zone)}
        aria-label="Перейти к {zone.zoneType}"
      />
    {/each}
  </div>
</div>
```

### Файл src/routes/figurines/+page.svelte (заглушка списка)
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import type { FigurineListItem } from '$lib/types/api';

  let figurines: FigurineListItem[] = [];

  onMount(async () => {
    figurines = await api.getAllFigurines();
  });
</script>

<div class="min-h-screen p-8">
  <button
    class="mb-8 text-cabinet-dust hover:text-cabinet-bone transition-colors duration-300"
    on:click={() => goto('/')}
  >
    ← Назад в кабинет
  </button>

  <h1 class="text-3xl text-cabinet-bone mb-12">Обитатели</h1>

  <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-8">
    {#each figurines as figurine, i}
      <button
        class="text-left group animate-fade-in"
        style="animation-delay: {i * 100}ms"
        on:click={() => goto(`/figurines/${figurine.id}`)}
      >
        {#if figurine.faceImageUrl}
          <img
            src={figurine.faceImageUrl}
            alt={figurine.name}
            class="w-full aspect-square object-cover cabinet-image mb-4"
          />
        {:else}
          <div class="w-full aspect-square bg-cabinet-wood/30 mb-4" />
        {/if}
        <h2 class="font-display text-xl text-cabinet-bone group-hover:text-cabinet-dust transition-colors duration-300">
          {figurine.name}
        </h2>
      </button>
    {/each}
  </div>
</div>
```

### Файл src/routes/figurines/[id]/+page.svelte (заглушка)
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { api } from '$lib/api';
  import type { Figurine } from '$lib/types/api';

  let figurine: Figurine | null = null;
  let selectedImageIndex = 0;

  $: id = $page.params.id;

  onMount(async () => {
    figurine = await api.getFigurine(id);
  });

  $: faceImage = figurine?.images.find(img => img.imageType === 'face');
  $: detailImages = figurine?.images.filter(img => img.imageType === 'detail') ?? [];
  $: currentImage = selectedImageIndex === 0 ? faceImage : detailImages[selectedImageIndex - 1];
</script>

{#if figurine}
  <div class="min-h-screen p-8 max-w-4xl mx-auto animate-fade-in">
    <button
      class="mb-8 text-cabinet-dust hover:text-cabinet-bone transition-colors duration-300"
      on:click={() => goto('/figurines')}
    >
      ← Назад
    </button>

    <!-- Основное изображение -->
    <div class="mb-6">
      {#if currentImage}
        <img
          src={currentImage.url}
          alt={currentImage.altText ?? figurine.name}
          class="w-full max-h-[60vh] object-contain"
        />
      {/if}
    </div>

    <!-- Галерея миниатюр -->
    {#if detailImages.length > 0}
      <div class="flex gap-4 mb-8">
        {#if faceImage}
          <button
            class="w-16 h-16 overflow-hidden border-2 transition-colors duration-300"
            class:border-cabinet-bone={selectedImageIndex === 0}
            class:border-transparent={selectedImageIndex !== 0}
            on:click={() => selectedImageIndex = 0}
          >
            <img src={faceImage.url} alt="" class="w-full h-full object-cover" />
          </button>
        {/if}
        {#each detailImages as img, i}
          <button
            class="w-16 h-16 overflow-hidden border-2 transition-colors duration-300"
            class:border-cabinet-bone={selectedImageIndex === i + 1}
            class:border-transparent={selectedImageIndex !== i + 1}
            on:click={() => selectedImageIndex = i + 1}
          >
            <img src={img.url} alt="" class="w-full h-full object-cover" />
          </button>
        {/each}
      </div>
    {/if}

    <!-- Имя -->
    <h1 class="font-display text-4xl text-cabinet-bone mb-6">
      {figurine.name}
    </h1>

    <!-- Текст -->
    {#if figurine.shortText}
      <p class="cabinet-quote max-w-2xl mb-8">
        «{figurine.shortText}»
      </p>
    {/if}

    <!-- Кнопка действия -->
    <button class="btn-cabinet">
      Узнать больше
    </button>
  </div>
{/if}
```

### Файл src/routes/workshop/+page.svelte (заглушка)
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import type { WorkshopItem } from '$lib/types/api';

  let items: WorkshopItem[] = [];

  // Генерация случайных позиций для "разбросанного" эффекта
  function getRandomPosition(index: number) {
    const seed = index * 17;
    return {
      x: 5 + (seed % 70),
      y: 10 + ((seed * 3) % 60),
      rotation: ((seed % 7) - 3),
    };
  }

  onMount(async () => {
    items = await api.getWorkshopContent();
  });
</script>

<div class="min-h-screen p-8 relative">
  <button
    class="mb-8 text-cabinet-dust hover:text-cabinet-bone transition-colors duration-300 relative z-10"
    on:click={() => goto('/')}
  >
    ← Назад в кабинет
  </button>

  <h1 class="text-3xl text-cabinet-bone mb-12 relative z-10">Мастерская</h1>

  <!-- Разбросанные элементы -->
  <div class="relative min-h-[70vh]">
    {#each items as item, i}
      {@const pos = getRandomPosition(i)}
      <div
        class="absolute w-64 animate-fade-in"
        style="
          left: {pos.x}%;
          top: {pos.y}%;
          transform: rotate({pos.rotation}deg);
          animation-delay: {i * 150}ms;
        "
      >
        {#if item.imageUrl}
          <img
            src={item.imageUrl}
            alt={item.caption ?? ''}
            class="w-full shadow-cabinet mb-2"
          />
        {/if}
        {#if item.caption}
          <p class="text-sm text-cabinet-dust">{item.caption}</p>
        {/if}
      </div>
    {/each}
  </div>
</div>
```

### Файл src/routes/author/+page.svelte (заглушка)
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import type { AuthorText } from '$lib/types/api';

  let texts: AuthorText[] = [];

  function getRandomStyle(index: number) {
    const seed = index * 13;
    return {
      rotation: ((seed % 5) - 2),
      marginLeft: (seed % 20) + '%',
      fontSize: ['text-lg', 'text-xl', 'text-base'][seed % 3],
    };
  }

  onMount(async () => {
    texts = await api.getAuthorTexts();
  });
</script>

<div class="min-h-screen p-8 max-w-3xl mx-auto">
  <button
    class="mb-8 text-cabinet-dust hover:text-cabinet-bone transition-colors duration-300"
    on:click={() => goto('/')}
  >
    ← Назад в кабинет
  </button>

  <div class="space-y-16 py-12">
    {#each texts as text, i}
      {@const style = getRandomStyle(i)}
      <blockquote
        class="cabinet-quote animate-fade-in {style.fontSize}"
        style="
          transform: rotate({style.rotation}deg);
          margin-left: {style.marginLeft};
          animation-delay: {i * 200}ms;
        "
      >
        «{text.content}»
      </blockquote>
    {/each}
  </div>
</div>
```

### Критерии готовности
- [ ] Все маршруты доступны
- [ ] Переходы между страницами плавные (fade)
- [ ] Кнопка "назад" работает на всех внутренних страницах
- [ ] Данные загружаются с backend
- [ ] Нет видимого меню навигации

---

## Задача 7: Cabinet Room — главное пространство (детальная реализация)

### Контекст
Это первый экран, который видит пользователь. Должен создать впечатление "я попал в чьё-то место".

### Цель
Реализовать атмосферную главную сцену с невидимыми интерактивными зонами.

### Визуальные требования
- Полноэкранное фоновое изображение комнаты
- Невидимые hit-areas поверх изображения
- При наведении — минимальное визуальное изменение
- Атмосфера тишины и присутствия

### Финальная реализация src/routes/+page.svelte
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { fade, scale } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { api } from '$lib/api';
  import type { CabinetZone } from '$lib/types/api';

  let zones: CabinetZone[] = [];
  let loaded = false;
  let hoveredZone: string | null = null;

  // Fallback зоны если БД пуста
  const defaultZones: CabinetZone[] = [
    { id: 'showcase', zoneType: 'showcase', x: 15, y: 25, width: 30, height: 45, targetRoute: '/figurines' },
    { id: 'desk', zoneType: 'desk', x: 35, y: 65, width: 35, height: 30, targetRoute: '/workshop' },
    { id: 'shelf', zoneType: 'shelf', x: 65, y: 20, width: 25, height: 35, targetRoute: '/figurines' },
    { id: 'note', zoneType: 'note', x: 70, y: 70, width: 15, height: 15, targetRoute: '/author' },
  ];

  onMount(async () => {
    try {
      const dbZones = await api.getCabinetZones();
      zones = dbZones.length > 0 ? dbZones : defaultZones;
    } catch {
      zones = defaultZones;
    }

    // Задержка для атмосферного появления
    setTimeout(() => {
      loaded = true;
    }, 100);
  });

  function handleZoneClick(zone: CabinetZone) {
    goto(zone.targetRoute);
  }

  function getZoneLabel(zoneType: string): string {
    const labels: Record<string, string> = {
      showcase: 'Витрина с фигурами',
      desk: 'Рабочий стол',
      shelf: 'Полка с обитателями',
      note: 'Записка автора',
    };
    return labels[zoneType] ?? zoneType;
  }
</script>

<svelte:head>
  <title>Cabinet of Curiosities</title>
</svelte:head>

<div class="relative w-full h-screen overflow-hidden bg-cabinet-bg">
  <!-- Фоновое изображение с плавным появлением -->
  {#if loaded}
    <div
      class="absolute inset-0"
      in:scale={{ duration: 1000, start: 1.02, opacity: 0, easing: cubicOut }}
    >
      <!-- Основное изображение комнаты -->
      <img
        src="/images/cabinet-room.jpg"
        alt="Кабинет редкостей"
        class="w-full h-full object-cover"
      />

      <!-- Затемнение по краям для глубины -->
      <div class="absolute inset-0 bg-gradient-radial from-transparent via-transparent to-cabinet-bg/50 pointer-events-none" />

      <!-- Интерактивные зоны -->
      {#each zones as zone, i}
        <button
          class="absolute transition-all duration-500 ease-cabinet rounded-sm cursor-pointer"
          style="
            left: {zone.x}%;
            top: {zone.y}%;
            width: {zone.width}%;
            height: {zone.height}%;
          "
          on:click={() => handleZoneClick(zone)}
          on:mouseenter={() => hoveredZone = zone.id}
          on:mouseleave={() => hoveredZone = null}
          aria-label={getZoneLabel(zone.zoneType)}
          in:fade={{ delay: 800 + i * 100, duration: 400 }}
        >
          <!-- Подсветка при наведении -->
          <div
            class="absolute inset-0 rounded-sm transition-all duration-500 ease-cabinet"
            class:bg-cabinet-bone/5={hoveredZone === zone.id}
            class:shadow-cabinet={hoveredZone === zone.id}
          />

          <!-- Тонкая рамка при наведении (почти незаметная) -->
          <div
            class="absolute inset-0 rounded-sm border transition-all duration-500 ease-cabinet"
            class:border-cabinet-bone/10={hoveredZone === zone.id}
            class:border-transparent={hoveredZone !== zone.id}
          />
        </button>
      {/each}
    </div>
  {/if}

  <!-- Начальный затемнённый экран (для эффекта "глаза привыкают") -->
  {#if !loaded}
    <div class="absolute inset-0 bg-cabinet-bg" />
  {/if}
</div>

<style>
  /* Радиальный градиент для виньетки */
  .bg-gradient-radial {
    background: radial-gradient(ellipse at center, transparent 0%, transparent 50%, rgba(46, 43, 40, 0.5) 100%);
  }
</style>
```

### Требования к изображению комнаты
Файл: `static/images/cabinet-room.jpg`
- Разрешение: минимум 1920x1080
- Стиль: тёплые тона, приглушённый свет, старый интерьер
- Должны быть видны: витрина/шкаф, стол, полка, место для записки
- НЕ должно быть: яркого света, современной мебели, белых поверхностей

### Критерии готовности
- [ ] Изображение загружается плавно (scale + fade)
- [ ] Зоны кликабельны но почти невидимы
- [ ] При наведении — едва заметная подсветка
- [ ] Нет ощущения "интерфейса" — только комната
- [ ] Работает на разных размерах окна

---

## Задача 8: Figurine View — экран персонажа (детальная реализация)

### Контекст
Детальный просмотр фигуры. Должен задержать внимание и вызвать эмоциональный отклик.

### Цель
Создать страницу, которая заставляет остановиться и рассмотреть.

### Ключевые принципы
- Фокус на изображении лица
- Текст — НЕ описание товара, а фрагмент истории
- Кнопка — нейтральная, не "Купить"
- Всё медленное, плавное

### Финальная реализация src/routes/figurines/[id]/+page.svelte
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { fade, fly, crossfade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { api } from '$lib/api';
  import type { Figurine, FigurineImage } from '$lib/types/api';

  let figurine: Figurine | null = null;
  let selectedImageIndex = 0;
  let loaded = false;

  // Crossfade для смены изображений
  const [send, receive] = crossfade({
    duration: 400,
    easing: cubicOut,
  });

  $: id = $page.params.id;

  onMount(async () => {
    figurine = await api.getFigurine(id);
    setTimeout(() => loaded = true, 100);
  });

  // Сортировка изображений: face первым, потом detail
  $: sortedImages = figurine?.images
    .slice()
    .sort((a, b) => {
      if (a.imageType === 'face') return -1;
      if (b.imageType === 'face') return 1;
      return 0;
    }) ?? [];

  $: currentImage = sortedImages[selectedImageIndex];

  function selectImage(index: number) {
    if (index !== selectedImageIndex) {
      selectedImageIndex = index;
    }
  }

  function goBack() {
    goto('/figurines');
  }
</script>

<svelte:head>
  <title>{figurine?.name ?? 'Загрузка...'} — Cabinet of Curiosities</title>
</svelte:head>

{#if figurine && loaded}
  <div
    class="min-h-screen bg-cabinet-bg"
    in:fade={{ duration: 500, easing: cubicOut }}
  >
    <div class="max-w-5xl mx-auto px-6 py-8 lg:px-12 lg:py-12">

      <!-- Кнопка назад -->
      <button
        class="group flex items-center gap-2 text-cabinet-dust
               hover:text-cabinet-bone transition-colors duration-400 mb-8"
        on:click={goBack}
        in:fade={{ delay: 200, duration: 400 }}
      >
        <span class="transform group-hover:-translate-x-1 transition-transform duration-300">←</span>
        <span class="text-sm">Назад к обитателям</span>
      </button>

      <div class="grid lg:grid-cols-2 gap-12 lg:gap-16">

        <!-- Левая колонка: изображения -->
        <div
          class="space-y-6"
          in:fly={{ x: -20, duration: 600, delay: 300, easing: cubicOut }}
        >
          <!-- Основное изображение -->
          <div class="relative aspect-[3/4] bg-cabinet-wood/20 overflow-hidden">
            {#key currentImage?.id}
              {#if currentImage}
                <img
                  src={currentImage.url}
                  alt={currentImage.altText ?? figurine.name}
                  class="absolute inset-0 w-full h-full object-cover"
                  in:fade={{ duration: 400, easing: cubicOut }}
                />
              {/if}
            {/key}

            <!-- Тонкая рамка -->
            <div class="absolute inset-0 border border-cabinet-wood/30 pointer-events-none" />
          </div>

          <!-- Миниатюры -->
          {#if sortedImages.length > 1}
            <div
              class="flex gap-3"
              in:fade={{ delay: 500, duration: 400 }}
            >
              {#each sortedImages as img, i}
                <button
                  class="relative w-16 h-16 lg:w-20 lg:h-20 overflow-hidden
                         transition-all duration-400 ease-cabinet"
                  class:ring-1={selectedImageIndex === i}
                  class:ring-cabinet-bone={selectedImageIndex === i}
                  class:ring-offset-2={selectedImageIndex === i}
                  class:ring-offset-cabinet-bg={selectedImageIndex === i}
                  class:opacity-60={selectedImageIndex !== i}
                  class:hover:opacity-80={selectedImageIndex !== i}
                  on:click={() => selectImage(i)}
                >
                  <img
                    src={img.url}
                    alt=""
                    class="w-full h-full object-cover"
                  />
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <!-- Правая колонка: информация -->
        <div
          class="flex flex-col justify-center"
          in:fly={{ x: 20, duration: 600, delay: 400, easing: cubicOut }}
        >
          <!-- Имя персонажа -->
          <h1 class="font-display text-3xl lg:text-4xl text-cabinet-bone mb-2 tracking-wide">
            {figurine.name}
          </h1>

          <!-- Год (если есть) -->
          {#if figurine.year}
            <p class="text-cabinet-dust text-sm mb-8">{figurine.year}</p>
          {/if}

          <!-- Текст персонажа -->
          {#if figurine.shortText}
            <blockquote
              class="relative pl-6 mb-10"
              in:fade={{ delay: 600, duration: 500 }}
            >
              <!-- Декоративная линия -->
              <div class="absolute left-0 top-0 bottom-0 w-px bg-cabinet-fabric/50" />

              <p class="font-display text-xl lg:text-2xl text-cabinet-dust leading-relaxed italic">
                «{figurine.shortText}»
              </p>
            </blockquote>
          {/if}

          <!-- Статус -->
          {#if figurine.status === 'sold'}
            <p class="text-cabinet-fabric text-sm mb-6 tracking-wide uppercase">
              Нашёл дом
            </p>
          {:else if figurine.status === 'reserved'}
            <p class="text-accent-olive text-sm mb-6 tracking-wide uppercase">
              Ожидает встречи
            </p>
          {/if}

          <!-- Кнопка действия (только для available) -->
          {#if figurine.status === 'available'}
            <div in:fade={{ delay: 700, duration: 400 }}>
              <button class="btn-cabinet">
                Узнать историю
              </button>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
{:else}
  <!-- Загрузка -->
  <div class="min-h-screen bg-cabinet-bg flex items-center justify-center">
    <div class="text-cabinet-dust animate-pulse">...</div>
  </div>
{/if}
```

### Требования к тексту персонажей
Текст должен быть:
- НЕ описанием ("Фигура высотой 30 см из полимерной глины")
- НЕ маркетингом ("Уникальная авторская работа, идеальный подарок")
- А фрагментом истории, атмосферой, ощущением

**Примеры хорошего текста:**
> «Она никогда не подметала следы. Дом сам их забывал.»

> «В каждом доме есть тот, кто помнит все разговоры. Он не повторяет их — просто хранит.»

> «Он пришёл с севера, но никогда не говорил откуда именно.»

**Примеры плохого текста:**
> «Коллекционная фигура ручной работы из качественных материалов»

> «Отличный подарок для ценителей авторского искусства»

### Критерии готовности
- [ ] Изображение — главный фокус страницы
- [ ] Галерея работает плавно (crossfade)
- [ ] Текст вызывает интерес, а не желание пролистать
- [ ] Кнопка НЕ выглядит как "Купить"
- [ ] Статус "sold" отображается элегантно ("Нашёл дом")

---

## Задача 9: Workshop — мастерская (детальная реализация)

### Контекст
Показывает процесс создания. Должна выглядеть как рабочий стол с разбросанными элементами.

### Цель
Создать ощущение "заглянул в мастерскую художника".

### Принципы
- НЕ аккуратная сетка
- Элементы "разбросаны" естественно
- Разные размеры, небольшие повороты
- Ощущение беспорядка творческого процесса

### Финальная реализация src/routes/workshop/+page.svelte
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { api } from '$lib/api';
  import type { WorkshopItem } from '$lib/types/api';

  let items: WorkshopItem[] = [];
  let loaded = false;
  let expandedItem: string | null = null;

  // Генератор "случайных" но стабильных позиций
  function getItemStyle(index: number, total: number) {
    // Используем index как seed для псевдослучайности
    const seed = (index + 1) * 17;

    // Располагаем в сетке 3 колонки, но со смещениями
    const col = index % 3;
    const row = Math.floor(index / 3);

    // Базовые позиции с вариацией
    const baseX = 5 + col * 32;
    const baseY = row * 35;

    // Добавляем "случайное" смещение
    const offsetX = (seed % 15) - 7;
    const offsetY = ((seed * 3) % 10) - 5;

    // Небольшой поворот
    const rotation = ((seed % 7) - 3) * 0.8;

    // Разные размеры
    const sizes = ['w-56', 'w-64', 'w-48', 'w-72'];
    const size = sizes[seed % sizes.length];

    return {
      left: `${baseX + offsetX}%`,
      top: `${baseY + offsetY}%`,
      rotation: `${rotation}deg`,
      size,
      zIndex: index,
    };
  }

  onMount(async () => {
    items = await api.getWorkshopContent();
    setTimeout(() => loaded = true, 100);
  });

  function toggleExpand(id: string) {
    expandedItem = expandedItem === id ? null : id;
  }
</script>

<svelte:head>
  <title>Мастерская — Cabinet of Curiosities</title>
</svelte:head>

<div
  class="min-h-screen bg-cabinet-bg overflow-hidden"
  in:fade={{ duration: 400 }}
>
  <!-- Текстура поверхности стола -->
  <div
    class="fixed inset-0 opacity-10 pointer-events-none"
    style="background-image: url('/images/wood-texture.jpg'); background-size: cover;"
  />

  <div class="relative z-10 p-6 lg:p-12">
    <!-- Навигация -->
    <button
      class="group flex items-center gap-2 text-cabinet-dust
             hover:text-cabinet-bone transition-colors duration-400 mb-6"
      on:click={() => goto('/')}
      in:fade={{ delay: 200, duration: 400 }}
    >
      <span class="transform group-hover:-translate-x-1 transition-transform duration-300">←</span>
      <span class="text-sm">Назад в кабинет</span>
    </button>

    <!-- Заголовок -->
    <h1
      class="font-display text-2xl lg:text-3xl text-cabinet-bone mb-4"
      in:fly={{ y: -10, duration: 500, delay: 300 }}
    >
      Мастерская
    </h1>

    <p
      class="text-cabinet-dust text-sm max-w-md mb-12"
      in:fade={{ delay: 400, duration: 400 }}
    >
      Здесь рождаются обитатели кабинета. Процесс долгий — каждая деталь требует времени.
    </p>

    <!-- Разбросанные элементы -->
    <div class="relative min-h-[80vh]">
      {#if loaded}
        {#each items as item, i}
          {@const style = getItemStyle(i, items.length)}
          <div
            class="absolute {style.size} cursor-pointer group"
            style="
              left: {style.left};
              top: {style.top};
              transform: rotate({style.rotation});
              z-index: {expandedItem === item.id ? 100 : style.zIndex};
            "
            in:fly={{
              y: 30,
              duration: 600,
              delay: 500 + i * 150,
              easing: cubicOut
            }}
            on:click={() => toggleExpand(item.id)}
            on:keypress={(e) => e.key === 'Enter' && toggleExpand(item.id)}
            role="button"
            tabindex="0"
          >
            <!-- Карточка -->
            <div
              class="bg-cabinet-wood/20 p-3 shadow-cabinet
                     transition-all duration-500 ease-cabinet
                     group-hover:shadow-cabinet-lg group-hover:-translate-y-1
                     {expandedItem === item.id ? 'scale-110 shadow-cabinet-lg' : ''}"
            >
              <!-- Изображение -->
              {#if item.imageUrl}
                <div class="relative overflow-hidden mb-3">
                  <img
                    src={item.imageUrl}
                    alt={item.caption ?? 'Процесс работы'}
                    class="w-full"
                  />
                  <!-- Лёгкая виньетка -->
                  <div class="absolute inset-0 shadow-cabinet-inner pointer-events-none" />
                </div>
              {/if}

              <!-- Подпись -->
              {#if item.caption}
                <p class="text-cabinet-dust text-xs leading-relaxed">
                  {item.caption}
                </p>
              {/if}
            </div>

            <!-- Тень "бумаги" -->
            <div
              class="absolute -bottom-2 -right-2 -z-10 w-full h-full
                     bg-black/20 blur-sm transition-all duration-500"
              class:blur-md={expandedItem === item.id}
            />
          </div>
        {/each}
      {/if}
    </div>
  </div>
</div>
```

### Требования к контенту мастерской
Фотографии и подписи должны показывать:
- Этапы создания (наброски, лепка, роспись)
- Инструменты и материалы
- Детали процесса

**Примеры хороших подписей:**
> «Первые наброски. Лицо ещё не решило, кем быть.»

> «Руки сохнут три дня. Терпение — часть материала.»

> «Ткань должна помнить, что была чем-то другим.»

### Критерии готовности
- [ ] Элементы выглядят "разбросанными", не выровненными
- [ ] При наведении — элемент приподнимается
- [ ] При клике — элемент увеличивается
- [ ] Ощущение рабочего беспорядка, не галереи

---

## Задача 10: Author Presence — присутствие автора (детальная реализация)

### Контекст
Создаёт ощущение личного присутствия художника через обрывочные тексты.

### Цель
Передать голос автора без биографии и маркетинга.

### Принципы
- Тексты как записки, найденные случайно
- Без хронологии и структуры
- Обрывочные мысли, не завершённые истории
- Разное позиционирование, как будто разбросаны

### Финальная реализация src/routes/author/+page.svelte
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { api } from '$lib/api';
  import type { AuthorText } from '$lib/types/api';

  let texts: AuthorText[] = [];
  let loaded = false;

  // Стили для каждой записки
  function getNoteStyle(index: number) {
    const seed = (index + 1) * 23;

    // Разные размеры шрифта
    const sizes = ['text-lg', 'text-xl', 'text-2xl', 'text-base'];

    // Смещение от центра
    const alignments = [
      'ml-0 mr-auto',           // слева
      'ml-auto mr-0',           // справа
      'mx-auto',                // центр
      'ml-12',                  // слегка справа
      'mr-12 ml-auto',          // слегка слева
    ];

    // Небольшой поворот
    const rotation = ((seed % 5) - 2) * 0.5;

    return {
      size: sizes[seed % sizes.length],
      alignment: alignments[seed % alignments.length],
      rotation: `${rotation}deg`,
      maxWidth: ['max-w-md', 'max-w-lg', 'max-w-sm'][seed % 3],
    };
  }

  onMount(async () => {
    texts = await api.getAuthorTexts();
    setTimeout(() => loaded = true, 100);
  });
</script>

<svelte:head>
  <title>Голос — Cabinet of Curiosities</title>
</svelte:head>

<div
  class="min-h-screen bg-cabinet-bg"
  in:fade={{ duration: 400 }}
>
  <div class="max-w-4xl mx-auto px-6 py-8 lg:px-12 lg:py-16">

    <!-- Навигация -->
    <button
      class="group flex items-center gap-2 text-cabinet-dust
             hover:text-cabinet-bone transition-colors duration-400 mb-16"
      on:click={() => goto('/')}
      in:fade={{ delay: 200, duration: 400 }}
    >
      <span class="transform group-hover:-translate-x-1 transition-transform duration-300">←</span>
      <span class="text-sm">Назад в кабинет</span>
    </button>

    <!-- Записки автора -->
    <div class="space-y-20 lg:space-y-28">
      {#if loaded}
        {#each texts as text, i}
          {@const style = getNoteStyle(i)}
          <blockquote
            class="{style.size} {style.alignment} {style.maxWidth}"
            style="transform: rotate({style.rotation});"
            in:fly={{
              y: 20,
              duration: 600,
              delay: 300 + i * 200,
              easing: cubicOut
            }}
          >
            <!-- Декоративный элемент -->
            <div class="w-8 h-px bg-cabinet-fabric/30 mb-6" />

            <!-- Текст -->
            <p class="font-display text-cabinet-dust leading-relaxed italic">
              «{text.content}»
            </p>

            <!-- Нижний декоративный элемент (не у всех) -->
            {#if i % 3 === 0}
              <div class="w-4 h-px bg-cabinet-fabric/20 mt-6 ml-auto" />
            {/if}
          </blockquote>
        {/each}

        <!-- Финальный элемент — пустота -->
        <div
          class="h-32"
          in:fade={{ delay: 300 + texts.length * 200 + 400, duration: 800 }}
        />
      {/if}
    </div>
  </div>
</div>
```

### Требования к авторским текстам
Тексты должны быть:
- Короткими (1-2 предложения)
- Обрывочными (не завершённые мысли)
- Атмосферными (создают настроение)
- Личными (голос автора)

**Примеры хороших текстов:**
> «Я оставляю трещины. Они говорят больше, чем гладкая поверхность.»

> «Каждая фигура помнит руки, которые её создали.»

> «Иногда я не знаю, кто кого создаёт — я их или они меня.»

> «Пыль — это не грязь. Это время, которое осело.»

> «Я не делаю кукол. Куклы улыбаются.»

**Примеры плохих текстов:**
> «Я художник с 15-летним стажем, работаю в технике...»

> «Мои работы можно найти в коллекциях по всему миру...»

> «Добро пожаловать в мой мир творчества!»

### Критерии готовности
- [ ] Тексты появляются с задержкой (эффект чтения)
- [ ] Разное позиционирование (не список)
- [ ] Разный размер шрифта
- [ ] Ощущение "нашёл чьи-то записки"
- [ ] Нет биографии, нет маркетинга

---

## Задача 11: Тестовые данные и seed-скрипт

### Контекст
Для тестирования и демонстрации нужен реалистичный контент.

### Цель
Создать SQL-скрипт для заполнения БД тестовыми данными.

### Файл src-tauri/src/db/seed.sql
```sql
-- ============================================================
-- ТЕСТОВЫЕ ДАННЫЕ ДЛЯ MVP
-- Запускать после создания схемы
-- ============================================================

-- Очистка (для повторного запуска)
DELETE FROM images;
DELETE FROM texts;
DELETE FROM figurines;
DELETE FROM cabinet_zones;

-- ============================================================
-- ФИГУРЫ (5-7 штук)
-- ============================================================

INSERT INTO figurines (id, name, short_text, year, status, sort_order) VALUES
(
  'fig-001',
  'Хранительница порога',
  'Она стоит там, где заканчивается один дом и начинается другой. Никто не знает, что она охраняет — вход или выход.',
  2023,
  'available',
  1
),
(
  'fig-002',
  'Тот, кто слушает стены',
  'В каждом доме есть тот, кто помнит все разговоры. Он не повторяет их — просто хранит.',
  2024,
  'available',
  2
),
(
  'fig-003',
  'Собирательница пыли',
  'Она никогда не подметала следы. Дом сам их забывал.',
  2023,
  'sold',
  3
),
(
  'fig-004',
  'Молчаливый гость',
  'Он пришёл с севера, но никогда не говорил откуда именно. В его карманах — только сухие листья.',
  2024,
  'available',
  4
),
(
  'fig-005',
  'Та, что видит сквозь',
  'Её глаза закрыты, но она видит больше других. Говорят, она знает, где спрятаны потерянные вещи.',
  2022,
  'reserved',
  5
),
(
  'fig-006',
  'Хранитель старых имён',
  'У него есть список. Никто не знает, чьи имена в нём записаны, и зачем он их хранит.',
  2024,
  'available',
  6
),
(
  'fig-007',
  'Ночной садовник',
  'Он выходит только после заката. Его сад не виден при свете дня.',
  2023,
  'available',
  7
);

-- ============================================================
-- ИЗОБРАЖЕНИЯ
-- Пути относительные, будут в static/images/
-- ============================================================

-- Хранительница порога
INSERT INTO images (id, figurine_id, image_type, file_path, alt_text, sort_order) VALUES
('img-001-face', 'fig-001', 'face', 'images/figurines/fig-001/face.jpg', 'Лицо Хранительницы порога', 1),
('img-001-detail-1', 'fig-001', 'detail', 'images/figurines/fig-001/detail-1.jpg', 'Руки Хранительницы', 2),
('img-001-detail-2', 'fig-001', 'detail', 'images/figurines/fig-001/detail-2.jpg', 'Одежда Хранительницы', 3),
('img-001-full', 'fig-001', 'full', 'images/figurines/fig-001/full.jpg', 'Хранительница порога полностью', 4);

-- Тот, кто слушает стены
INSERT INTO images (id, figurine_id, image_type, file_path, alt_text, sort_order) VALUES
('img-002-face', 'fig-002', 'face', 'images/figurines/fig-002/face.jpg', 'Лицо Слушающего', 1),
('img-002-detail-1', 'fig-002', 'detail', 'images/figurines/fig-002/detail-1.jpg', 'Уши Слушающего', 2),
('img-002-full', 'fig-002', 'full', 'images/figurines/fig-002/full.jpg', 'Слушающий полностью', 3);

-- Собирательница пыли
INSERT INTO images (id, figurine_id, image_type, file_path, alt_text, sort_order) VALUES
('img-003-face', 'fig-003', 'face', 'images/figurines/fig-003/face.jpg', 'Лицо Собирательницы', 1),
('img-003-detail-1', 'fig-003', 'detail', 'images/figurines/fig-003/detail-1.jpg', 'Платье Собирательницы', 2);

-- Молчаливый гость
INSERT INTO images (id, figurine_id, image_type, file_path, alt_text, sort_order) VALUES
('img-004-face', 'fig-004', 'face', 'images/figurines/fig-004/face.jpg', 'Лицо Молчаливого гостя', 1),
('img-004-detail-1', 'fig-004', 'detail', 'images/figurines/fig-004/detail-1.jpg', 'Карманы с листьями', 2),
('img-004-full', 'fig-004', 'full', 'images/figurines/fig-004/full.jpg', 'Молчаливый гость полностью', 3);

-- Та, что видит сквозь
INSERT INTO images (id, figurine_id, image_type, file_path, alt_text, sort_order) VALUES
('img-005-face', 'fig-005', 'face', 'images/figurines/fig-005/face.jpg', 'Закрытые глаза', 1),
('img-005-detail-1', 'fig-005', 'detail', 'images/figurines/fig-005/detail-1.jpg', 'Руки провидицы', 2);

-- Хранитель старых имён
INSERT INTO images (id, figurine_id, image_type, file_path, alt_text, sort_order) VALUES
('img-006-face', 'fig-006', 'face', 'images/figurines/fig-006/face.jpg', 'Лицо Хранителя', 1),
('img-006-detail-1', 'fig-006', 'detail', 'images/figurines/fig-006/detail-1.jpg', 'Список имён', 2);

-- Ночной садовник
INSERT INTO images (id, figurine_id, image_type, file_path, alt_text, sort_order) VALUES
('img-007-face', 'fig-007', 'face', 'images/figurines/fig-007/face.jpg', 'Лицо Садовника', 1),
('img-007-detail-1', 'fig-007', 'detail', 'images/figurines/fig-007/detail-1.jpg', 'Инструменты садовника', 2),
('img-007-full', 'fig-007', 'full', 'images/figurines/fig-007/full.jpg', 'Ночной садовник полностью', 3);

-- ============================================================
-- АВТОРСКИЕ ТЕКСТЫ
-- ============================================================

INSERT INTO texts (id, category, content, sort_order) VALUES
('author-001', 'author', 'Я оставляю трещины. Они говорят больше, чем гладкая поверхность.', 1),
('author-002', 'author', 'Каждая фигура помнит руки, которые её создали.', 2),
('author-003', 'author', 'Иногда я не знаю, кто кого создаёт — я их или они меня.', 3),
('author-004', 'author', 'Пыль — это не грязь. Это время, которое осело.', 4),
('author-005', 'author', 'Я не делаю кукол. Куклы улыбаются.', 5);

-- ============================================================
-- КОНТЕНТ МАСТЕРСКОЙ
-- ============================================================

INSERT INTO texts (id, category, content, caption, image_path, sort_order) VALUES
(
  'workshop-001',
  'workshop',
  'Начало',
  'Первые наброски. Лицо ещё не решило, кем быть.',
  'images/workshop/sketch.jpg',
  1
),
(
  'workshop-002',
  'workshop',
  'Форма',
  'Глина помнит каждое прикосновение. Нельзя торопиться.',
  'images/workshop/clay.jpg',
  2
),
(
  'workshop-003',
  'workshop',
  'Ожидание',
  'Руки сохнут три дня. Терпение — часть материала.',
  'images/workshop/drying.jpg',
  3
),
(
  'workshop-004',
  'workshop',
  'Цвет',
  'Краска должна быть приглушённой. Яркость — для других историй.',
  'images/workshop/painting.jpg',
  4
),
(
  'workshop-005',
  'workshop',
  'Одежда',
  'Ткань должна помнить, что была чем-то другим.',
  'images/workshop/fabric.jpg',
  5
),
(
  'workshop-006',
  'workshop',
  'Детали',
  'Последние штрихи. Здесь решается, оживёт ли.',
  'images/workshop/details.jpg',
  6
);

-- ============================================================
-- ЗОНЫ КАБИНЕТА
-- ============================================================

INSERT INTO cabinet_zones (id, zone_type, x_percent, y_percent, width_percent, height_percent, target_route, sort_order) VALUES
('zone-showcase', 'showcase', 15, 20, 30, 50, '/figurines', 1),
('zone-desk', 'desk', 35, 65, 35, 30, '/workshop', 2),
('zone-shelf', 'shelf', 60, 15, 30, 40, '/figurines', 3),
('zone-note', 'note', 75, 70, 12, 12, '/author', 4);
```

### Структура папок для изображений
```
static/
├── images/
│   ├── cabinet-room.jpg          # Главное изображение комнаты
│   ├── wood-texture.jpg          # Текстура дерева для мастерской
│   ├── figurines/
│   │   ├── fig-001/
│   │   │   ├── face.jpg
│   │   │   ├── detail-1.jpg
│   │   │   ├── detail-2.jpg
│   │   │   └── full.jpg
│   │   ├── fig-002/
│   │   │   ├── face.jpg
│   │   │   ├── detail-1.jpg
│   │   │   └── full.jpg
│   │   └── ... (остальные фигуры)
│   └── workshop/
│       ├── sketch.jpg
│       ├── clay.jpg
│       ├── drying.jpg
│       ├── painting.jpg
│       ├── fabric.jpg
│       └── details.jpg
```

### Команда для seed (добавить в Repository)
```rust
impl Database {
    pub fn seed_if_empty(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Проверить, есть ли данные
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM figurines",
            [],
            |row| row.get(0)
        )?;

        if count == 0 {
            conn.execute_batch(include_str!("seed.sql"))?;
        }

        Ok(())
    }
}
```

### Критерии готовности
- [ ] Seed-скрипт выполняется без ошибок
- [ ] 7 фигур в базе
- [ ] 5 авторских текстов
- [ ] 6 элементов мастерской
- [ ] 4 зоны кабинета
- [ ] Структура папок создана
- [ ] Placeholder-изображения на местах

---

## Задача 12: Финальная интеграция и проверка

### Контекст
Все компоненты готовы, нужно собрать и проверить MVP.

### Цель
Убедиться, что приложение работает как единое целое и соответствует философии проекта.

### Чеклист интеграции

#### Backend
- [ ] БД инициализируется при первом запуске
- [ ] Seed-данные загружаются если БД пуста
- [ ] Все команды работают и возвращают данные
- [ ] Пути к изображениям корректные

#### Frontend
- [ ] Все страницы загружаются
- [ ] Переходы между страницами плавные
- [ ] Данные отображаются корректно
- [ ] Нет console.log в production

#### Визуально
- [ ] Нигде нет белого цвета
- [ ] Нигде нет жирного текста
- [ ] Анимации медленные (не быстрее 300ms)
- [ ] Шрифты загружаются (Cormorant Garamond, Source Serif 4)

#### Атмосфера (субъективно)
- [ ] Cabinet Room создаёт ощущение "вошёл в комнату"
- [ ] Figurine View задерживает внимание
- [ ] Workshop выглядит как рабочий стол
- [ ] Author Presence — как найденные записки
- [ ] Нет ощущения магазина
- [ ] Нет желания "пролистать"

### Команды для сборки
```bash
# Development
npm run dev
cargo tauri dev

# Production build
cargo tauri build

# Проверка размера бандла
ls -lh src-tauri/target/release/bundle/
```

### Тестирование на разных размерах
- [ ] 800x600 (минимум)
- [ ] 1280x720 (laptop)
- [ ] 1920x1080 (desktop)
- [ ] Резкое изменение размера окна

### Возможные проблемы и решения

| Проблема | Решение |
|----------|---------|
| Изображения не загружаются | Проверить asset:// protocol в tauri.conf.json |
| Шрифты не применяются | Проверить Google Fonts import в app.css |
| Белые элементы | Искать bg-white, text-white, border-white |
| Жирный текст | Искать font-bold, font-semibold в классах |
| Быстрые анимации | Искать duration-100, duration-200 |

### Критерии готовности MVP
Согласно spec3.md:

1. **Пользователь проводит >3 минут**
   - Контент интересен
   - Нет желания закрыть
   - Хочется рассмотреть

2. **Не возникает желания "пролистать"**
   - Тексты читаются
   - Изображения рассматриваются
   - Нет информационного шума

3. **Фигурки запоминаются**
   - У каждой свой характер
   - Тексты создают образ
   - Остаётся впечатление

---

## Порядок выполнения

```
[1] Инициализация Tauri + SvelteKit
         ↓
[2] Tailwind CSS с темой ←──────────────────────┐
         ↓                                       │
[3] SQLite схема и инициализация                 │
         ↓                                       │
[4] Rust модели и DTO                            │
         ↓                                       │
[5] Tauri команды (API)                          │
         ↓                                       │
[6] Структура frontend ──────────────────────────┤
         ↓                                       │
    ┌────┴────┬─────────┬──────────┐             │
    ↓         ↓         ↓          ↓             │
[7]       [8]       [9]        [10]              │
Cabinet   Figurine  Workshop   Author            │
Room      View                 Presence          │
    └────┬────┴─────────┴──────────┘             │
         ↓                                       │
[11] Тестовые данные ────────────────────────────┘
         ↓
[12] Финальная интеграция
```

---

## Памятка для LLM-агента

### ДЕЛАТЬ
- Читать этот документ перед каждой задачей
- Проверять визуально после каждого изменения
- Использовать классы Tailwind из темы проекта
- Делать всё медленным и плавным
- Сохранять атмосферу "тишины"

### НЕ ДЕЛАТЬ
- Использовать белый цвет (bg-white, text-white)
- Использовать жирный текст (font-bold)
- Делать быстрые анимации (<300ms)
- Добавлять маркетинговые тексты
- Превращать в каталог/магазин
- Добавлять функции сверх MVP

### Если не уверен
Спроси: "Это разрушает тишину? Это ускоряет восприятие? Это похоже на магазин?"

Если да — не делай.

---

*Документ создан на основе spec3.md*
*Версия: 2.0 — с Tailwind CSS и детальными инструкциями*
