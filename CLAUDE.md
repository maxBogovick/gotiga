# Gotiga — Claude Context

## Project identity (read before every decision)
Gothic miniature showcase. NOT a shop. Aesthetic = silence, dust, parchment, old house.
Rule: if a feature "speeds up perception" or "looks like a store" — it doesn't belong.

## Dual deployment
`isTauri` flag (`src/lib/api.ts:18`) → Tauri IPC; else → fetch `/api/v1`.
All API calls go through the `api` object in `src/lib/api.ts`. Never call fetch directly.

## Routes
`/` home · `/figurines` archive · `/figurines/[id]` detail · `/author` · `/workshop` · `/upcoming` (not in nav) · `/admin`

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
- Booking creates a `cancelToken` — user saves it to cancel later (no account system)
- `FigurineClaimsStore` persists tokens in localStorage, re-verifies on mount + tab focus
- Gallery keyboard nav: ←/→ arrows (undiscoverable — no hint shown)
- `view-transition-name: figurine-{id}` enables shared-element morph card→detail; `startViewTransition` is wired globally in `+layout.svelte` (onNavigate). Each `figurine-{id}` must be unique per rendered page or the transition aborts.
- Admin auth: `sessionStorage` only (lost on tab close by design)
- `series` field exists in types but is not editable in admin form yet
