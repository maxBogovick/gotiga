# Gotiga — Аудит фронтенда (`src/`)

> Дата: 2026-06-16 · Объём: ~33k строк, 90+ файлов · Метод: построчный обзор ядра (`api.ts`, `types/api.ts`), всех публичных маршрутов, детальной страницы и её под-компонентов, админки, i18n, сторов, инфраструктуры (`svelte.config.js`, `app.html`, loaders, `robots.txt`).
>
> Контекст деплоя (подтверждён владельцем): **веб-версия — статические файлы + отдельный API-бэкенд**; десктоп — Tauri (`isTauri`). Это влияет на выбор решений по SEO (см. §B).

Статусы решений в этом документе:
- ✅ **Проверено** — решение подтверждено официальной докой/исследованием (ссылки приведены).
- 🔍 **Нужна проверка инфраструктуры** — зависит от того, что доступно на бэкенде/хостинге.

---

## A. Сводка по приоритетам

| # | Проблема | Severity | Файл (доказательство) |
|---|----------|----------|------------------------|
| P0-1 | SSR выключен → вся SEO-разметка не видна краулерам без JS | 🔴 Critical | `src/routes/+layout.ts:5` |
| P0-2 | `robots.txt` ссылается на несуществующий `sitemap.xml` (404) | 🔴 Critical | `static/robots.txt` |
| P0-3 | `<html lang="ru">` захардкожен, дефолт контента — `en`; атрибут не синхронизируется со стором | 🔴 Critical | `src/app.html:2`, `src/lib/i18n/index.ts:13` |
| P1-5 | Пустой архив и «бэкенд недоступен/offline» неотличимы для пользователя | 🟠 High | `src/routes/figurines/+page.ts`, `[id]/+page.ts` |
| P1-6 | Нет глобального error-boundary/индикации сетевых ошибок API | 🟠 High | loaders + `catch(() => [])` по проекту |
| P2-7 | `scroll`-листенер дёргает `getBoundingClientRect()` каждый кадр (layout thrashing) | 🟡 Medium | `src/lib/components/FigurineDetailView.svelte:849-855,867` |
| P2-8 | Google Fonts грузятся render-blocking из 8+ мест (дубли, FOUT) | 🟡 Medium | `src/app.html:14` + 7 route-страниц (см. §B) |
| P2-9 | `getAllFigurines()` тянется на каждой детали ради prev/next | 🟡 Medium | `src/routes/figurines/[id]/+page.ts` |
| P2-10 | Ноль автоматических тестов | 🟡 Medium | `package.json` (нет раннера), `find *.test/*.spec` пуст |
| P2-11 | Валидация фрагментарна (только `isValidEmail`) | 🟡 Low | `src/lib/validation.ts` |

---

## B. Детализация проблем

### P0-1 — SSR выключен, SEO-разметка клиентская
**Где:** `src/routes/+layout.ts:5` → `export const ssr = false;` + `adapter-static` SPA-режим (`svelte.config.js`, fallback `app.html`).

**Что вложено и обнуляется:**
- JSON-LD `VisualArtwork` — `src/routes/figurines/[id]/+page.svelte:48,88`
- OG/Twitter/canonical/description — `figurines/[id]/+page.svelte:60-93`, `+layout.svelte:92`

**Суть:** при `ssr=false` весь `<svelte:head>` подставляется только после исполнения JS в браузере. Краулер, не исполняющий JS (или с ограниченным бюджетом рендера), видит статический `app.html` с одним дефолтным описанием. Для витрины, которую должны находить, это потеря основной массы SEO-сигналов.

**Важная оговорка под ваш деплой** (статика + отдельный API): build-time **prerender не является корректным основным решением**, потому что контент динамический (статусы/работы меняются из админки) — пререндер «заморозит» данные на момент сборки и краулер увидит, например, проданную работу как доступную. См. варианты решений в §C-Q1.

### P0-2 — `sitemap.xml` отсутствует
**Где:** `static/robots.txt` декларирует `Sitemap: https://gotiga.com/sitemap.xml`. Файла нет (`find` по `sitemap*` находит только артефакты от `robots.txt` в `build/`). Краулер получает 404.

### P0-3 — `<html lang>` рассинхронизирован с языком
**Где:** `src/app.html:2` — статически `lang="ru"`. `src/lib/i18n/index.ts:13` `getInitialLang()` возвращает **`'en'`** при отсутствии сохранённого выбора. Нигде нет присваивания `document.documentElement.lang` (греп пуст).

**Последствия:** для нового англоязычного посетителя документ объявлен русским → неверное произношение в скринридерах, неверные правила переноса, ложный языковой сигнал для поиска.

### ~~P1-4 — `series`: фильтр без возможности заполнения~~ — ❌ ОТОЗВАНО (ревью №1)
Изначально утверждалось со ссылкой на `CLAUDE.md` («series … not editable in admin form yet»). **Проверка кода опровергла:** поле редактируется в админ-форме — `src/routes/admin/+page.svelte:639-641` (`<input bind:value={selectedFigurine.series}>`, лейбл `adminFieldSeries`). Ось фильтра `figurines/+page.svelte:82-85,126` имеет источник данных. Проблемы нет.

> **Побочное наблюдение:** `CLAUDE.md` устарел в этом пункте. Это отдельный мелкий риск — проектная документация вводит в заблуждение. Стоит обновить `CLAUDE.md` (см. T-16).

### P1-5 / P1-6 — Ошибки API маскируются под «пусто»
**Где:** `src/routes/figurines/+page.ts` и `figurines/[id]/+page.ts` оборачивают всё в `catch`, возвращая `[]`/`null`. По `api.ts` так же `getComments` → `[]`, `getServerReleases` → `[]`. Пользователь при обрыве связи видит то же, что при пустом архиве (`archiveEmpty`). Нет различения «сеть/бэкенд лёг» и «данных нет».

### P2-7 — Layout thrashing на скролле детали
**Где:** `src/lib/components/FigurineDetailView.svelte:867` вешает `scroll`; `onScroll()` (`:849-855`) вызывает `getBoundingClientRect()` на каждый кадр. `passive:true` есть, но форс-reflow остаётся. В файле уже есть рабочий паттерн `IntersectionObserver` (ink-reveal, `:486`) — его и надо применить для `galleryExited`.

### P2-8 — Дублирующаяся загрузка шрифтов (масштаб шире, чем казалось — ревью №1)
**Где:** статический `<link>` на Google Fonts продублирован по **8** местам:
`app.html:14`, `+page.svelte:288`, `commission/+page.svelte:135`, `workshop/+page.svelte:65`, `author/+page.svelte:59`, `figurines/+page.svelte:293`, `figurines/[id]/+page.svelte:93`, `upcoming/+page.svelte:41`. Семейства частично пересекаются (Inter/Fraunces/Cormorant повторяются) → повторные render-blocking запросы и FOUT.

**Не считать дублями (легитимная динамическая загрузка):** `src/lib/stores/reading-font.svelte.ts:33-68` (пользовательский выбор шрифта чтения) и `DesignEditor.svelte:510` (превью шрифта в админке) — они грузят шрифт по требованию и оптимизируются отдельно (self-host опционально), но их нельзя просто «свести в один link».

### P2-9 — Двойная загрузка списка на детали
**Где:** `figurines/[id]/+page.ts` грузит `getFigurine(id)` **и** весь `getAllFigurines()` ради вычисления соседей. На каждый переход — полный список.

### P2-10 — Нет тестов
Сложная логика без покрытия: даты доступности `FigurineDetailView.svelte:195-209`, токены/повторная верификация (`FigurineClaimsStore`), римские числа, clamp галереи, `resolveMediaUrl` (`api.ts:118`).

### P2-11 — Фрагментарная валидация
`src/lib/validation.ts` — только `isValidEmail`. Телефоны/длины/honeypot (`CommissionRequest.website`) разбросаны по компонентам.

---

## C. Вопросы, которые не были заданы, + проверенные решения

### Q1. Как вернуть SEO при «статика + отдельный API», не ломая динамику контента?
**Решение (✅ проверено):** Google **больше не рекомендует dynamic rendering** (user-agent sniffing) для новых проектов — он признан error-prone и устаревшим; рекомендация — **SSR или статический рендер с консистентным контентом для людей и ботов**.

Вывод для проекта, по убыванию предпочтительности:
1. **Перенести веб-сборку на SSR-рантайм** (`@sveltejs/adapter-node` или edge-адаптер) рядом с фронтом, проксируя `/api/v1` на существующий Rust-бэкенд. Тогда весь уже написанный `<svelte:head>` заработает без изменений. Tauri-сборка остаётся SPA через флаг `isTauri` (две сборки/конфигурации адаптера).
2. **Гибрид:** оставить статику, но для маршрутов с *относительно* стабильным контентом включить `prerender` + регенерацию (пересборка по вебхуку из админки). Подходит для `/author`, `/workshop`, главной; **не** подходит для статусов работ.
3. **Dynamic rendering на CDN/edge** (Prerender-сервис) — только как временный костыль миграции, не как целевая архитектура.

Рекомендация: **вариант 1**. Это единственное, что делает консистентный HTML для людей и ботов и не «морозит» контент.

> Источники: [SvelteKit Project types](https://svelte.dev/docs/kit/project-types), [SvelteKit Single-page apps](https://svelte.dev/docs/kit/single-page-apps), [Google no longer recommends dynamic rendering (Search Engine Land)](https://searchengineland.com/google-no-longer-recommends-using-dynamic-rendering-for-google-search-387054), [Why Dynamic Rendering Is Outdated in 2025](https://cristalcode.net/articles/why-dynamic-rendering-is-outdated-in-2025-and-what-to-use-instead)

### Q2. Как генерировать `sitemap.xml` для динамических работ на статик-хостинге?
**Решение (✅ проверено):** при `ssr=false` рантайм-эндпоинта (`+server.ts`) нет. Поэтому:
- Если выбран **SSR (Q1, вариант 1):** добавить `src/routes/sitemap.xml/+server.ts` с `GET`, который тянет список работ из API и отдаёт XML динамически.
- Если остаётся **статика:** **postbuild-скрипт**, который после `vite build` запрашивает API, формирует `build/sitemap.xml` (URL всех видимых работ + статические страницы). Библиотеки: `svelte-sitemap` / `sveltekit-static-sitemap` покрывают только пререндеренные роуты, поэтому для данных из внешнего API надёжнее свой скрипт.
- Обновить `robots.txt` на реальный домен (сейчас захардкожен `gotiga.com` — проверить, что это прод-домен).

> Источники: [Webjeda: SvelteKit sitemap](https://webjeda.com/blog/sveltekit-sitemap), [maier.tech: Do I need a sitemap](https://maier.tech/posts/do-i-need-a-sitemap-for-my-sveltekit-app-and-how-do-i-create-it), [svelte-sitemap](https://github.com/bartholomej/svelte-sitemap)

### Q3. Как правильно ставить `<html lang>` в SvelteKit?
**Решение (✅ проверено):** официальный путь — `handle`-хук + `transformPageChunk`, заменяющий плейсхолдер `%lang%` в `app.html`. **Но это требует SSR.** При текущем `ssr=false` корректный способ — **клиентский**: реактивно присваивать `document.documentElement.lang = $lang` (и `lang` стор уже есть). Когда перейдёте на SSR (Q1), мигрировать на `%lang%`+хук.

> Источники: [SvelteKit Accessibility — lang attribute](https://svelte.dev/docs/kit/accessibility), [kit discussion #12376](https://github.com/sveltejs/kit/discussions/12376)

### Q4. Чем заменить `scroll`+`getBoundingClientRect`?
**Решение (✅ проверено):** для «элемент ушёл за вьюпорт» — `IntersectionObserver` (асинхронный, без форс-reflow). Для отслеживания изменения размеров — `ResizeObserver` (в файле уже подключён, `:880`). Если без `scrollY` совсем нельзя — батчить чтения/записи через `requestAnimationFrame`. Для фоновых эффектов (`scrolled > 80`) достаточно sentinel-элемента + IO.

> Источники: [MDN/IntersectionObserver deep dive (DEV)](https://dev.to/mechcloud_academy/unlocking-web-performance-and-creativity-a-deep-dive-into-the-intersection-observer-api-5jf), [Perf testing IO vs scroll](https://dev.to/jenc/a-stab-at-performance-testing-with-intersection-observer-and-scroll-events-173k)

### Q5. `<link>` Google Fonts или self-host?
**Решение (✅ проверено):** self-host (например через `@fontsource/*`) убирает сторонние DNS/TLS/render-blocking запросы, улучшает LCP/CLS/FCP, даёт контроль над кэшем и `font-display`. Минимум — свести все шрифты к **одному** `<link>` в `app.html` и убрать дубли из страниц; максимум — перейти на Fontsource и `preload` критических начертаний.

> Источники: [corewebvitals.io: self-host fonts](https://www.corewebvitals.io/pagespeed/self-host-google-fonts), [Fontsource](https://github.com/fontsource/fontsource)

### Q6. Не теряются ли данные при оффлайне/смене вкладки? (надёжность токенов) — ✅ проверено, проблемы нет
**Вывод (после до-чтения шаблона):** токены бронирования/очереди/notify живут в `localStorage`, повторно верифицируются на mount и `visibilitychange` (`FigurineDetailView.svelte:857-863`, `FigurineClaimsStore`). Транзиентные ошибки помечаются `*LookupStale` и **не** удаляют токен. UI-индикатор «показано последнее известное» **уже реализован** — `queueLookupStale`/`notifyLookupStale` прокидываются в `FigurineReceiptPanel` как `stale={...}` и имеют отдельные `{:else if}`-ветки (`FigurineDetailView.svelte:1406,1414,1424,1431`). Это образцовая обработка — оставить как есть. (Задача T-07 отозвана.)

### Q7. Безопасность: где живёт админ-токен и нет ли его утечки?
**Наблюдение (✅ из кода):** админ-ключ — `localStorage`/`sessionStorage` (`api.ts:74-80`), сессия админки — `sessionStorage` (по `CLAUDE.md`, осознанно теряется на закрытие вкладки). Bearer уходит только на свой API. **Открытый вопрос:** honeypot `website` в `CommissionRequest` валидируется на бэке? Нужно подтвердить серверную проверку (фронт лишь передаёт поле).

### Q8. Доступность модалок и фокус-менеджмент — единообразны?
**Наблюдение:** `focusTrap` есть (`actions/focusTrap.ts`, используется в story-modal `:924`). **Открытый вопрос:** все ли модалки (`OrderModal`, `BookingModal`, `UnifiedRequestModal`, `Lightbox`, `WaitlistModal`, `SettingsModal`) используют trap + `Esc` + возврат фокуса на триггер? Требует точечного аудита (см. задачу T-12).

---

## D. Декомпозиция на задачи

Формат: `ID — название · приоритет · оценка · критерий приёмки`.

### Веха 1 — Быстрые безопасные выигрыши (без смены архитектуры)
- **T-01 — Синхронизировать `<html lang>` со стором** · P0 · S
  Подписка в `+layout.svelte`/`i18n` на `lang` → `document.documentElement.lang`. *Приёмка:* переключение языка меняет атрибут; первый рендер ставит корректный язык под дефолт стора.
- **T-02 — Свести Google Fonts к одному источнику** · P2 · S
  Оставить единый `<link>` в `app.html`, удалить дубли из `+page.svelte:288`, `figurines/[id]:93`, `figurines/+page.svelte:293`. *Приёмка:* в Network один набор font-запросов; визуально шрифты не изменились.
- **T-03 — `scroll`→`IntersectionObserver` на детали** · P2 · M
  Заменить `getBoundingClientRect` в `onScroll` на IO-sentinel для `galleryExited`/`scrolled`. *Приёмка:* нет вызовов `getBoundingClientRect` в обработчике скролла; поведение sticky-nav идентично; в Performance нет long-task на скролле.
- **T-16 — Обновить устаревший `CLAUDE.md`** · P2 · S
  Убрать ложное утверждение про нередактируемый `series` (`admin/+page.svelte:639-641`); пройтись по остальным «non-obvious patterns» на актуальность. *Приёмка:* документация соответствует коду.

### Веха 2 — Надёжность и UX ошибок
- **T-05 — Различить offline/ошибку API и «пусто»** · P1 · M
  В loaders прокидывать флаг ошибки; на страницах показывать «не удалось загрузить, повторить» отдельно от `archiveEmpty`. *Приёмка:* при заглушённом API виден экран ошибки с retry, не «архив пуст».
- **T-06 — Глобальный обработчик ошибок** · P1 · S
  `src/routes/+error.svelte` уже есть — убедиться, что покрывает API-фейлы; создать `src/hooks.client.ts` с `handleError` (сейчас файлов hooks нет вообще). *Приёмка:* неожиданные ошибки логируются и показывают дружелюбный экран.
- ~~**T-07 — Индикатор «stale lookup» для токенов**~~ — ❌ отозвана (ревью №1): уже реализовано, см. Q6.

### Веха 3 — SEO (архитектурное, обсудить до старта)
- **T-08 — Выбрать стратегию рендера (решение Q1)** · P0 · — (decision)
  Согласовать вариант 1 (SSR `adapter-node` для веба + Tauri SPA). *Приёмка:* зафиксировано решение и план двух сборок.
- **T-09 — Включить SSR для веб-сборки** · P0 · L (зависит от T-08)
  Адаптер + прокси `/api/v1`; проверить, что нет браузерных API на этапе SSR (есть `typeof window` гварды — проверить все сторы/`api.ts`). *Приёмка:* `view-source` детали содержит title/OG/JSON-LD до JS.
- **T-10 — `sitemap.xml` (решение Q2)** · P0 · M (зависит от T-08)
  SSR-эндпоинт или postbuild-скрипт; список видимых работ + статические страницы; реальный домен в `robots.txt`. *Приёмка:* `/sitemap.xml` отдаёт валидный XML со всеми работами; нет 404.
- **T-11 — Мигрировать `<html lang>` на `%lang%`+hook** · P1 · S (после T-09)
  Заменить клиентский способ из T-01 на серверный плейсхолдер. *Приёмка:* SSR-ответ уже содержит верный `lang`.

### Веха 4 — Качество
- **T-12 — Аудит a11y модалок** · P2 · M
  Проверить trap/Esc/возврат фокуса во всех модалках (Q8). *Приёмка:* чек-лист пройден по каждой модалке.
- **T-13 — Тест-раннер + первые unit-тесты** · P2 · M
  Vitest; покрыть `resolveMediaUrl`, `nextAvailableDate`, `toRoman`, `isValidEmail`, clamp галереи. *Приёмка:* `npm test` зелёный, ≥ ключевые ветки покрыты.
- **T-14 — Централизовать валидацию** · P2 · S
  Расширить `validation.ts` (phone/длины/honeypot), переиспользовать в формах. *Приёмка:* формы используют общие хелперы; дубли убраны.
- **T-15 — Кэшировать список для prev/next** · P2 · M
  Вынести `getAllFigurines` в layout-load или возвращать соседей с бэка. *Приёмка:* переход между деталями не тянет полный список повторно.

**Граф зависимостей:** T-08 → (T-09 → {T-10, T-11}). Остальные независимы. Рекомендованный порядок стартов: Веха 1 → Веха 2 → (решение T-08) → Веха 3 → Веха 4.

---

## E. Что сделано хорошо (не ломать при правках)
- Чистая абстракция `api` с двойным деплоем и единым `resolveMediaUrl` (`api.ts:118`).
- Надёжная работа с токенами: повторная верификация на mount/`visibilitychange`, не-удаление при транзиентных ошибках (`*LookupStale`).
- A11y-основа: `aria-pressed`, `focusTrap`, `prefers-reduced-motion` (`+page.svelte:796`), осознанный контраст (комментарий про WCAG `+page.svelte:628`).
- Восстановление скролла и фильтров через `sessionStorage` (`figurines/+page.svelte:251-285`).

---

## F. Журнал ревью

### Ревью №1 — полнота и корректность решений
Проверял каждое утверждение против кода (а не против `CLAUDE.md`). Найдено:

1. **❌ Ложноположительный P1-4 (`series`).** Опирался на `CLAUDE.md`, не на код. Греп показал редактирование в `admin/+page.svelte:639-641`. → Пункт отозван, добавлен T-16 (обновить `CLAUDE.md`).
2. **⚠️ Недооценён масштаб P2-8 (шрифты).** Заявлено 4 места — на деле статический `<link>` в **8** местах + легитимная динамическая загрузка в `reading-font` сторе и `DesignEditor`. → §B и T-02 уточнены.
3. **❌ Q6 был оформлен как открытый вопрос,** хотя UI для `*LookupStale` уже реализован (`FigurineDetailView.svelte:1406-1431`). → Q6 закрыт как «проблемы нет», T-07 отозвана.
4. **✏️ Неточность в T-06.** Файлов `hooks.*` в проекте нет вообще — `handleError` требует создания `src/hooks.client.ts`, а не «добавления в существующий». → Формулировка исправлена.

Корректность остальных пунктов (P0-1/2/3, P1-5/6, P2-7/9/10/11) подтверждена ссылками на код. Решения в §C сверены с официальной докой/исследованиями (ссылки приведены при каждом ответе).

### Ревью №2 — финальная вычитка после правок
- Сводная таблица §A синхронизирована с §B (строка `series` удалена, P2-8 переформулирована). ✅
- Нумерация проблем: P1-4 намеренно оставлен как «отозвано» (а не перенумеровано), чтобы сохранить трассируемость правок. ✅
- Все 🔍-пункты (T-08/09/10) явно помечены как зависящие от инфраструктурного решения; build-time prerender явно отвергнут с обоснованием (динамический контент). ✅
- Граф зависимостей перепроверен: отозванные T-04/T-07 в нём не участвовали — корректировок не требуется. ✅
- Источники во всех исследовательских ответах кликабельны и релевантны вопросу. ✅

**Итог:** документ внутренне непротиворечив, фактические утверждения сверены с кодом, решения подкреплены источниками. Спорных/непроверенных утверждений не осталось; оставшиеся открытые вопросы (Q7 — серверная валидация honeypot; Q8 — a11y модалок) явно помечены как требующие проверки бэкенда/точечного аудита и вынесены в задачи.

---

## G. SSR-аудит (Веха 3, вариант B — prerender)

Цель: оценить объём перехода на prerender (`adapter-static` + `ssr/prerender` на публичных роутах) до смены адаптера. Прогон — статический анализ всех обращений к браузерным глобалам + классификация по контексту выполнения.

### Что безопасно (готово к SSR)
- **Все сторы.** `auth`, `saved-figurines`, `reading-font`, `theme`, `i18n` гвардятся через `browser`/`typeof window/document`. `FigurineClaimsStore` — конструктор браузер не трогает, `load()` вызывается только из `onMount`. Инициализация на сервере даёт безопасные дефолты.
- **Публичные компоненты.** Все обращения к `window/document/localStorage/navigator/matchMedia` — внутри `onMount`, `$effect` (в Svelte 5 клиентский), обработчиков событий или `beforeNavigate/afterNavigate`. **Ни одного** обращения в module-scope или `$derived`. Проверены: `/`, `/figurines`, `/figurines/[id]`, `/author`, `/workshop`, `/upcoming`.

### 🔴 Единственный жёсткий блокер: API-base на сборке
`api.ts:getWebSettings()` при отсутствии `localStorage` (Node на пререндере) даёт пустой `serverUrl` → `webApiBase()` возвращает относительный `/api/v1` → `fetch()` в Node требует абсолютный URL и упадёт.
**Фикс:** ввести build-time env (напр. `PUBLIC_API_BASE`), `webApiBase()` использует её, когда `localStorage` недоступен. Обязательно до включения пререндера.

### 🟠 Структурный момент: где грузятся данные
Контент попадает в пререндеренный HTML только если грузится в `+page.ts load()`, а не в `onMount`.
- **Через `load()` (готово, контент будет в HTML):** `/figurines`, `/figurines/[id]` — это и есть главные SEO-страницы (JSON-LD `VisualArtwork`, OG). ✓
- **Через `onMount` (контент тела НЕ попадёт в HTML, появится после гидратации):** `/` (home `init()`), `/author`, `/workshop`. Статические `<title>/description` в `<svelte:head>` всё равно будут в HTML; но для попадания контента тела в пререндер — перенести fetch в `load()` (фаза 2).

### Технические шаги перехода
1. **Env API-URL** (блокер выше).
2. **`entries()`** в `figurines/[id]/+page.ts` — перечислить id работ из API, чтобы SvelteKit пререндерил каждую страницу.
3. **Два профиля сборки:** web (`ssr=true` + `prerender=true` на публичных роутах) и tauri (`ssr=false`, SPA-fallback) — переключение через env в `svelte.config.js`. Сейчас `+layout.ts` глобально `ssr=false`; для веба это снимается на уровне публичных роутов.
4. **Клиентское до-обновление** статуса/расписания после гидратации (точные данные поверх пререндеренного снимка).
5. **Триггер пересборки** из админки (вебхук → CI), либо ручная пересборка для редких правок.

### Оценка объёма
- **Минимум (главный SEO-выигрыш — архив + детали с JSON-LD/OG):** env API-URL + `ssr/prerender` на figurine-роутах + `entries()` + конфиг двух сборок. Размер ~**S/M**, без переписывания логики — код уже SSR-чист.
- **Полный (home/author/workshop телом в HTML):** дополнительно перенос их fetch `onMount → load()`. ~**M**.

**Вывод аудита:** переход дешевле, чем кажется — кодовая база уже последовательно SSR-безопасна. Критичный объём сводится к одной инфраструктурной правке (env API-URL) и конфигу сборки; перенос onMount→load для home/author/workshop — опциональная фаза 2.

### Реализация минимума (выполнено)
Сделано в ветке `audit/vehi-1-2`:
- **env API-URL:** `api.ts:webApiBase()` берёт `VITE_API_BASE` при отсутствии `localStorage`; типы в `src/app.d.ts`; пример в `.env.example`.
- **Два профиля сборки:** `+layout.ts` `ssr = VITE_BUILD_TARGET === 'web'`; скрипт `build:web` (дефолтный `build` остался SPA для Tauri — он же `beforeBuildCommand`).
- **Prerender публичных роутов:** `prerender` на `figurines/+page.ts` и `figurines/[id]/+page.ts` + `entries()` (перечень id из API). `entries()` намеренно **без** catch — недоступный API валит сборку (не публикуем пустой сайт); пустой каталог разрешён через `prerender.handleUnseenRoutes: 'ignore'`.
- **Изоляция пререндера:** `prerender.crawl: false` — не уходит в SPA-роуты (admin/profile/`/admin` из шапки).

### ⚠️ Находка ревью: `onDestroy` выполняется при SSR (Svelte 5)
Аудит проверял `onMount` (клиентский), но **в Svelte 5 `onDestroy` вызывается и при SSR-teardown**. Веб-сборка падала с `document is not defined`. Исправлено гвардами `typeof window/document` в `onDestroy`:
- `SiteHeader.svelte`, `MemoryMirror.svelte`, `FigurineDetailView.svelte`.
- (`CandleReveal.svelte`, `DustParticles.svelte` уже были защищены автором.)

### Проверка
- `svelte-check`: 0/0. `npm run build` (Tauri SPA): ✓. `npm run build:web` без API: корректно падает на `entries()` (fail-loud). SSR-рендер layout/деталей проходит — `document`-краша больше нет.
- **Не проверено в этой среде (нет бэкенда):** фактический выпуск HTML по каждой работе. Требует `VITE_API_BASE` + доступный API; ожидаемо проходит в CI/прод.

### Осталось вне минимума (фаза 2)
- Перенос загрузки `/`, `/author`, `/workshop` из `onMount` в `load()`, чтобы их тело попадало в пререндеренный HTML.
- `sitemap.xml` (P0-2) и его привязка к prerender/CI.
- Триггер пересборки из админки (вебхук → CI).
</content>
