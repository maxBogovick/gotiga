// Service worker сайта (vite-plugin-pwa, стратегия `injectManifest`; сборка — vite.config.js).
// Лежит не в `src/service-worker.*`: это имя SvelteKit подхватил бы сам и собрал бы второй раз.
//
// Что он делает:
//  1. Предкэш: скрипты, стили и картинки самого сайта (список подставляет сборка).
//  2. Страницы: сначала сеть (4 с), при отказе — копия страницы, которую человек уже открывал;
//     страницы, которых на устройстве нет, заменяет `offline.html`.
//  3. Данные и картинки работ: те же правила, что были у прежнего workbox-файла, но
//     общие данные хранятся 30 дней (раньше 5 минут — без сети открытая работа пропадала).
//  4. Новая версия ждёт: страница спрашивает человека (`pwa.svelte.ts`) и присылает
//     `SKIP_WAITING`; молча перезагружать вкладку с недописанным письмом нельзя.
//  5. Push-уведомления о новом на сайте (работы, байки, листы вестника).
//
// Проверку типов этот файл не проходит намеренно: у него другая глобальная область (worker),
// и подключение `lib.webworker` ломает типы всех страниц.
// @ts-nocheck

import { clientsClaim } from 'workbox-core';
import {
  precacheAndRoute,
  cleanupOutdatedCaches,
  matchPrecache,
} from 'workbox-precaching';
import { registerRoute, setCatchHandler } from 'workbox-routing';
import {
  CacheFirst,
  NetworkFirst,
  NetworkOnly,
  StaleWhileRevalidate,
} from 'workbox-strategies';
import { ExpirationPlugin } from 'workbox-expiration';
import { CacheableResponsePlugin } from 'workbox-cacheable-response';

const DAY = 60 * 60 * 24;
const API = '/api/v1';

self.__WB_DISABLE_DEV_LOGS = true;

precacheAndRoute(self.__WB_MANIFEST);
cleanupOutdatedCaches();
clientsClaim();

// Обновление применяется только по просьбе страницы.
self.addEventListener('message', (event) => {
  if (event.data && event.data.type === 'SKIP_WAITING') self.skipWaiting();
});

// ── Что не кэшируется вовсе ────────────────────────────────────────────────

// Пути, в которых лежит чей-то ключ или личное дело: копия на устройстве — утечка.
const PRIVATE_PATHS = [
  '/admin',
  '/auth/',
  '/profile',
  '/bookings',
  '/cancel/',
  '/certificate/',
  '/unsubscribe/',
  '/subscribe/confirm/',
  '/gazette/watch/',
];
const isPrivatePath = (pathname) => PRIVATE_PATHS.some((p) => pathname.startsWith(p));

// Запрос от имени вошедшего (ключ в заголовке): ответ только для него, в общий кэш не идёт.
const isAuthed = (request) => request.headers.has('Authorization');

// ── Страницы ───────────────────────────────────────────────────────────────

// Ключ копии страницы — без меток источника: `?src=pwa` и `?utm_…` не отдельные страницы.
const pageKey = {
  cacheKeyWillBeUsed: async ({ request }) => {
    const url = new URL(request.url);
    url.searchParams.delete('src');
    for (const k of [...url.searchParams.keys()]) {
      if (k.startsWith('utm_')) url.searchParams.delete(k);
    }
    return url.href;
  },
};

// Личные страницы идут мимо кэша. Маршрут стоит раньше общего: workbox берёт первый подошедший.
registerRoute(
  ({ request, url }) => request.mode === 'navigate' && isPrivatePath(url.pathname),
  new NetworkOnly(),
);

const NOT_PAGES = [/^\/api\//, /^\/static\//, /^\/_app\//, /\/feed\.xml$/, /\/sitemap\.xml$/];

registerRoute(
  ({ request, url }) =>
    request.mode === 'navigate' && !NOT_PAGES.some((re) => re.test(url.pathname)),
  new NetworkFirst({
    cacheName: 'pages',
    networkTimeoutSeconds: 4,
    plugins: [
      pageKey,
      new CacheableResponsePlugin({ statuses: [200] }),
      new ExpirationPlugin({ maxEntries: 80, maxAgeSeconds: 30 * DAY, purgeOnQuotaError: true }),
    ],
  }),
);

// Нет сети и нет копии: страница «нет сети» из предкэша.
setCatchHandler(async ({ request }) => {
  if (request.destination === 'document') {
    const offline = await matchPrecache('offline.html');
    if (offline) return offline;
  }
  return Response.error();
});

// ── Шрифты и общие файлы сайта ─────────────────────────────────────────────

registerRoute(
  ({ url }) => /\/fonts\/.*\.woff2$/i.test(url.pathname),
  new CacheFirst({
    cacheName: 'self-hosted-fonts',
    plugins: [new ExpirationPlugin({ maxEntries: 40, maxAgeSeconds: 365 * DAY })],
  }),
);
registerRoute(
  ({ url }) => url.pathname === '/fonts/fonts.css',
  new StaleWhileRevalidate({ cacheName: 'self-hosted-fonts-css' }),
);
registerRoute(
  ({ url }) => url.origin === 'https://fonts.googleapis.com',
  new StaleWhileRevalidate({ cacheName: 'gfonts-stylesheets' }),
);
registerRoute(
  ({ url }) => url.origin === 'https://fonts.gstatic.com',
  new CacheFirst({
    cacheName: 'gfonts-webfonts',
    plugins: [
      new CacheableResponsePlugin({ statuses: [0, 200] }),
      new ExpirationPlugin({ maxAgeSeconds: 365 * DAY }),
    ],
  }),
);

// Куски кода, которые не вошли в предкэш (панели, открываемые по требованию).
registerRoute(
  ({ url }) => url.origin === self.location.origin && url.pathname.startsWith('/_app/immutable/'),
  new CacheFirst({
    cacheName: 'app-chunks',
    plugins: [new ExpirationPlugin({ maxEntries: 200, maxAgeSeconds: 30 * DAY })],
  }),
);

// ── Данные ─────────────────────────────────────────────────────────────────

// Язык, предложенный для страны, зависит от того, кто спрашивает.
registerRoute(({ url }) => url.pathname === `${API}/site/locale`, new NetworkOnly());

// Всё, что требует входа, несёт ключ в адресе или служит самому сайту: только сеть, копии нет.
// Ключ в адресе — это брони, заявки, лист ожидания, сертификаты, подписки и отписки
// (`/bookings/cancel/<ключ>`, `/certificates/<ключ>`, …): ответ несёт имя и почту, и копия на
// общем компьютере пережила бы выход. Запросы не-GET маршрутами не ловятся вовсе.
const PRIVATE_API =
  /^\/api\/v1\/(admin|auth|push|analytics|bookings|commissions|orders|waitlist|subscribe|unsubscribe|certificates|gazette\/watch|studio|users)(\/|$)/;
registerRoute(
  ({ url, request }) => PRIVATE_API.test(url.pathname) || isAuthed(request),
  new NetworkOnly(),
);
// Ответы на запросы поиска и вопросов: у каждого нажатия свой адрес, и сто пятьдесят таких
// записей вытеснили бы из кэша списки работ и листов, ради которых он нужен без сети.
registerRoute(
  ({ url }) => /^\/api\/v1\/(search|ask)(\/|$)/.test(url.pathname),
  new NetworkOnly(),
);
registerRoute(
  ({ url }) => url.pathname.startsWith(`${API}/`),
  new NetworkFirst({
    // Имя новое: в прежнем `api-cache` лежат ответы с ключами в адресах (см. выше), и срок их
    // хранения новым правилом стал бы 30 дней. Прежний кэш стирается при активации.
    cacheName: 'api-v2',
    networkTimeoutSeconds: 5,
    plugins: [
      new CacheableResponsePlugin({ statuses: [200] }),
      new ExpirationPlugin({ maxEntries: 150, maxAgeSeconds: 30 * DAY, purgeOnQuotaError: true }),
    ],
  }),
);

// ── Картинки и файлы автора ────────────────────────────────────────────────

// Рисунки игры и байок лежат под тем же именем, что и страницы (`/battles/…`, `/tales/…`),
// и в предкэш не входят (около 19 МБ): кэшируются, когда их впервые показали. Файл подменяют
// по тому же имени (перерисованная земля, движение, обложка), поэтому «сначала копия, потом
// проверка», а не «копия на месяц»: прежде такой файл обновляла выкладка через предкэш.
registerRoute(
  ({ url, request }) =>
    request.mode !== 'navigate' &&
    url.origin === self.location.origin &&
    /^\/(battles|tales)\//.test(url.pathname) &&
    /\.(png|webp|jpe?g|gif|svg)$/i.test(url.pathname),
  new StaleWhileRevalidate({
    cacheName: 'site-art',
    plugins: [
      new CacheableResponsePlugin({ statuses: [200] }),
      new ExpirationPlugin({ maxEntries: 400, maxAgeSeconds: 30 * DAY, purgeOnQuotaError: true }),
    ],
  }),
);

// Видео и звук не кэшируются: большие и идут частями (Range).
registerRoute(
  ({ url }) => /\.(mp4|webm|mov|m4v|mp3|ogg|wav|m4a)$/i.test(url.pathname),
  new NetworkOnly(),
);

// Фон кабинета перезаписывается по тому же адресу: сначала старый, потом новый.
registerRoute(
  ({ url }) => url.pathname.startsWith('/static/backgrounds/'),
  new StaleWhileRevalidate({ cacheName: 'backend-background' }),
);
// Под этими папками файл называется по содержимому (uuid на каждую загрузку, nginx отдаёт
// их как неизменяемые): копия верна всегда, и ходить в сеть незачем.
registerRoute(
  ({ url }) => /^\/static\/(images|videos|audio|avatars|seals)\//.test(url.pathname),
  new CacheFirst({
    cacheName: 'static-immutable',
    plugins: [
      new CacheableResponsePlugin({ statuses: [200] }),
      new ExpirationPlugin({ maxEntries: 500, maxAgeSeconds: 30 * DAY, purgeOnQuotaError: true }),
    ],
  }),
);
// Остальное под /static/ (рамки, листы, обложки) подменяют по тому же имени: сначала копия,
// потом проверка.
registerRoute(
  ({ url }) => url.pathname.startsWith('/static/'),
  new StaleWhileRevalidate({
    cacheName: 'static-live',
    plugins: [
      new CacheableResponsePlugin({ statuses: [200] }),
      new ExpirationPlugin({ maxEntries: 300, maxAgeSeconds: 30 * DAY, purgeOnQuotaError: true }),
    ],
  }),
);

// При каждой новой версии:
//  - стираются кэши прежних правил (в них копии, которые новые правила хранили бы дольше или
//    не должны были хранить вовсе);
//  - стираются копии страниц. В них записаны адреса скриптов прежней сборки (`/_app/immutable/
//    entry/start.<хэш>.js`); у новой версии эти файлы уже не лежат ни в предкэше, ни на сервере,
//    и такая страница, поданная из кэша (нет сети или сеть медленнее четырёх секунд), показала
//    бы текст, но не ожила бы. Страницы набираются заново по мере просмотра.
self.addEventListener('activate', (event) => {
  event.waitUntil(
    Promise.all(['api-cache', 'backend-static', 'pages'].map((name) => caches.delete(name))),
  );
});

// ── Push ───────────────────────────────────────────────────────────────────

// Сервер присылает `{title, body, url, tag, image?}` (services/push.rs).
self.addEventListener('push', (event) => {
  let data = {};
  try {
    data = event.data ? event.data.json() : {};
  } catch {
    data = { body: event.data ? event.data.text() : '' };
  }
  const title = data.title || 'Ritunia';
  const options = {
    body: data.body || '',
    icon: '/pwa/icon-192.png',
    badge: '/pwa/icon-monochrome-512.png',
    tag: data.tag || undefined,
    // Одна запись на тег: вторая по тому же поводу заменит первую, а не дописывается.
    renotify: false,
    image: data.image || undefined,
    lang: data.lang || undefined,
    data: { url: data.url || '/' },
  };
  event.waitUntil(self.registration.showNotification(title, options));
});

self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  const target = new URL(
    (event.notification.data && event.notification.data.url) || '/',
    self.location.origin,
  );
  // Уведомление ведёт только на этот сайт, что бы ни пришло в сообщении.
  if (target.origin !== self.location.origin) return;
  // Метка источника: по ней в замере видно, сколько людей привели уведомления.
  if (!target.searchParams.has('src')) target.searchParams.set('src', 'push');
  event.waitUntil(
    (async () => {
      const all = await self.clients.matchAll({ type: 'window', includeUncontrolled: true });
      // Окно уже на этой странице — просто показать его.
      const same = all.find((client) => {
        const url = new URL(client.url);
        return url.origin === self.location.origin && url.pathname === target.pathname;
      });
      if (same && 'focus' in same) {
        await same.focus();
        return;
      }
      // Иначе новое окно. Открытую вкладку на другой странице не уводим: в ней могло лежать
      // недописанное письмо. У установленного приложения `launch_handler` манифеста
      // (`navigate-existing`) сам направит его единственное окно на эту страницу.
      await self.clients.openWindow(target.href);
    })(),
  );
});

function keyBytes(base64) {
  const pad = '='.repeat((4 - (base64.length % 4)) % 4);
  const raw = atob((base64 + pad).replace(/-/g, '+').replace(/_/g, '/'));
  return Uint8Array.from(raw, (c) => c.charCodeAt(0));
}

// Браузер сам сменил подписку (ключ устарел): подписываемся заново. Темы и язык читателя
// лежат на странице, а worker их не видит, поэтому страница оставляет их в кэше `push-prefs`
// (`pwa.svelte.ts`); сервер к тому же берёт их из прежней записи по `oldEndpoint`.
self.addEventListener('pushsubscriptionchange', (event) => {
  event.waitUntil(
    (async () => {
      try {
        const config = await (await fetch(`${API}/push/config`)).json();
        if (!config.publicKey) return;
        const sub = await self.registration.pushManager.subscribe({
          userVisibleOnly: true,
          applicationServerKey: keyBytes(config.publicKey),
        });
        let prefs = {};
        try {
          const saved = await (await caches.open('push-prefs')).match('/__push-prefs');
          if (saved) prefs = await saved.json();
        } catch {
          /* темы возьмёт сервер из прежней записи */
        }
        const { endpoint, keys } = sub.toJSON();
        await fetch(`${API}/push/subscribe`, {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({
            endpoint,
            keys,
            topics: Array.isArray(prefs.topics) ? prefs.topics : [],
            lang: typeof prefs.lang === 'string' ? prefs.lang : '',
            oldEndpoint: event.oldSubscription ? event.oldSubscription.endpoint : null,
          }),
        });
      } catch {
        /* следующий заход на сайт подтвердит подписку сам (loadPush) */
      }
    })(),
  );
});
