# Лекция 1. CRUD — четыре операции, на которых стоит весь бэкенд

> Прочитав эту лекцию, ты поймёшь, что такое CRUD, как он ложится на слои бэкенда Gotiga,
> как писать запросы через `sqlx`, как обрабатывать ошибки и где жить валидации.
> В конце — полностью разобранный пример «Заметки» (Notes), который повторяет задачу из блока 1.

---

## 1. Что такое CRUD и зачем он

CRUD — это четыре базовые операции над данными:

| Буква | Операция | SQL | HTTP-метод | Пример в Gotiga |
|-------|----------|-----|-----------|-----------------|
| **C**reate | создать | `INSERT` | `POST` | посетитель оставил заказ |
| **R**ead | прочитать | `SELECT` | `GET` | показать список фигурок |
| **U**pdate | изменить | `UPDATE` | `PATCH`/`PUT` | админ сменил статус заказа |
| **D**elete | удалить | `DELETE` | `DELETE` | удалить фигурку |

Почти любой бэкенд — это много-много CRUD над разными сущностями (фигурки, заказы, комментарии,
бронирования). Если ты умеешь чисто написать CRUD над одной сущностью — ты умеешь писать бэкенд.

**Главная идея:** одна сущность = одна таблица в БД + одна структура в Rust + набор из 4 операций.

---

## 2. Слои: почему нельзя писать всё в одном месте

В Gotiga запрос проходит через три слоя. Это не бюрократия — это разделение ответственности,
чтобы код можно было читать, тестировать и менять по частям.

```
HTTP-запрос
   │
   ▼
┌─────────────────────────────┐
│ КОНТРОЛЛЕР  (api/handlers.rs)│  ← разобрать запрос, вызвать сервис, собрать HTTP-ответ
└─────────────┬───────────────┘
              ▼
┌─────────────────────────────┐
│ СЕРВИС      (services/mod.rs)│  ← бизнес-логика: проверки, правила, «что делать»
└─────────────┬───────────────┘
              ▼
┌─────────────────────────────┐
│ РЕПОЗИТОРИЙ (db/mod.rs)      │  ← только SQL: «как достать/положить данные»
└─────────────┬───────────────┘
              ▼
          Postgres
```

**Правило, которое спрашивают на защите:**
- В контроллере **нет** SQL и **нет** бизнес-правил. Он только переводит HTTP ↔ вызов сервиса.
- В сервисе **нет** SQL. Он решает «можно ли», «что проверить», «что вернуть».
- В репозитории **нет** бизнес-правил. Он только выполняет конкретный запрос.

Зачем так? Если завтра захочешь поменять БД с Postgres на что-то другое — трогаешь только репозиторий.
Если поменялось бизнес-правило — только сервис. Контроллер вообще не в курсе, как устроена БД.

Эталон в коде — поток заказа: `create_order` в `handlers.rs` → `AppService::create_order`
(`services/mod.rs:963`) → методы `Repository` в `db/mod.rs`. Открой и пройди его глазами сверху вниз.

---

## 3. Async и `await` — самый минимум

Все обращения к БД в Gotiga — **асинхронные**. Тебе пока достаточно знать три вещи:

1. Функция, которая ждёт БД, помечена `async`: `pub async fn ... `.
2. Когда внутри зовёшь другую async-функцию — ставишь `.await`:
   ```rust
   let count = self.repo.count_figurines().await?;
   ```
3. `.await` означает «подожди результат, не блокируя другие запросы». Думай о нём как о «дай ответ, когда будет готов».

Глубоко в async лезть сейчас не надо — просто ставь `.await` там, где зовёшь async-функцию.

---

## 4. `Result` и `?` — как мы обрабатываем ошибки

Почти каждая операция с БД может провалиться (нет сети, нет строки, кривые данные). В Rust такие
функции возвращают `Result<T, E>` — «или значение `T`, или ошибка `E`».

В Gotiga есть готовый псевдоним (`error.rs:96`):
```rust
pub type Result<T> = std::result::Result<T, AppError>;
```
То есть `Result<i64>` = «или `i64`, или `AppError`».

**Оператор `?`** — это «если ошибка — выйди из функции и верни её наверх; если значение — достань его».

```rust
pub async fn count_figurines(&self) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM figurines")
        .fetch_one(&self.pool)
        .await?;   // ← если запрос упал, ошибка улетит наверх сама
    Ok(count)
}
```

**Никогда не пиши `.unwrap()` или `.expect()` на пути запроса.** `.unwrap()` при ошибке
**уронит весь сервер** (panic). `?` — аккуратно вернёт ошибку клиенту. Запомни: `?` хорошо, `.unwrap()` плохо.

> Почему `?` вообще работает? Потому что `AppError` умеет превращаться из ошибок `sqlx`
> (`error.rs:12`: `#[from] sqlx::Error`). Оператор `?` сам вызывает эту конверсию.

---

## 5. `sqlx` — как писать запросы

`sqlx` — библиотека для работы с БД. Тебе хватит четырёх кирпичей.

### 5.1. Чтение одной строки → `query_as` + `fetch_one`
```rust
let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notes")
    .fetch_one(&self.pool)   // ровно одна строка; если 0 — ошибка
    .await?;
```

### 5.2. Чтение многих строк → `query_as` + `fetch_all`
```rust
let notes: Vec<Note> = sqlx::query_as::<_, Note>("SELECT * FROM notes WHERE figurine_id = $1")
    .bind(&figurine_id)
    .fetch_all(&self.pool)
    .await?;
```
Чтобы `Note` собирался из строки автоматически, на структуре должен быть `#[derive(sqlx::FromRow)]`.

### 5.3. Может быть, а может и нет → `fetch_optional`
```rust
let note: Option<Note> = sqlx::query_as::<_, Note>("SELECT * FROM notes WHERE id = $1")
    .bind(id)
    .fetch_optional(&self.pool)   // вернёт Some(...) или None
    .await?;
```
`fetch_optional` — твой друг для «найти по id»: `None` удобно превратить в `AppError::NotFound`.

### 5.4. Запись (INSERT/UPDATE/DELETE) → `query` + `execute`
```rust
let result = sqlx::query("DELETE FROM notes WHERE id = $1")
    .bind(id)
    .execute(&self.pool)
    .await?;
let deleted: u64 = result.rows_affected();   // сколько строк затронули
```

### Подстановка значений: `$1`, `$2` и `.bind(...)`
В Postgres плейсхолдеры — `$1`, `$2`, ... Значения подставляются через `.bind(...)` **по порядку**.

```rust
sqlx::query("INSERT INTO notes (id, figurine_id, text) VALUES ($1, $2, $3)")
    .bind(Uuid::new_v4())
    .bind(&note.figurine_id)
    .bind(&note.text)
    .execute(&self.pool)
    .await?;
```

> **Безопасность (важно):** НИКОГДА не склеивай SQL строками вручную
> (`format!("... WHERE id = {id}")`). Это дыра SQL-инъекции. Всегда `$1` + `.bind`.

---

## 6. Валидация — почему в сервисе, а не в БД и не в контроллере

«Текст заметки не должен быть пустым» — это **бизнес-правило**. Его место — сервис:

```rust
pub async fn create_note(&self, note: NewNote) -> Result<()> {
    let text = note.text.trim();
    if text.is_empty() {
        return Err(AppError::BadRequest("Текст заметки пустой".into()));
    }
    if text.chars().count() > 500 {
        return Err(AppError::BadRequest("Слишком длинный текст".into()));
    }
    self.repo.insert_note(note).await
}
```

Почему не в БД? Потому что БД вернёт невнятную ошибку, а пользователю нужен понятный текст и код 400.
Почему не в контроллере? Потому что то же правило может понадобиться из другого места — пусть живёт в одном.

---

## 7. Превращение ошибок в HTTP-коды

Об этом заботится `AppError` (`error.rs`). Ты просто возвращаешь нужный вариант, а HTTP-код подставится сам:

| Ситуация | Вариант | HTTP |
|----------|---------|------|
| невалидный ввод | `AppError::BadRequest(msg)` | 400 |
| не нашли по id | `AppError::NotFound(msg)` | 404 |
| нет прав | `AppError::Unauthorized` | 401 |
| ошибка БД | `AppError::Database(...)` (сам через `?`) | 500 |

«Найти по id» обычно выглядит так:
```rust
let note = self.repo.get_note(id).await?
    .ok_or_else(|| AppError::NotFound(format!("Заметка {id} не найдена")))?;
```
`ok_or_else` превращает `None` в ошибку `NotFound`. Очень частый приём — запомни его.

---

## 8. Полный разбор: фича «Заметки» от БД до HTTP

Соберём весь CRUD на маленькой сущности `Note`. Это ровно то, что нужно сдать в задачах 1.1–1.4.

### Шаг 1. Миграция — `migrations/2026XXXX_notes.sql`
```sql
CREATE TABLE notes (
    id          UUID PRIMARY KEY,
    figurine_id TEXT NOT NULL,
    text        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
```
Миграции применяются автоматически при старте сервера (`main.rs`). Имя файла — с датой впереди, чтобы порядок был предсказуем.

### Шаг 2. Модель — в `models/mod.rs`
```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Note {
    pub id: Uuid,
    pub figurine_id: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

// то, что присылает клиент при создании (без id и даты — их ставит сервер)
#[derive(Debug, Deserialize)]
pub struct NewNote {
    pub figurine_id: String,
    pub text: String,
}
```
- `FromRow` — чтобы `sqlx` собрал `Note` из строки БД.
- `Serialize` — чтобы отдать `Note` клиенту как JSON.
- `Deserialize` — чтобы принять `NewNote` из JSON-тела запроса.

### Шаг 3. Репозиторий — в `db/mod.rs` (CRUD-методы)
```rust
impl Repository {
    pub async fn insert_note(&self, note: NewNote) -> Result<Note> {
        let row = sqlx::query_as::<_, Note>(
            "INSERT INTO notes (id, figurine_id, text)
             VALUES ($1, $2, $3)
             RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(&note.figurine_id)
        .bind(&note.text)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_notes(&self, figurine_id: &str) -> Result<Vec<Note>> {
        let notes = sqlx::query_as::<_, Note>(
            "SELECT * FROM notes WHERE figurine_id = $1 ORDER BY created_at DESC",
        )
        .bind(figurine_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(notes)
    }

    pub async fn get_note(&self, id: Uuid) -> Result<Option<Note>> {
        let note = sqlx::query_as::<_, Note>("SELECT * FROM notes WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(note)
    }

    pub async fn update_note(&self, id: Uuid, text: &str) -> Result<u64> {
        let res = sqlx::query("UPDATE notes SET text = $1 WHERE id = $2")
            .bind(text)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected())
    }

    pub async fn delete_note(&self, id: Uuid) -> Result<u64> {
        let res = sqlx::query("DELETE FROM notes WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected())
    }
}
```

### Шаг 4. Сервис — в `services/mod.rs` (правила + оркестрация)
```rust
impl AppService {
    pub async fn create_note(&self, note: NewNote) -> Result<Note> {
        let text = note.text.trim();
        if text.is_empty() {
            return Err(AppError::BadRequest("Текст заметки пустой".into()));
        }
        self.repo.insert_note(note).await
    }

    pub async fn list_notes(&self, figurine_id: &str) -> Result<Vec<Note>> {
        self.repo.list_notes(figurine_id).await
    }

    pub async fn edit_note(&self, id: Uuid, text: String) -> Result<()> {
        let text = text.trim();
        if text.is_empty() {
            return Err(AppError::BadRequest("Текст заметки пустой".into()));
        }
        let affected = self.repo.update_note(id, text).await?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("Заметка {id} не найдена")));
        }
        Ok(())
    }

    pub async fn remove_note(&self, id: Uuid) -> Result<()> {
        let affected = self.repo.delete_note(id).await?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("Заметка {id} не найдена")));
        }
        Ok(())
    }
}
```
Заметь: проверка «строк затронуто 0 → 404» — это бизнес-решение, и оно в сервисе.

### Шаг 5. Контроллер — в `api/handlers.rs`
```rust
pub async fn create_note(
    State(service): State<AppService>,
    Json(note): Json<NewNote>,
) -> Result<Json<Note>> {
    let saved = service.create_note(note).await?;
    Ok(Json(saved))
}

pub async fn list_notes(
    State(service): State<AppService>,
    Path(figurine_id): Path<String>,
) -> Result<Json<Vec<Note>>> {
    let notes = service.list_notes(&figurine_id).await?;
    Ok(Json(notes))
}

pub async fn delete_note(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.remove_note(id).await?;
    Ok(StatusCode::NO_CONTENT)   // 204 — удалили, тела нет
}
```
- `State(service)` — axum достаёт сервис из общего состояния (это `Clone`-структура, копия дешёвая).
- `Json(note)` — axum разбирает JSON-тело в `NewNote`.
- `Path(...)` — берёт кусок из URL (`/notes/:id`).

### Шаг 6. Роуты — в `api/mod.rs` (рядом с комментариями)
```rust
.route("/figurines/:id/notes",
    get(handlers::list_notes).post(handlers::create_note))
.route("/notes/:id",
    delete(handlers::delete_note).patch(handlers::edit_note))
```

Всё. Запрос `POST /api/v1/figurines/abc/notes` с телом `{"figurine_id":"abc","text":"тихо"}`
пройдёт контроллер → сервис (проверит текст) → репозиторий (INSERT) → вернётся JSON с новой заметкой.

---

## 9. Лучшие практики (чек-лист перед сдачей)

- [ ] Везде на пути запроса `?`, **ни одного** `.unwrap()`/`.expect()`.
- [ ] Значения в SQL только через `.bind($1)`, никакого `format!` с данными в строке запроса.
- [ ] Валидация и правила — в сервисе. SQL — в репозитории. Разбор HTTP — в контроллере.
- [ ] «Не нашли» → `NotFound` (404), «кривой ввод» → `BadRequest` (400).
- [ ] У модели для чтения из БД есть `#[derive(sqlx::FromRow)]`; для JSON — `Serialize`/`Deserialize`.
- [ ] Update/Delete возвращают `rows_affected()`, и сервис проверяет `== 0` → 404.
- [ ] Имена методов читаются как действия: `create_note`, `list_notes`, `delete_note`.

## 10. Частые ошибки новичков

| Симптом | Причина | Как чинить |
|---------|---------|-----------|
| Сервер падает при кривом вводе | `.unwrap()` на пути запроса | заменить на `?` + `AppError` |
| `fetch_one` кидает ошибку, когда строки нет | `fetch_one` требует ровно 1 строку | для «найти по id» брать `fetch_optional` |
| Удаление несуществующего «успешно» | не проверили `rows_affected()` | `if affected == 0 { NotFound }` |
| SQL-инъекция в ревью | склейка строк | только `.bind` |
| Бизнес-правило в SQL/контроллере | смешали слои | вынести в сервис |

---

### Что теперь делать
Открой `STUDENT_TASKS_BASIC.md`, блок 1. Повтори фичу «Заметки» по шагам выше своими руками,
потом добавь `edit_note`-хендлер сам (по образцу `delete_note`). На защите будь готов показать,
где именно в твоём коде живёт каждая из трёх ответственностей (контроллер/сервис/репозиторий).
