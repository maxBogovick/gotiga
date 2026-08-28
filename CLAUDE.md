# Gotiga — Claude Context

## Project identity (read before every decision)
Gothic miniature showcase. NOT a shop. Aesthetic = silence, dust, parchment, old house.
Rule: if a feature "speeds up perception" or "looks like a store" — it doesn't belong.

## Dual deployment
`isTauri` flag (`src/lib/api.ts:18`) → Tauri IPC; else → fetch `/api/v1`.
All API calls go through the `api` object in `src/lib/api.ts`. Never call fetch directly.

## Routes
`/` home · `/figurines` archive · `/figurines/[id]` detail · `/author` · `/workshop` · `/upcoming` (not in nav) · `/gazette` · `/tales` shelf of tall tales · `/battles` card shelf (not in nav) · `/battles/etude` studies — a match against the keeper (no SSR: a match belongs to a person) · `/admin`

## Key localStorage keys
`gotiga_api_key` · `gotiga_server_url` · `gotiga_wishlist` · `gotiga_viewed` · `gotiga_claims_${id}`

## Types that matter
`FigurineStatus`: available | sold | reserved | in_progress  
`BookingStatus`: pending | confirmed | rejected | cancelled  
`ScheduleEntry.entryType`: showing | booking | pending  
`OrderRequest.mode`: request | question | notify

## i18n
`import { t, lang } from '$lib/i18n'` → `$t('key')` in template.  
`en.ts` is the source of truth for keys. Always add to both files.

## Aesthetic constants
bg parchment `#f8f1e7` · text `#34251c` · accent `#c65f3c` · deep `#6f3b24` · border `#d8c6b1`  
Modals: rotate-1deg frame, double border, wax seal on success, Georgia/Fraunces/Inter fonts.

## Non-obvious patterns
- Booking creates a `cancelToken` — user saves it to cancel later. Works fully anonymously; logged-in users can additionally link bookings to their profile (`api.userLinkBookings`).
- `FigurineClaimsStore` persists tokens in localStorage, re-verifies on mount + tab focus
- Gallery keyboard nav: ←/→ arrows (undiscoverable — no hint shown)
- `view-transition-name: figurine-{id}` enables shared-element morph card→detail; `startViewTransition` is wired globally in `+layout.svelte` (onNavigate). Each `figurine-{id}` must be unique per rendered page or the transition aborts.
- Admin auth: `sessionStorage` only (lost on tab close by design)
- A tale is a gazette leaf with `kind = 'tale'` — same table, same admin plumbing — but it lives at `/tales/[slug]`, not `/gazette/[slug]`, which 308s to the shelf. Always build leaf links through `leafHref()`; it makes that choice in one place. The shelf is arranged by hand (`shelf_order`), not by date.
- `series` field is editable in the admin figurine form (`admin/+page.svelte`) and is a filter axis on the archive page.
- Web fonts: one consolidated Google Fonts `<link>` lives in `app.html`; route `<svelte:head>` blocks must NOT add their own (reader-font and admin design-preview fonts load dynamically via their stores).
- Битвы: a card is `battle_cards`, one row per figurine (`UNIQUE figurine_id`). Two 1..5 ranges that are NOT the same thing — `tier` is the card's rank (frame + price, keeper's choice), `level` is the state of one person's copy (`battle_owned_cards`, raised with dust). The wallet (`battle_wallet_entries`) is append-only: balance is `SUM(amount)`, and every write carries an `idem_key`. The five frames live in `settings.battle_frames`, not per card — design in the frame, content in the card. A frame is either PAINTED (paper/ink/border/foil) or DRESSED (`frameImage`, worn `overlay` for a cut-out or `behind` for a solid picture), with the content placed by four percentage insets plus `aspect`, `artShare`, `paperImage`, `titleFont` and `layout`. Frame art has its OWN upload (`POST /admin/battles/frames/art` → lossy WebP with alpha, under `uploads/frames/`, mounted in the `/static` subdir allowlist in `api/mod.rs`) — the ordinary `/upload` writes JPEG renditions and would fill in the hole. Frames are stretched, never 9-sliced: these assets are single pictures with position-specific ornament, so the card takes the picture's ratio on upload instead. `BattleCard.svelte` is the only renderer: shelf, preview and admin all use it, and it is sized by container queries, never by props. The card is FOUR BANDS inside the frame's window — header (race · type), photograph, properties (name plate, named traits, effect, note, Health/Mana), footer (level notches, cost/power). Three bands take a share from the frame (`headerShare`/`artShare`/`footShare`); the properties band takes the remainder so it can never be squeezed to nothing. A race is a dictionary row (`battle_races`, ON DELETE SET NULL — removing one never removes its cards); a trait belongs to one card and lives as JSON in `battle_cards.traits`.
- Сцена боя описана в `BATTLE-SCENE.md`: фотография работы на клетке (тот же `BattleCard`), числа только у раненых, разбор урона по наведению на след удара, ход хранителя проигрывается по событию с паузами, исход — сургучная печать. Запрещены таймер, полоски здоровья, всплывающие числа, подсказки «нажмите сюда». `prefers-reduced-motion` — обязательство, а не украшение.

- Бой: правила живут в отдельном крейте `src-tauri/battle-core` — путевая зависимость сервера, **без** workspace (иначе каталог сборки сервера переезжает). В ядре одна зависимость, `serde`; ни sqlx, ни часов, ни `rand` — границу держит `Cargo.toml`, а не уговор. Партия — это `reduce(state, action) -> (state, events)`, чистая функция; журнал действий и есть истина, доска в базе только кэш. Клиент не знает ни одного правила: сервер присылает `legalActions`, страница выбирает из них. Испытание (`battle_challenges`) задаёт **обе** стороны, как шахматный этюд, и пыль даётся за испытание (`idem_key = battle:{id}`), а не за победу — иначе PvE становится фермой. Числа правил выбраны замером: `tools/mutants.py` проверяет проверки, `examples/svodka.rs` меряет баланс, и обоснование каждого умолчания лежит в `Rules::default()`. Никогда не называйте переменную `state` в компоненте: `$state` тогда становится подпиской на неё, и компонент молча уезжает в legacy-режим.

- `<html lang>` is kept in sync with the language store from `+layout.svelte` (SPA has no SSR handle hook).
