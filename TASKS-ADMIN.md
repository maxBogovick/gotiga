# Cabinet of Curiosities — Админка (Панель управления)

## Назначение

Админка — это **отдельное desktop-приложение** для управления контентом Cabinet of Curiosities.

Позволяет:
- Добавлять, редактировать, удалять фигуры
- Загружать и управлять изображениями
- Редактировать тексты (авторские, мастерская)
- Настраивать интерактивные зоны кабинета
- Предпросматривать контент

---

## Технологический стек

| Слой | Технология | Примечание |
|------|------------|------------|
| Desktop shell | Tauri 2.x | Отдельное приложение |
| Backend | Rust | Общая БД с основным приложением |
| Frontend | SvelteKit (SPA) | Функциональный UI |
| Стили | Tailwind CSS | Нейтральная тема (не как в основном приложении) |
| База данных | SQLite | Та же БД `cabinet.db` |

---

## Архитектурное решение

### Вариант: Отдельное приложение (рекомендуется)

```
gotiga/     # Основное приложение
cabinet-admin/              # Админка (отдельно)
shared/
├── database/               # Общий путь к БД
└── images/                 # Общие изображения
```

**Преимущества:**
- Чистое разделение логики
- Основное приложение остаётся минимальным
- Админка может иметь свой UI-стиль
- Можно запускать параллельно

---

## Задача A1: Инициализация проекта админки

### Контекст
Создаём отдельное Tauri-приложение для управления контентом.

### Цель
Рабочий каркас админки с доступом к общей БД.

### Пошаговая инструкция

#### Шаг A1.1: Создание проекта
```bash
# В родительской директории
npm create tauri-app@latest cabinet-admin -- --template sveltekit-ts

cd cabinet-admin
```

#### Шаг A1.2: Структура директорий
```
cabinet-admin/
├── src/                    # SvelteKit frontend
│   ├── lib/
│   │   ├── components/     # UI компоненты
│   │   │   ├── forms/      # Формы редактирования
│   │   │   ├── lists/      # Списки с действиями
│   │   │   └── ui/         # Базовые UI элементы
│   │   ├── stores/         # Состояние
│   │   └── api.ts          # API вызовы
│   └── routes/
│       ├── +layout.svelte
│       ├── +page.svelte           # Dashboard
│       ├── figurines/
│       │   ├── +page.svelte       # Список фигур
│       │   ├── new/+page.svelte   # Создание
│       │   └── [id]/+page.svelte  # Редактирование
│       ├── texts/
│       │   ├── author/+page.svelte
│       │   └── workshop/+page.svelte
│       ├── zones/+page.svelte
│       └── preview/+page.svelte
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── db/             # Работа с БД
│   │   ├── commands/       # CRUD команды
│   │   └── file_ops/       # Операции с файлами
│   └── tauri.conf.json
└── static/
```

#### Шаг A1.3: Настройка tauri.conf.json
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Cabinet Admin",
  "version": "0.1.0",
  "identifier": "com.cabinet.admin",
  "build": {
    "frontendDist": "../build"
  },
  "app": {
    "windows": [
      {
        "title": "Cabinet Admin",
        "width": 1400,
        "height": 900,
        "minWidth": 1000,
        "minHeight": 700,
        "resizable": true,
        "center": true
      }
    ],
    "security": {
      "csp": null
    }
  },
  "plugins": {
    "dialog": {
      "open": true,
      "save": true
    },
    "fs": {
      "scope": ["$APPDATA/**", "$RESOURCE/**", "**"]
    }
  }
}
```

#### Шаг A1.4: Зависимости Rust (Cargo.toml)
```toml
[package]
name = "cabinet-admin"
version = "0.1.0"
edition = "2021"

[lib]
name = "cabinet_admin_lib"
crate-type = ["lib", "cdylib", "staticlib"]

[dependencies]
tauri = { version = "2", features = ["protocol-asset"] }
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
rusqlite = { version = "0.31", features = ["bundled"] }
uuid = { version = "1", features = ["v4"] }
thiserror = "1"
image = "0.25"  # Для обработки изображений
```

#### Шаг A1.5: Настройка пути к общей БД
```rust
// src-tauri/src/db/mod.rs
use std::path::PathBuf;

pub fn get_shared_db_path() -> PathBuf {
    // Путь к БД основного приложения
    // В production это будет в AppData
    // В development — относительный путь

    if cfg!(debug_assertions) {
        // Development: относительный путь к основному приложению
        PathBuf::from("../gotiga/data/cabinet.db")
    } else {
        // Production: стандартный путь AppData
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("com.cabinet.curiosities")
            .join("cabinet.db")
    }
}
```

### Критерии готовности
- [ ] Проект создан и запускается
- [ ] Админка подключается к общей БД
- [ ] Базовый layout отображается

---

## Задача A2: Tailwind CSS для админки

### Контекст
Админка использует нейтральный, функциональный дизайн — НЕ художественный стиль основного приложения.

### Цель
Настроить Tailwind с чистой, профессиональной темой.

### tailwind.config.js
```javascript
/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      colors: {
        // Нейтральная палитра для админки
        'admin': {
          'bg': '#1a1a1a',
          'surface': '#2a2a2a',
          'border': '#3a3a3a',
          'text': '#e5e5e5',
          'muted': '#888888',
          'accent': '#6366f1',  // Indigo
          'success': '#22c55e',
          'warning': '#f59e0b',
          'danger': '#ef4444',
        }
      },
      fontFamily: {
        'sans': ['Inter', 'system-ui', 'sans-serif'],
        'mono': ['JetBrains Mono', 'monospace'],
      },
    },
  },
  plugins: [],
}
```

### src/app.css
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono&display=swap');

@layer base {
  html {
    @apply bg-admin-bg text-admin-text antialiased;
  }

  body {
    @apply font-sans text-sm min-h-screen;
  }
}

@layer components {
  /* Кнопки */
  .btn {
    @apply px-4 py-2 rounded-md font-medium transition-colors duration-200
           focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-admin-bg;
  }

  .btn-primary {
    @apply btn bg-admin-accent text-white hover:bg-admin-accent/90
           focus:ring-admin-accent;
  }

  .btn-secondary {
    @apply btn bg-admin-surface border border-admin-border text-admin-text
           hover:bg-admin-border focus:ring-admin-border;
  }

  .btn-danger {
    @apply btn bg-admin-danger text-white hover:bg-admin-danger/90
           focus:ring-admin-danger;
  }

  .btn-ghost {
    @apply btn bg-transparent text-admin-muted hover:text-admin-text
           hover:bg-admin-surface;
  }

  .btn-sm {
    @apply px-3 py-1.5 text-xs;
  }

  /* Инпуты */
  .input {
    @apply w-full px-3 py-2 bg-admin-surface border border-admin-border rounded-md
           text-admin-text placeholder-admin-muted
           focus:outline-none focus:ring-2 focus:ring-admin-accent focus:border-transparent
           transition-colors duration-200;
  }

  .textarea {
    @apply input min-h-[120px] resize-y;
  }

  .select {
    @apply input appearance-none cursor-pointer;
  }

  .label {
    @apply block text-sm font-medium text-admin-muted mb-1.5;
  }

  /* Карточки */
  .card {
    @apply bg-admin-surface border border-admin-border rounded-lg;
  }

  .card-header {
    @apply px-4 py-3 border-b border-admin-border;
  }

  .card-body {
    @apply p-4;
  }

  /* Таблицы */
  .table {
    @apply w-full text-left;
  }

  .table th {
    @apply px-4 py-3 text-xs font-medium text-admin-muted uppercase tracking-wider
           bg-admin-bg border-b border-admin-border;
  }

  .table td {
    @apply px-4 py-3 border-b border-admin-border;
  }

  .table tr:hover td {
    @apply bg-admin-bg/50;
  }

  /* Badges */
  .badge {
    @apply inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium;
  }

  .badge-success {
    @apply badge bg-admin-success/20 text-admin-success;
  }

  .badge-warning {
    @apply badge bg-admin-warning/20 text-admin-warning;
  }

  .badge-danger {
    @apply badge bg-admin-danger/20 text-admin-danger;
  }

  /* Sidebar */
  .sidebar {
    @apply w-64 bg-admin-surface border-r border-admin-border min-h-screen;
  }

  .sidebar-item {
    @apply flex items-center gap-3 px-4 py-2.5 text-admin-muted
           hover:text-admin-text hover:bg-admin-bg transition-colors;
  }

  .sidebar-item.active {
    @apply text-admin-text bg-admin-bg border-l-2 border-admin-accent;
  }
}
```

### Критерии готовности
- [ ] Tailwind настроен
- [ ] Компоненты стилизованы
- [ ] UI выглядит профессионально и функционально

---

## Задача A3: Layout и навигация админки

### Контекст
Админка имеет стандартную структуру: сайдбар + основной контент.

### Цель
Создать базовый layout с навигацией.

### src/routes/+layout.svelte
```svelte
<script lang="ts">
  import '../app.css';
  import { page } from '$app/stores';

  const navItems = [
    { href: '/', label: 'Dashboard', icon: '📊' },
    { href: '/figurines', label: 'Фигуры', icon: '🎭' },
    { href: '/texts/author', label: 'Тексты автора', icon: '✍️' },
    { href: '/texts/workshop', label: 'Мастерская', icon: '🔧' },
    { href: '/zones', label: 'Зоны кабинета', icon: '🗺️' },
    { href: '/preview', label: 'Предпросмотр', icon: '👁️' },
  ];

  $: currentPath = $page.url.pathname;

  function isActive(href: string): boolean {
    if (href === '/') return currentPath === '/';
    return currentPath.startsWith(href);
  }
</script>

<div class="flex min-h-screen">
  <!-- Sidebar -->
  <aside class="sidebar flex flex-col">
    <!-- Logo -->
    <div class="px-4 py-6 border-b border-admin-border">
      <h1 class="text-lg font-semibold text-admin-text">Cabinet Admin</h1>
      <p class="text-xs text-admin-muted mt-1">Управление контентом</p>
    </div>

    <!-- Navigation -->
    <nav class="flex-1 py-4">
      {#each navItems as item}
        <a
          href={item.href}
          class="sidebar-item"
          class:active={isActive(item.href)}
        >
          <span class="text-lg">{item.icon}</span>
          <span>{item.label}</span>
        </a>
      {/each}
    </nav>

    <!-- Footer -->
    <div class="px-4 py-4 border-t border-admin-border">
      <p class="text-xs text-admin-muted">
        Cabinet of Curiosities v0.1.0
      </p>
    </div>
  </aside>

  <!-- Main content -->
  <main class="flex-1 overflow-auto">
    <slot />
  </main>
</div>
```

### src/routes/+page.svelte (Dashboard)
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';

  let stats = {
    figurines: 0,
    available: 0,
    sold: 0,
    authorTexts: 0,
    workshopItems: 0,
  };

  onMount(async () => {
    stats = await api.getDashboardStats();
  });
</script>

<div class="p-6">
  <h1 class="text-2xl font-semibold mb-6">Dashboard</h1>

  <!-- Stats Grid -->
  <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
    <div class="card card-body">
      <p class="text-admin-muted text-sm">Всего фигур</p>
      <p class="text-3xl font-semibold mt-1">{stats.figurines}</p>
    </div>

    <div class="card card-body">
      <p class="text-admin-muted text-sm">Доступно</p>
      <p class="text-3xl font-semibold text-admin-success mt-1">{stats.available}</p>
    </div>

    <div class="card card-body">
      <p class="text-admin-muted text-sm">Продано</p>
      <p class="text-3xl font-semibold text-admin-warning mt-1">{stats.sold}</p>
    </div>

    <div class="card card-body">
      <p class="text-admin-muted text-sm">Текстов автора</p>
      <p class="text-3xl font-semibold mt-1">{stats.authorTexts}</p>
    </div>
  </div>

  <!-- Quick Actions -->
  <div class="card">
    <div class="card-header">
      <h2 class="font-medium">Быстрые действия</h2>
    </div>
    <div class="card-body flex flex-wrap gap-3">
      <a href="/figurines/new" class="btn btn-primary">
        + Добавить фигуру
      </a>
      <a href="/texts/author" class="btn btn-secondary">
        Редактировать тексты
      </a>
      <a href="/preview" class="btn btn-secondary">
        Открыть предпросмотр
      </a>
    </div>
  </div>
</div>
```

### Критерии готовности
- [ ] Сайдбар отображается
- [ ] Навигация работает
- [ ] Активный пункт подсвечивается
- [ ] Dashboard показывает статистику

---

## Задача A4: CRUD команды для фигур (Backend)

### Контекст
Нужны Tauri-команды для полного управления фигурами.

### Цель
Реализовать Create, Read, Update, Delete операции.

### src-tauri/src/commands/figurines.rs
```rust
use tauri::State;
use uuid::Uuid;
use crate::db::Database;
use crate::models::*;

// ==================== CREATE ====================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFigurineInput {
    pub name: String,
    pub short_text: Option<String>,
    pub year: Option<i32>,
    pub status: String,
}

#[tauri::command]
pub async fn create_figurine(
    input: CreateFigurineInput,
    db: State<'_, Database>
) -> Result<FigurineDto, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let id = Uuid::new_v4().to_string();

    // Получить максимальный sort_order
    let max_order: i32 = conn
        .query_row("SELECT COALESCE(MAX(sort_order), 0) FROM figurines", [], |row| row.get(0))
        .unwrap_or(0);

    conn.execute(
        "INSERT INTO figurines (id, name, short_text, year, status, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            id,
            input.name,
            input.short_text,
            input.year,
            input.status,
            max_order + 1
        ],
    ).map_err(|e| format!("Failed to create figurine: {}", e))?;

    // Вернуть созданную фигуру
    Ok(FigurineDto {
        id,
        name: input.name,
        short_text: input.short_text,
        year: input.year,
        status: input.status,
        images: vec![],
    })
}

// ==================== READ ====================

#[tauri::command]
pub async fn get_figurine_for_edit(
    id: String,
    db: State<'_, Database>
) -> Result<Option<FigurineEditDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let figurine = conn.query_row(
        "SELECT id, name, short_text, year, status, sort_order
         FROM figurines WHERE id = ?",
        [&id],
        |row| Ok(FigurineEditDto {
            id: row.get(0)?,
            name: row.get(1)?,
            short_text: row.get(2)?,
            year: row.get(3)?,
            status: row.get(4)?,
            sort_order: row.get(5)?,
        })
    ).optional().map_err(|e| e.to_string())?;

    Ok(figurine)
}

#[tauri::command]
pub async fn get_all_figurines_admin(
    db: State<'_, Database>
) -> Result<Vec<FigurineListDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT f.id, f.name, f.status, f.year, f.sort_order,
                (SELECT COUNT(*) FROM images WHERE figurine_id = f.id) as image_count
         FROM figurines f
         ORDER BY f.sort_order"
    ).map_err(|e| e.to_string())?;

    let figurines = stmt.query_map([], |row| {
        Ok(FigurineListDto {
            id: row.get(0)?,
            name: row.get(1)?,
            status: row.get(2)?,
            year: row.get(3)?,
            sort_order: row.get(4)?,
            image_count: row.get(5)?,
        })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect();

    Ok(figurines)
}

// ==================== UPDATE ====================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFigurineInput {
    pub id: String,
    pub name: String,
    pub short_text: Option<String>,
    pub year: Option<i32>,
    pub status: String,
}

#[tauri::command]
pub async fn update_figurine(
    input: UpdateFigurineInput,
    db: State<'_, Database>
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let affected = conn.execute(
        "UPDATE figurines
         SET name = ?2, short_text = ?3, year = ?4, status = ?5
         WHERE id = ?1",
        rusqlite::params![
            input.id,
            input.name,
            input.short_text,
            input.year,
            input.status
        ],
    ).map_err(|e| format!("Failed to update: {}", e))?;

    if affected == 0 {
        return Err("Figurine not found".to_string());
    }

    Ok(())
}

#[tauri::command]
pub async fn reorder_figurines(
    ids: Vec<String>,
    db: State<'_, Database>
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    for (index, id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE figurines SET sort_order = ?1 WHERE id = ?2",
            rusqlite::params![index as i32 + 1, id],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

// ==================== DELETE ====================

#[tauri::command]
pub async fn delete_figurine(
    id: String,
    db: State<'_, Database>
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Удаление каскадное — изображения удалятся автоматически
    let affected = conn.execute(
        "DELETE FROM figurines WHERE id = ?",
        [&id],
    ).map_err(|e| e.to_string())?;

    if affected == 0 {
        return Err("Figurine not found".to_string());
    }

    Ok(())
}
```

### DTO для админки (добавить в models)
```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FigurineEditDto {
    pub id: String,
    pub name: String,
    pub short_text: Option<String>,
    pub year: Option<i32>,
    pub status: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FigurineListDto {
    pub id: String,
    pub name: String,
    pub status: String,
    pub year: Option<i32>,
    pub sort_order: i32,
    pub image_count: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub figurines: i32,
    pub available: i32,
    pub sold: i32,
    pub reserved: i32,
    pub author_texts: i32,
    pub workshop_items: i32,
}
```

### Критерии готовности
- [ ] Создание фигуры работает
- [ ] Редактирование сохраняется
- [ ] Удаление работает (с каскадом)
- [ ] Сортировка drag-n-drop готова

---

## Задача A5: UI списка и редактирования фигур

### Контекст
Нужен удобный интерфейс для управления фигурами.

### Цель
Создать список с действиями и форму редактирования.

### src/routes/figurines/+page.svelte
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import type { FigurineListItem } from '$lib/types';

  let figurines: FigurineListItem[] = [];
  let loading = true;
  let deleteConfirm: string | null = null;

  onMount(async () => {
    await loadFigurines();
  });

  async function loadFigurines() {
    loading = true;
    figurines = await api.getAllFigurinesAdmin();
    loading = false;
  }

  async function handleDelete(id: string) {
    if (deleteConfirm === id) {
      await api.deleteFigurine(id);
      deleteConfirm = null;
      await loadFigurines();
    } else {
      deleteConfirm = id;
      // Сбросить через 3 секунды
      setTimeout(() => {
        if (deleteConfirm === id) deleteConfirm = null;
      }, 3000);
    }
  }

  function getStatusBadge(status: string) {
    switch (status) {
      case 'available': return 'badge-success';
      case 'sold': return 'badge-warning';
      case 'reserved': return 'badge-danger';
      default: return '';
    }
  }

  function getStatusLabel(status: string) {
    switch (status) {
      case 'available': return 'Доступна';
      case 'sold': return 'Продана';
      case 'reserved': return 'Резерв';
      default: return status;
    }
  }
</script>

<div class="p-6">
  <!-- Header -->
  <div class="flex items-center justify-between mb-6">
    <h1 class="text-2xl font-semibold">Фигуры</h1>
    <a href="/figurines/new" class="btn btn-primary">
      + Добавить фигуру
    </a>
  </div>

  <!-- Table -->
  <div class="card">
    {#if loading}
      <div class="p-8 text-center text-admin-muted">
        Загрузка...
      </div>
    {:else if figurines.length === 0}
      <div class="p-8 text-center">
        <p class="text-admin-muted mb-4">Фигур пока нет</p>
        <a href="/figurines/new" class="btn btn-primary">
          Добавить первую
        </a>
      </div>
    {:else}
      <table class="table">
        <thead>
          <tr>
            <th class="w-12">#</th>
            <th>Название</th>
            <th class="w-24">Год</th>
            <th class="w-32">Статус</th>
            <th class="w-24">Фото</th>
            <th class="w-48">Действия</th>
          </tr>
        </thead>
        <tbody>
          {#each figurines as fig, i}
            <tr>
              <td class="text-admin-muted">{i + 1}</td>
              <td>
                <a
                  href="/figurines/{fig.id}"
                  class="text-admin-accent hover:underline font-medium"
                >
                  {fig.name}
                </a>
              </td>
              <td class="text-admin-muted">{fig.year ?? '—'}</td>
              <td>
                <span class={getStatusBadge(fig.status)}>
                  {getStatusLabel(fig.status)}
                </span>
              </td>
              <td class="text-admin-muted">{fig.imageCount}</td>
              <td>
                <div class="flex gap-2">
                  <a
                    href="/figurines/{fig.id}"
                    class="btn btn-ghost btn-sm"
                  >
                    Редактировать
                  </a>
                  <button
                    class="btn btn-sm"
                    class:btn-danger={deleteConfirm === fig.id}
                    class:btn-ghost={deleteConfirm !== fig.id}
                    on:click={() => handleDelete(fig.id)}
                  >
                    {deleteConfirm === fig.id ? 'Подтвердить?' : 'Удалить'}
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>
```

### src/routes/figurines/[id]/+page.svelte (Редактирование)
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { api } from '$lib/api';
  import ImageUploader from '$lib/components/ImageUploader.svelte';

  let figurine = {
    id: '',
    name: '',
    shortText: '',
    year: null as number | null,
    status: 'available',
  };

  let images: any[] = [];
  let loading = true;
  let saving = false;
  let error = '';

  $: id = $page.params.id;
  $: isNew = id === 'new';

  onMount(async () => {
    if (!isNew) {
      const data = await api.getFigurineForEdit(id);
      if (data) {
        figurine = {
          id: data.id,
          name: data.name,
          shortText: data.shortText ?? '',
          year: data.year,
          status: data.status,
        };
        images = await api.getImagesForFigurine(id);
      }
    }
    loading = false;
  });

  async function handleSubmit() {
    if (!figurine.name.trim()) {
      error = 'Название обязательно';
      return;
    }

    saving = true;
    error = '';

    try {
      if (isNew) {
        const created = await api.createFigurine({
          name: figurine.name,
          shortText: figurine.shortText || null,
          year: figurine.year,
          status: figurine.status,
        });
        goto(`/figurines/${created.id}`);
      } else {
        await api.updateFigurine({
          id: figurine.id,
          name: figurine.name,
          shortText: figurine.shortText || null,
          year: figurine.year,
          status: figurine.status,
        });
      }
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function handleImagesChange() {
    images = await api.getImagesForFigurine(id);
  }
</script>

<div class="p-6 max-w-4xl">
  <!-- Header -->
  <div class="flex items-center gap-4 mb-6">
    <a href="/figurines" class="btn btn-ghost">
      ← Назад
    </a>
    <h1 class="text-2xl font-semibold">
      {isNew ? 'Новая фигура' : 'Редактирование'}
    </h1>
  </div>

  {#if loading}
    <div class="card card-body text-center text-admin-muted">
      Загрузка...
    </div>
  {:else}
    <div class="grid lg:grid-cols-2 gap-6">
      <!-- Форма -->
      <div class="card">
        <div class="card-header">
          <h2 class="font-medium">Основная информация</h2>
        </div>
        <form class="card-body space-y-4" on:submit|preventDefault={handleSubmit}>
          {#if error}
            <div class="p-3 bg-admin-danger/20 text-admin-danger rounded-md text-sm">
              {error}
            </div>
          {/if}

          <div>
            <label class="label" for="name">Название *</label>
            <input
              id="name"
              type="text"
              class="input"
              bind:value={figurine.name}
              placeholder="Хранительница порога"
            />
          </div>

          <div>
            <label class="label" for="shortText">
              Текст персонажа
              <span class="text-admin-muted font-normal">(атмосферный, не описание!)</span>
            </label>
            <textarea
              id="shortText"
              class="textarea"
              bind:value={figurine.shortText}
              placeholder="Она стоит там, где заканчивается один дом и начинается другой..."
              rows="4"
            />
            <p class="text-xs text-admin-muted mt-1">
              Фрагмент истории, не маркетинг. 1-3 предложения.
            </p>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="label" for="year">Год создания</label>
              <input
                id="year"
                type="number"
                class="input"
                bind:value={figurine.year}
                placeholder="2024"
                min="2000"
                max="2100"
              />
            </div>

            <div>
              <label class="label" for="status">Статус</label>
              <select id="status" class="select" bind:value={figurine.status}>
                <option value="available">Доступна</option>
                <option value="reserved">Зарезервирована</option>
                <option value="sold">Продана</option>
              </select>
            </div>
          </div>

          <div class="pt-4 flex gap-3">
            <button
              type="submit"
              class="btn btn-primary"
              disabled={saving}
            >
              {saving ? 'Сохранение...' : 'Сохранить'}
            </button>
            <a href="/figurines" class="btn btn-secondary">
              Отмена
            </a>
          </div>
        </form>
      </div>

      <!-- Изображения (только для существующей фигуры) -->
      {#if !isNew}
        <div class="card">
          <div class="card-header">
            <h2 class="font-medium">Изображения</h2>
          </div>
          <div class="card-body">
            <ImageUploader
              figurineId={figurine.id}
              {images}
              on:change={handleImagesChange}
            />
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
```

### Критерии готовности
- [ ] Список фигур отображается
- [ ] Создание новой фигуры работает
- [ ] Редактирование сохраняется
- [ ] Удаление с подтверждением работает

---

## Задача A6: Загрузка и управление изображениями

### Контекст
Нужно загружать, сортировать и удалять изображения фигур.

### Цель
Создать компонент загрузки изображений с drag-n-drop.

### src/lib/components/ImageUploader.svelte
```svelte
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { api } from '$lib/api';

  export let figurineId: string;
  export let images: Array<{
    id: string;
    imageType: string;
    url: string;
    altText: string | null;
  }> = [];

  const dispatch = createEventDispatcher();

  let uploading = false;
  let dragOver = false;

  const imageTypes = [
    { value: 'face', label: 'Лицо (главное)' },
    { value: 'detail', label: 'Деталь' },
    { value: 'full', label: 'Полный вид' },
  ];

  async function handleSelectFile() {
    const selected = await open({
      multiple: true,
      filters: [{
        name: 'Images',
        extensions: ['jpg', 'jpeg', 'png', 'webp']
      }]
    });

    if (selected) {
      const files = Array.isArray(selected) ? selected : [selected];
      await uploadFiles(files);
    }
  }

  async function uploadFiles(paths: string[]) {
    uploading = true;

    for (const path of paths) {
      try {
        await api.uploadImage({
          figurineId,
          filePath: path,
          imageType: 'detail', // По умолчанию
        });
      } catch (e) {
        console.error('Upload failed:', e);
      }
    }

    uploading = false;
    dispatch('change');
  }

  async function handleDelete(imageId: string) {
    if (confirm('Удалить изображение?')) {
      await api.deleteImage(imageId);
      dispatch('change');
    }
  }

  async function handleTypeChange(imageId: string, newType: string) {
    await api.updateImageType(imageId, newType);
    dispatch('change');
  }

  function getTypeLabel(type: string): string {
    return imageTypes.find(t => t.value === type)?.label ?? type;
  }
</script>

<div class="space-y-4">
  <!-- Список изображений -->
  {#if images.length > 0}
    <div class="grid grid-cols-2 gap-4">
      {#each images as image}
        <div class="relative group bg-admin-bg rounded-lg overflow-hidden">
          <img
            src={image.url}
            alt={image.altText ?? ''}
            class="w-full aspect-square object-cover"
          />

          <!-- Overlay с действиями -->
          <div class="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100
                      transition-opacity flex flex-col justify-between p-3">
            <div class="flex justify-end">
              <button
                class="w-8 h-8 bg-admin-danger rounded-full text-white
                       flex items-center justify-center hover:bg-admin-danger/80"
                on:click={() => handleDelete(image.id)}
              >
                ×
              </button>
            </div>

            <div>
              <select
                class="select text-xs"
                value={image.imageType}
                on:change={(e) => handleTypeChange(image.id, e.currentTarget.value)}
              >
                {#each imageTypes as type}
                  <option value={type.value}>{type.label}</option>
                {/each}
              </select>
            </div>
          </div>

          <!-- Badge типа -->
          <div class="absolute top-2 left-2">
            <span class="badge bg-admin-bg/80 text-admin-text text-xs">
              {getTypeLabel(image.imageType)}
            </span>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Drop zone -->
  <button
    class="w-full border-2 border-dashed rounded-lg p-8 text-center
           transition-colors cursor-pointer"
    class:border-admin-border={!dragOver}
    class:border-admin-accent={dragOver}
    class:bg-admin-accent/10={dragOver}
    on:click={handleSelectFile}
    on:dragover|preventDefault={() => dragOver = true}
    on:dragleave={() => dragOver = false}
    on:drop|preventDefault={() => dragOver = false}
  >
    {#if uploading}
      <p class="text-admin-muted">Загрузка...</p>
    {:else}
      <p class="text-admin-muted mb-2">
        Перетащите изображения сюда
      </p>
      <p class="text-xs text-admin-muted">
        или нажмите для выбора файлов
      </p>
    {/if}
  </button>

  <p class="text-xs text-admin-muted">
    Поддерживаются: JPG, PNG, WebP. Рекомендуемый размер: 1200×1600px
  </p>
</div>
```

### Backend команды для изображений
```rust
// src-tauri/src/commands/images.rs

use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;
use crate::db::Database;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadImageInput {
    pub figurine_id: String,
    pub file_path: String,
    pub image_type: String,
}

#[tauri::command]
pub async fn upload_image(
    input: UploadImageInput,
    app: tauri::AppHandle,
    db: State<'_, Database>
) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Генерируем ID и путь назначения
    let id = Uuid::new_v4().to_string();
    let source = PathBuf::from(&input.file_path);

    let extension = source.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg");

    // Путь: images/figurines/{figurine_id}/{image_type}_{id}.{ext}
    let relative_path = format!(
        "images/figurines/{}/{}_{}.{}",
        input.figurine_id,
        input.image_type,
        &id[..8],
        extension
    );

    // Получаем абсолютный путь к static директории
    let static_dir = app.path().resource_dir()
        .map_err(|e| e.to_string())?
        .join("static");

    let dest_path = static_dir.join(&relative_path);

    // Создаём директорию
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Копируем и оптимизируем изображение
    let img = image::open(&source).map_err(|e| e.to_string())?;

    // Ресайз если слишком большое
    let img = if img.width() > 2000 || img.height() > 2000 {
        img.resize(2000, 2000, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    img.save(&dest_path).map_err(|e| e.to_string())?;

    // Получаем максимальный sort_order для этой фигуры
    let max_order: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM images WHERE figurine_id = ?",
            [&input.figurine_id],
            |row| row.get(0)
        )
        .unwrap_or(0);

    // Сохраняем в БД
    conn.execute(
        "INSERT INTO images (id, figurine_id, image_type, file_path, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            id,
            input.figurine_id,
            input.image_type,
            relative_path,
            max_order + 1
        ],
    ).map_err(|e| e.to_string())?;

    Ok(id)
}

#[tauri::command]
pub async fn delete_image(
    id: String,
    app: tauri::AppHandle,
    db: State<'_, Database>
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Получаем путь к файлу
    let file_path: String = conn.query_row(
        "SELECT file_path FROM images WHERE id = ?",
        [&id],
        |row| row.get(0)
    ).map_err(|e| e.to_string())?;

    // Удаляем файл
    let static_dir = app.path().resource_dir()
        .map_err(|e| e.to_string())?
        .join("static");

    let full_path = static_dir.join(&file_path);
    std::fs::remove_file(&full_path).ok(); // Игнорируем ошибку если файла нет

    // Удаляем из БД
    conn.execute("DELETE FROM images WHERE id = ?", [&id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_image_type(
    id: String,
    image_type: String,
    db: State<'_, Database>
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE images SET image_type = ?2 WHERE id = ?1",
        rusqlite::params![id, image_type],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_images_for_figurine(
    figurine_id: String,
    db: State<'_, Database>
) -> Result<Vec<ImageDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT id, figurine_id, image_type, file_path, alt_text
         FROM images
         WHERE figurine_id = ?
         ORDER BY sort_order"
    ).map_err(|e| e.to_string())?;

    let images = stmt.query_map([&figurine_id], |row| {
        let file_path: String = row.get(3)?;
        Ok(ImageDto {
            id: row.get(0)?,
            image_type: row.get(2)?,
            url: format!("asset://localhost/{}", file_path),
            alt_text: row.get(4)?,
        })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect();

    Ok(images)
}
```

### Критерии готовности
- [ ] Выбор файлов через диалог работает
- [ ] Изображения копируются в static
- [ ] Изображения оптимизируются (resize)
- [ ] Тип изображения можно менять
- [ ] Удаление работает (файл + БД)

---

## Задача A7: Редактирование текстов

### Контекст
Нужны страницы для редактирования авторских текстов и контента мастерской.

### src/routes/texts/author/+page.svelte
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';

  let texts: Array<{ id: string; content: string }> = [];
  let newText = '';
  let loading = true;
  let saving = false;

  onMount(async () => {
    texts = await api.getAuthorTexts();
    loading = false;
  });

  async function handleAdd() {
    if (!newText.trim()) return;

    saving = true;
    await api.createAuthorText(newText.trim());
    newText = '';
    texts = await api.getAuthorTexts();
    saving = false;
  }

  async function handleUpdate(id: string, content: string) {
    await api.updateText(id, content);
  }

  async function handleDelete(id: string) {
    if (confirm('Удалить текст?')) {
      await api.deleteText(id);
      texts = await api.getAuthorTexts();
    }
  }
</script>

<div class="p-6 max-w-3xl">
  <h1 class="text-2xl font-semibold mb-2">Тексты автора</h1>
  <p class="text-admin-muted mb-6">
    Короткие обрывочные мысли, создающие присутствие художника.
    Не биография, не маркетинг — атмосфера.
  </p>

  {#if loading}
    <div class="card card-body text-center text-admin-muted">
      Загрузка...
    </div>
  {:else}
    <!-- Существующие тексты -->
    <div class="space-y-4 mb-6">
      {#each texts as text, i}
        <div class="card">
          <div class="card-body">
            <div class="flex items-start gap-4">
              <span class="text-admin-muted text-sm w-6">{i + 1}.</span>
              <textarea
                class="textarea flex-1"
                rows="2"
                value={text.content}
                on:blur={(e) => handleUpdate(text.id, e.currentTarget.value)}
                placeholder="Текст..."
              />
              <button
                class="btn btn-ghost btn-sm text-admin-danger"
                on:click={() => handleDelete(text.id)}
              >
                ×
              </button>
            </div>
          </div>
        </div>
      {/each}
    </div>

    <!-- Добавление нового -->
    <div class="card">
      <div class="card-header">
        <h2 class="font-medium">Добавить текст</h2>
      </div>
      <div class="card-body">
        <textarea
          class="textarea mb-4"
          rows="3"
          bind:value={newText}
          placeholder="«Я оставляю трещины. Они говорят больше, чем гладкая поверхность.»"
        />
        <button
          class="btn btn-primary"
          disabled={saving || !newText.trim()}
          on:click={handleAdd}
        >
          {saving ? 'Добавление...' : 'Добавить'}
        </button>
      </div>
    </div>

    <!-- Подсказки -->
    <div class="mt-6 p-4 bg-admin-surface rounded-lg border border-admin-border">
      <h3 class="text-sm font-medium mb-2">Примеры хороших текстов:</h3>
      <ul class="text-sm text-admin-muted space-y-1">
        <li>• «Каждая фигура помнит руки, которые её создали.»</li>
        <li>• «Пыль — это не грязь. Это время, которое осело.»</li>
        <li>• «Я не делаю кукол. Куклы улыбаются.»</li>
      </ul>
    </div>
  {/if}
</div>
```

### src/routes/texts/workshop/+page.svelte
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { api } from '$lib/api';

  interface WorkshopItem {
    id: string;
    content: string;
    caption: string | null;
    imageUrl: string | null;
  }

  let items: WorkshopItem[] = [];
  let loading = true;
  let editingId: string | null = null;

  let newItem = {
    content: '',
    caption: '',
    imagePath: '',
  };

  onMount(async () => {
    items = await api.getWorkshopContent();
    loading = false;
  });

  async function handleSelectImage() {
    const selected = await open({
      filters: [{
        name: 'Images',
        extensions: ['jpg', 'jpeg', 'png', 'webp']
      }]
    });

    if (selected && typeof selected === 'string') {
      newItem.imagePath = selected;
    }
  }

  async function handleAdd() {
    if (!newItem.caption.trim()) return;

    await api.createWorkshopItem({
      content: newItem.content,
      caption: newItem.caption,
      imagePath: newItem.imagePath || null,
    });

    newItem = { content: '', caption: '', imagePath: '' };
    items = await api.getWorkshopContent();
  }

  async function handleDelete(id: string) {
    if (confirm('Удалить элемент мастерской?')) {
      await api.deleteWorkshopItem(id);
      items = await api.getWorkshopContent();
    }
  }
</script>

<div class="p-6">
  <h1 class="text-2xl font-semibold mb-2">Мастерская</h1>
  <p class="text-admin-muted mb-6">
    Фотографии процесса с короткими подписями.
    Показывают ручную работу и этапы создания.
  </p>

  {#if loading}
    <div class="card card-body text-center text-admin-muted">
      Загрузка...
    </div>
  {:else}
    <div class="grid lg:grid-cols-2 gap-6">
      <!-- Существующие элементы -->
      <div class="space-y-4">
        <h2 class="font-medium text-admin-muted">Элементы ({items.length})</h2>

        {#each items as item}
          <div class="card">
            <div class="card-body flex gap-4">
              {#if item.imageUrl}
                <img
                  src={item.imageUrl}
                  alt=""
                  class="w-24 h-24 object-cover rounded"
                />
              {:else}
                <div class="w-24 h-24 bg-admin-bg rounded flex items-center justify-center text-admin-muted">
                  Нет фото
                </div>
              {/if}

              <div class="flex-1">
                <p class="text-sm mb-1">{item.caption || 'Без подписи'}</p>
                <p class="text-xs text-admin-muted">{item.content}</p>
              </div>

              <button
                class="btn btn-ghost btn-sm text-admin-danger self-start"
                on:click={() => handleDelete(item.id)}
              >
                ×
              </button>
            </div>
          </div>
        {/each}
      </div>

      <!-- Форма добавления -->
      <div class="card h-fit">
        <div class="card-header">
          <h2 class="font-medium">Добавить элемент</h2>
        </div>
        <div class="card-body space-y-4">
          <div>
            <label class="label">Заголовок</label>
            <input
              type="text"
              class="input"
              bind:value={newItem.content}
              placeholder="Начало"
            />
          </div>

          <div>
            <label class="label">Подпись *</label>
            <textarea
              class="textarea"
              rows="2"
              bind:value={newItem.caption}
              placeholder="Первые наброски. Лицо ещё не решило, кем быть."
            />
          </div>

          <div>
            <label class="label">Изображение</label>
            <div class="flex gap-2">
              <input
                type="text"
                class="input flex-1"
                value={newItem.imagePath}
                placeholder="Путь к файлу..."
                readonly
              />
              <button
                class="btn btn-secondary"
                on:click={handleSelectImage}
              >
                Выбрать
              </button>
            </div>
          </div>

          <button
            class="btn btn-primary w-full"
            disabled={!newItem.caption.trim()}
            on:click={handleAdd}
          >
            Добавить
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>
```

### Критерии готовности
- [ ] Авторские тексты: добавление, редактирование, удаление
- [ ] Мастерская: управление элементами с изображениями
- [ ] Inline-редактирование (blur save)
- [ ] Подсказки по формату контента

---

## Задача A8: Настройка зон кабинета

### Контекст
Нужен визуальный редактор интерактивных зон на главном изображении.

### src/routes/zones/+page.svelte
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';

  interface Zone {
    id: string;
    zoneType: string;
    x: number;
    y: number;
    width: number;
    height: number;
    targetRoute: string;
  }

  let zones: Zone[] = [];
  let selectedZone: Zone | null = null;
  let imageLoaded = false;

  const zoneTypes = [
    { value: 'showcase', label: 'Витрина', route: '/figurines' },
    { value: 'desk', label: 'Стол', route: '/workshop' },
    { value: 'shelf', label: 'Полка', route: '/figurines' },
    { value: 'note', label: 'Записка', route: '/author' },
  ];

  onMount(async () => {
    zones = await api.getCabinetZones();
  });

  async function handleZoneUpdate(zone: Zone) {
    await api.updateCabinetZone(zone);
    zones = await api.getCabinetZones();
  }

  async function handleAddZone() {
    const newZone = await api.createCabinetZone({
      zoneType: 'showcase',
      x: 10,
      y: 10,
      width: 20,
      height: 20,
      targetRoute: '/figurines',
    });
    zones = await api.getCabinetZones();
    selectedZone = zones.find(z => z.id === newZone.id) ?? null;
  }

  async function handleDeleteZone(id: string) {
    if (confirm('Удалить зону?')) {
      await api.deleteCabinetZone(id);
      zones = await api.getCabinetZones();
      if (selectedZone?.id === id) {
        selectedZone = null;
      }
    }
  }

  function getZoneColor(type: string): string {
    const colors: Record<string, string> = {
      showcase: 'border-blue-500 bg-blue-500/20',
      desk: 'border-green-500 bg-green-500/20',
      shelf: 'border-yellow-500 bg-yellow-500/20',
      note: 'border-purple-500 bg-purple-500/20',
    };
    return colors[type] ?? 'border-gray-500 bg-gray-500/20';
  }
</script>

<div class="p-6">
  <div class="flex items-center justify-between mb-6">
    <div>
      <h1 class="text-2xl font-semibold">Зоны кабинета</h1>
      <p class="text-admin-muted text-sm mt-1">
        Настройте интерактивные области на главном изображении
      </p>
    </div>
    <button class="btn btn-primary" on:click={handleAddZone}>
      + Добавить зону
    </button>
  </div>

  <div class="grid lg:grid-cols-3 gap-6">
    <!-- Визуальный редактор -->
    <div class="lg:col-span-2">
      <div class="card">
        <div class="card-body p-0">
          <div class="relative">
            <img
              src="/images/cabinet-room.jpg"
              alt="Cabinet Room"
              class="w-full"
              on:load={() => imageLoaded = true}
            />

            {#if imageLoaded}
              {#each zones as zone}
                <button
                  class="absolute border-2 transition-all cursor-pointer {getZoneColor(zone.zoneType)}"
                  class:ring-2={selectedZone?.id === zone.id}
                  class:ring-white={selectedZone?.id === zone.id}
                  style="
                    left: {zone.x}%;
                    top: {zone.y}%;
                    width: {zone.width}%;
                    height: {zone.height}%;
                  "
                  on:click={() => selectedZone = zone}
                >
                  <span class="absolute top-1 left-1 text-xs bg-black/50 text-white px-1 rounded">
                    {zone.zoneType}
                  </span>
                </button>
              {/each}
            {/if}
          </div>
        </div>
      </div>
    </div>

    <!-- Панель редактирования -->
    <div>
      {#if selectedZone}
        <div class="card">
          <div class="card-header flex items-center justify-between">
            <h2 class="font-medium">Редактирование зоны</h2>
            <button
              class="btn btn-ghost btn-sm text-admin-danger"
              on:click={() => handleDeleteZone(selectedZone.id)}
            >
              Удалить
            </button>
          </div>
          <div class="card-body space-y-4">
            <div>
              <label class="label">Тип зоны</label>
              <select
                class="select"
                bind:value={selectedZone.zoneType}
                on:change={() => handleZoneUpdate(selectedZone)}
              >
                {#each zoneTypes as type}
                  <option value={type.value}>{type.label}</option>
                {/each}
              </select>
            </div>

            <div>
              <label class="label">Ссылка</label>
              <input
                type="text"
                class="input"
                bind:value={selectedZone.targetRoute}
                on:blur={() => handleZoneUpdate(selectedZone)}
              />
            </div>

            <div class="grid grid-cols-2 gap-4">
              <div>
                <label class="label">X (%)</label>
                <input
                  type="number"
                  class="input"
                  bind:value={selectedZone.x}
                  on:blur={() => handleZoneUpdate(selectedZone)}
                  min="0"
                  max="100"
                />
              </div>
              <div>
                <label class="label">Y (%)</label>
                <input
                  type="number"
                  class="input"
                  bind:value={selectedZone.y}
                  on:blur={() => handleZoneUpdate(selectedZone)}
                  min="0"
                  max="100"
                />
              </div>
              <div>
                <label class="label">Ширина (%)</label>
                <input
                  type="number"
                  class="input"
                  bind:value={selectedZone.width}
                  on:blur={() => handleZoneUpdate(selectedZone)}
                  min="1"
                  max="100"
                />
              </div>
              <div>
                <label class="label">Высота (%)</label>
                <input
                  type="number"
                  class="input"
                  bind:value={selectedZone.height}
                  on:blur={() => handleZoneUpdate(selectedZone)}
                  min="1"
                  max="100"
                />
              </div>
            </div>
          </div>
        </div>
      {:else}
        <div class="card card-body text-center text-admin-muted">
          <p>Выберите зону на изображении</p>
          <p class="text-xs mt-2">или создайте новую</p>
        </div>
      {/if}

      <!-- Легенда -->
      <div class="card mt-4">
        <div class="card-header">
          <h3 class="text-sm font-medium">Легенда</h3>
        </div>
        <div class="card-body space-y-2">
          {#each zoneTypes as type}
            <div class="flex items-center gap-2 text-sm">
              <div class="w-4 h-4 border-2 {getZoneColor(type.value)}"></div>
              <span>{type.label}</span>
              <span class="text-admin-muted">→ {type.route}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  </div>
</div>
```

### Критерии готовности
- [ ] Зоны отображаются поверх изображения
- [ ] Клик выбирает зону для редактирования
- [ ] Позиция и размер редактируются
- [ ] Тип и ссылка сохраняются
- [ ] Добавление/удаление работает

---

## Задача A9: Предпросмотр

### Контекст
Возможность увидеть как контент будет выглядеть в основном приложении.

### src/routes/preview/+page.svelte
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';

  let selectedView = 'cabinet';
  let figurines: any[] = [];
  let selectedFigurine: any = null;
  let authorTexts: any[] = [];
  let workshopItems: any[] = [];

  onMount(async () => {
    figurines = await api.getAllFigurines();
    authorTexts = await api.getAuthorTexts();
    workshopItems = await api.getWorkshopContent();

    if (figurines.length > 0) {
      selectedFigurine = await api.getFigurine(figurines[0].id);
    }
  });

  async function selectFigurine(id: string) {
    selectedFigurine = await api.getFigurine(id);
    selectedView = 'figurine';
  }
</script>

<div class="p-6">
  <div class="flex items-center justify-between mb-6">
    <h1 class="text-2xl font-semibold">Предпросмотр</h1>

    <div class="flex gap-2">
      <button
        class="btn btn-sm"
        class:btn-primary={selectedView === 'cabinet'}
        class:btn-secondary={selectedView !== 'cabinet'}
        on:click={() => selectedView = 'cabinet'}
      >
        Кабинет
      </button>
      <button
        class="btn btn-sm"
        class:btn-primary={selectedView === 'figurine'}
        class:btn-secondary={selectedView !== 'figurine'}
        on:click={() => selectedView = 'figurine'}
      >
        Фигура
      </button>
      <button
        class="btn btn-sm"
        class:btn-primary={selectedView === 'author'}
        class:btn-secondary={selectedView !== 'author'}
        on:click={() => selectedView = 'author'}
      >
        Автор
      </button>
      <button
        class="btn btn-sm"
        class:btn-primary={selectedView === 'workshop'}
        class:btn-secondary={selectedView !== 'workshop'}
        on:click={() => selectedView = 'workshop'}
      >
        Мастерская
      </button>
    </div>
  </div>

  <!-- Preview Container (имитация основного приложения) -->
  <div class="card overflow-hidden" style="background: #2E2B28;">
    <div class="aspect-video relative" style="font-family: 'Cormorant Garamond', serif; color: #CFC6B8;">

      {#if selectedView === 'cabinet'}
        <!-- Cabinet Preview -->
        <div class="absolute inset-0">
          <img
            src="/images/cabinet-room.jpg"
            alt="Cabinet"
            class="w-full h-full object-cover"
          />
        </div>

      {:else if selectedView === 'figurine' && selectedFigurine}
        <!-- Figurine Preview -->
        <div class="p-8 h-full flex gap-8">
          <div class="w-1/2">
            {#if selectedFigurine.images[0]}
              <img
                src={selectedFigurine.images[0].url}
                alt=""
                class="w-full h-full object-contain"
              />
            {/if}
          </div>
          <div class="w-1/2 flex flex-col justify-center">
            <h2 class="text-3xl mb-4">{selectedFigurine.name}</h2>
            {#if selectedFigurine.shortText}
              <p class="text-lg italic opacity-70">
                «{selectedFigurine.shortText}»
              </p>
            {/if}
          </div>
        </div>

        <!-- Выбор фигуры -->
        <div class="absolute bottom-4 left-4 flex gap-2">
          {#each figurines as fig}
            <button
              class="px-3 py-1 text-xs rounded"
              style="background: rgba(207, 198, 184, 0.2);"
              class:ring-1={selectedFigurine?.id === fig.id}
              on:click={() => selectFigurine(fig.id)}
            >
              {fig.name}
            </button>
          {/each}
        </div>

      {:else if selectedView === 'author'}
        <!-- Author Preview -->
        <div class="p-12 h-full overflow-auto">
          <div class="max-w-2xl mx-auto space-y-12">
            {#each authorTexts as text, i}
              <blockquote
                class="text-xl italic opacity-70"
                style="transform: rotate({(i % 3) - 1}deg); margin-left: {(i * 5) % 15}%;"
              >
                «{text.content}»
              </blockquote>
            {/each}
          </div>
        </div>

      {:else if selectedView === 'workshop'}
        <!-- Workshop Preview -->
        <div class="p-8 h-full relative">
          {#each workshopItems as item, i}
            <div
              class="absolute w-48 p-2"
              style="
                left: {10 + (i % 3) * 30}%;
                top: {10 + Math.floor(i / 3) * 40}%;
                transform: rotate({(i % 5) - 2}deg);
                background: rgba(90, 82, 76, 0.3);
              "
            >
              {#if item.imageUrl}
                <img src={item.imageUrl} alt="" class="w-full mb-2" />
              {/if}
              <p class="text-xs opacity-70">{item.caption}</p>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <p class="text-admin-muted text-sm mt-4 text-center">
    Это упрощённый предпросмотр. Запустите основное приложение для полного просмотра.
  </p>
</div>
```

### Критерии готовности
- [ ] Переключение между видами
- [ ] Стилизация как в основном приложении
- [ ] Предпросмотр фигур с выбором
- [ ] Предпросмотр текстов
- [ ] Предпросмотр мастерской

---

## Задача A10: API и TypeScript типы

### Контекст
Обёртка для всех Tauri-команд с типизацией.

### src/lib/api.ts
```typescript
import { invoke } from '@tauri-apps/api/core';
import type {
  FigurineListItem,
  FigurineEditData,
  Figurine,
  CreateFigurineInput,
  UpdateFigurineInput,
  AuthorText,
  WorkshopItem,
  CabinetZone,
  DashboardStats,
  ImageData,
} from './types';

export const api = {
  // ==================== Dashboard ====================

  async getDashboardStats(): Promise<DashboardStats> {
    return invoke('get_dashboard_stats');
  },

  // ==================== Figurines ====================

  async getAllFigurinesAdmin(): Promise<FigurineListItem[]> {
    return invoke('get_all_figurines_admin');
  },

  async getAllFigurines(): Promise<FigurineListItem[]> {
    return invoke('get_all_figurines');
  },

  async getFigurine(id: string): Promise<Figurine | null> {
    return invoke('get_figurine', { id });
  },

  async getFigurineForEdit(id: string): Promise<FigurineEditData | null> {
    return invoke('get_figurine_for_edit', { id });
  },

  async createFigurine(input: CreateFigurineInput): Promise<Figurine> {
    return invoke('create_figurine', { input });
  },

  async updateFigurine(input: UpdateFigurineInput): Promise<void> {
    return invoke('update_figurine', { input });
  },

  async deleteFigurine(id: string): Promise<void> {
    return invoke('delete_figurine', { id });
  },

  async reorderFigurines(ids: string[]): Promise<void> {
    return invoke('reorder_figurines', { ids });
  },

  // ==================== Images ====================

  async getImagesForFigurine(figurineId: string): Promise<ImageData[]> {
    return invoke('get_images_for_figurine', { figurineId });
  },

  async uploadImage(input: {
    figurineId: string;
    filePath: string;
    imageType: string;
  }): Promise<string> {
    return invoke('upload_image', { input });
  },

  async deleteImage(id: string): Promise<void> {
    return invoke('delete_image', { id });
  },

  async updateImageType(id: string, imageType: string): Promise<void> {
    return invoke('update_image_type', { id, imageType });
  },

  // ==================== Texts ====================

  async getAuthorTexts(): Promise<AuthorText[]> {
    return invoke('get_author_texts');
  },

  async createAuthorText(content: string): Promise<AuthorText> {
    return invoke('create_author_text', { content });
  },

  async updateText(id: string, content: string): Promise<void> {
    return invoke('update_text', { id, content });
  },

  async deleteText(id: string): Promise<void> {
    return invoke('delete_text', { id });
  },

  // ==================== Workshop ====================

  async getWorkshopContent(): Promise<WorkshopItem[]> {
    return invoke('get_workshop_content');
  },

  async createWorkshopItem(input: {
    content: string;
    caption: string;
    imagePath: string | null;
  }): Promise<WorkshopItem> {
    return invoke('create_workshop_item', { input });
  },

  async deleteWorkshopItem(id: string): Promise<void> {
    return invoke('delete_workshop_item', { id });
  },

  // ==================== Cabinet Zones ====================

  async getCabinetZones(): Promise<CabinetZone[]> {
    return invoke('get_cabinet_zones');
  },

  async createCabinetZone(input: Omit<CabinetZone, 'id'>): Promise<CabinetZone> {
    return invoke('create_cabinet_zone', { input });
  },

  async updateCabinetZone(zone: CabinetZone): Promise<void> {
    return invoke('update_cabinet_zone', { zone });
  },

  async deleteCabinetZone(id: string): Promise<void> {
    return invoke('delete_cabinet_zone', { id });
  },
};
```

### src/lib/types.ts
```typescript
export interface DashboardStats {
  figurines: number;
  available: number;
  sold: number;
  reserved: number;
  authorTexts: number;
  workshopItems: number;
}

export interface FigurineListItem {
  id: string;
  name: string;
  status: 'available' | 'sold' | 'reserved';
  year: number | null;
  sortOrder: number;
  imageCount: number;
}

export interface FigurineEditData {
  id: string;
  name: string;
  shortText: string | null;
  year: number | null;
  status: string;
  sortOrder: number;
}

export interface Figurine {
  id: string;
  name: string;
  shortText: string | null;
  year: number | null;
  status: string;
  images: ImageData[];
}

export interface CreateFigurineInput {
  name: string;
  shortText: string | null;
  year: number | null;
  status: string;
}

export interface UpdateFigurineInput {
  id: string;
  name: string;
  shortText: string | null;
  year: number | null;
  status: string;
}

export interface ImageData {
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

### Критерии готовности
- [ ] Все API-методы типизированы
- [ ] invoke-вызовы корректны
- [ ] Типы соответствуют backend DTO

---

## Порядок выполнения

```
[A1] Инициализация проекта
         ↓
[A2] Tailwind CSS для админки
         ↓
[A3] Layout и навигация
         ↓
[A10] API и типы ←─────────────────┐
         ↓                          │
[A4] Backend CRUD фигур             │
         ↓                          │
[A5] UI списка/редактирования ──────┤
         ↓                          │
[A6] Загрузка изображений ──────────┤
         ↓                          │
[A7] Редактирование текстов ────────┤
         ↓                          │
[A8] Настройка зон ─────────────────┤
         ↓                          │
[A9] Предпросмотр ──────────────────┘
```

---

## Критерии готовности админки

### Функциональность
- [ ] CRUD для фигур работает полностью
- [ ] Загрузка изображений с оптимизацией
- [ ] Редактирование всех текстов
- [ ] Визуальный редактор зон
- [ ] Предпросмотр контента

### Технически
- [ ] Общая БД с основным приложением
- [ ] Изображения сохраняются в правильное место
- [ ] Нет конфликтов при одновременной работе

### UX
- [ ] Интуитивный интерфейс
- [ ] Подсказки по формату контента
- [ ] Подтверждение удаления
- [ ] Сохранение при blur

---

## Памятка для LLM-агента

### Отличия от основного приложения
- UI функциональный, не художественный
- Используется стандартная нейтральная тема
- Фокус на удобстве, не на атмосфере
- Можно использовать белый цвет и bold

### Важно
- Админка — отдельное приложение
- Работает с той же БД
- Изображения сохраняются в static основного приложения
- Не влияет на художественную часть проекта

---

*Документ является дополнением к TASKS.md*
*Версия: 1.0*
