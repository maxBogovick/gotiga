import { browser } from '$app/environment';
import { get } from 'svelte/store';
import { api } from '$lib/api';
import { interact } from '$lib/analytics';
import { lang } from '$lib/i18n';
import type { PushTopic } from '$lib/types/api';

// Сайт как приложение: установка на экран «Домой», новая версия, работа без сети,
// число на значке и уведомления. Собрано в одном месте: об одном и том же — «установлено
// ли», «есть ли Service Worker» — спрашивают шапка, нижняя панель, подвал и страница.
//
// Сам Service Worker — `src/pwa/sw.js`; здесь только то, что нужно странице.

export const PUSH_TOPICS: readonly PushTopic[] = ['works', 'tales', 'gazette', 'game'];

/** Событие Chrome/Edge/Android: браузер готов показать собственный диалог установки. */
interface InstallPrompt extends Event {
  prompt(): Promise<void>;
  userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>;
}

/** `prompt` — родной диалог; `ios` — на iPhone/iPad только вручную через «Поделиться». */
export type InstallMode = 'prompt' | 'ios' | 'none';

const STRIP_KEY = 'gotiga_pwa_strip';
const PUSH_KEY = 'gotiga_push';
/** Закрытую полосу установки не показываем столько дней. */
const STRIP_QUIET_DAYS = 30;
const UPDATE_EVERY_MS = 60 * 60 * 1000;

const isWebBuild = import.meta.env.VITE_BUILD_TARGET === 'web';

function isStandalone(): boolean {
  if (!browser) return false;
  return (
    window.matchMedia('(display-mode: standalone)').matches ||
    window.matchMedia('(display-mode: window-controls-overlay)').matches ||
    (navigator as Navigator & { standalone?: boolean }).standalone === true
  );
}

function detectIos(): boolean {
  if (!browser) return false;
  const ua = navigator.userAgent;
  if (/iphone|ipad|ipod/i.test(ua)) return true;
  // iPadOS 13+ называет себя Mac, но у него есть касания.
  return navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1;
}

/**
 * На iPhone и iPad сайт добавляют на экран «Домой» из панели «Поделиться», а она есть
 * не везде. Её нет во встроенных браузерах (Telegram, Instagram, Facebook, WeChat — это
 * WebView без слова `Safari/` в строке браузера) и в Firefox для iOS. Приглашение туда,
 * где нужного пункта нет, — это инструкция, которую нельзя выполнить: а на сайт как раз
 * приходят из Telegram-канала.
 */
function iosCanAddToHomeScreen(): boolean {
  if (!detectIos()) return false;
  const ua = navigator.userAgent;
  if (!/safari\//i.test(ua)) return false;
  return !/fxios|fban|fbav|instagram|micromessenger|line\/|telegram/i.test(ua);
}

function keyBytes(base64: string): Uint8Array {
  const pad = '='.repeat((4 - (base64.length % 4)) % 4);
  const raw = atob((base64 + pad).replace(/-/g, '+').replace(/_/g, '/'));
  return Uint8Array.from(raw, (c) => c.charCodeAt(0));
}

/** Что устройство помнит об уведомлениях: темы, подписано ли и когда запись последний раз
 *  подтверждали серверу. */
type PushPrefs = { topics: PushTopic[]; on: boolean; syncedAt: number; endpoint: string };

/** Запись о подписке подтверждается серверу не чаще раза в сутки (см. `loadPush`). */
const PUSH_RESYNC_MS = 24 * 60 * 60 * 1000;
/** Кэш, из которого читает Service Worker: он не видит localStorage, а при смене подписки
 *  браузером ему нужны темы и язык (`pushsubscriptionchange` в `sw.js`). */
const PUSH_PREFS_CACHE = 'push-prefs';
const PUSH_PREFS_URL = '/__push-prefs';

function readPush(): PushPrefs {
  const fallback: PushPrefs = { topics: [...PUSH_TOPICS], on: false, syncedAt: 0, endpoint: '' };
  if (!browser) return fallback;
  try {
    const parsed = JSON.parse(localStorage.getItem(PUSH_KEY) ?? 'null') as Partial<PushPrefs> | null;
    if (!parsed) return fallback;
    const known = Array.isArray(parsed.topics)
      ? parsed.topics.filter((x): x is PushTopic => PUSH_TOPICS.includes(x as PushTopic))
      : [];
    return {
      // Пустой выбор не хранится: подписка без тем ничего бы не присылала.
      topics: known.length > 0 ? known : [...PUSH_TOPICS],
      on: parsed.on === true,
      syncedAt: typeof parsed.syncedAt === 'number' ? parsed.syncedAt : 0,
      endpoint: typeof parsed.endpoint === 'string' ? parsed.endpoint : '',
    };
  } catch {
    return fallback; // хранилище закрыто или запись испорчена: берём все темы
  }
}

function writePush(patch: Partial<PushPrefs>) {
  try {
    localStorage.setItem(PUSH_KEY, JSON.stringify({ ...readPush(), ...patch }));
  } catch {
    // без хранилища выбор тем не помнится — подписка от этого не страдает
  }
}

class PwaStore {
  /** Открыто как установленное приложение, а не вкладка. */
  standalone = $state(false);
  /** iPhone или iPad. */
  iosDevice = $state(false);
  /** …и в браузере, где есть «На экран “Домой”» (не Telegram, не Firefox). */
  iosInstallable = $state(false);
  /** Открыта подсказка установки для iPhone (стоит в раскладке, а не в кнопке — см. ниже). */
  guideOpen = $state(false);
  /** Нет сети сейчас. */
  offline = $state(false);
  /** Новая версия скачана и ждёт согласия. */
  updateReady = $state(false);

  #installEvent = $state.raw<InstallPrompt | null>(null);
  #updateSW: ((reload?: boolean) => Promise<void>) | null = null;
  #started = false;
  #lastUpdateCheck = 0;
  #pushLoaded = false;
  #pushWasOn = false;

  // ── Установка ──
  installMode: InstallMode = $derived(
    this.standalone
      ? 'none'
      : this.#installEvent
        ? 'prompt'
        : this.iosInstallable
          ? 'ios'
          : 'none',
  );

  // ── Уведомления ──
  pushSupported = $state(false);
  /** `null` — ещё не спрашивали сервер; `false` — на сервере уведомления не настроены. */
  pushConfigured = $state<boolean | null>(null);
  pushSubscribed = $state(false);
  pushTopics = $state<PushTopic[]>([...PUSH_TOPICS]);
  pushPermission = $state<NotificationPermission>('default');
  pushBusy = $state(false);
  pushFailed = $state(false);

  /** На iPhone уведомления приходят только установленному приложению. */
  pushNeedsInstall = $derived(this.iosInstallable && !this.standalone);

  constructor() {
    if (!browser) return;
    this.standalone = isStandalone();
    this.iosDevice = detectIos();
    this.iosInstallable = iosCanAddToHomeScreen();
    this.offline = !navigator.onLine;
    this.pushPermission = 'Notification' in window ? Notification.permission : 'denied';
    this.pushTopics = readPush().topics;
    this.#pushWasOn = readPush().on;

    // Событие приходит один раз и рано: слушатель ставится при загрузке модуля.
    window.addEventListener('beforeinstallprompt', (e) => {
      e.preventDefault();
      this.#installEvent = e as InstallPrompt;
    });
    window.addEventListener('appinstalled', () => {
      this.#installEvent = null;
      this.standalone = true;
      interact('pwa_install', 'installed', null, false);
    });
    window
      .matchMedia('(display-mode: standalone)')
      .addEventListener('change', (e) => (this.standalone = e.matches));
    window.addEventListener('online', () => (this.offline = false));
    window.addEventListener('offline', () => (this.offline = true));
  }

  /** Регистрирует Service Worker и подключает проверку обновлений. Зовётся один раз. */
  async start() {
    if (!browser || this.#started) return;
    this.#started = true;
    if (!isWebBuild || !('serviceWorker' in navigator)) return;
    try {
      const { registerSW } = await import('virtual:pwa-register');
      this.#updateSW = registerSW({
        immediate: false,
        onNeedRefresh: () => {
          this.updateReady = true;
          interact('pwa_update', 'ready', null, false);
        },
        onRegisteredSW: (_url, registration) => {
          if (!registration) return;
          const check = () => {
            this.#lastUpdateCheck = Date.now();
            void registration.update().catch(() => {});
          };
          setInterval(check, UPDATE_EVERY_MS);
          // Вернулся во вкладку или в приложение — проверить, если давно не смотрели.
          document.addEventListener('visibilitychange', () => {
            if (
              document.visibilityState === 'visible' &&
              Date.now() - this.#lastUpdateCheck > 10 * 60 * 1000
            ) {
              check();
            }
          });
        },
      });
    } catch {
      return;
    }
    this.pushSupported =
      'PushManager' in window && 'Notification' in window && 'serviceWorker' in navigator;
    if (this.pushSupported) {
      let first = true;
      lang.subscribe(() => {
        if (first) {
          first = false;
          return;
        }
        void this.syncPushLang();
      });
      // Подписанному устройству запись подтверждается сама (см. `loadPush`). Остальным
      // сервер на каждой странице не нужен: его спрашивает панель, когда её видно.
      if (this.#pushWasOn) void this.loadPush();
    }
  }

  // ── Установка ──

  /** Показывает родной диалог. На iPhone не работает: там открывается объяснение. */
  async install(): Promise<'accepted' | 'dismissed' | 'none'> {
    const event = this.#installEvent;
    if (!event) return 'none';
    interact('pwa_install', 'prompt', null, false);
    await event.prompt();
    const { outcome } = await event.userChoice;
    // Событие одноразовое: после выбора браузер новое не пришлёт, пока не сменится состояние.
    this.#installEvent = null;
    interact('pwa_install', outcome, null, false);
    return outcome;
  }

  /** Подсказка установки на iPhone. Состояние держит хранилище, а окно стоит в раскладке:
   *  кнопки в листе «Ещё» и в полосе убираются сами при нажатии, и окно, жившее в кнопке,
   *  исчезало вместе с ней. */
  openGuide() {
    interact('pwa_install', 'ios_guide', null, false);
    this.guideOpen = true;
  }

  closeGuide() {
    this.guideOpen = false;
  }

  /** Полоса «установить» над нижней панелью: закрытую не повторяем 30 дней. */
  stripQuiet(): boolean {
    if (!browser) return true;
    try {
      const until = Number(localStorage.getItem(STRIP_KEY));
      return Number.isFinite(until) && until > Date.now();
    } catch {
      return false;
    }
  }

  quietStrip() {
    try {
      localStorage.setItem(STRIP_KEY, String(Date.now() + STRIP_QUIET_DAYS * 86_400_000));
    } catch {
      // без хранилища полоса вернётся при следующем заходе
    }
  }

  // ── Новая версия ──

  /** Человек согласился: новая версия включается, страница перезагружается. */
  async applyUpdate() {
    interact('pwa_update', 'applied', null, false);
    if (this.#updateSW) await this.#updateSW(true);
    else location.reload();
  }

  postponeUpdate() {
    this.updateReady = false;
  }

  // ── Значок приложения ──

  /** Число на значке установленного приложения; 0 — значок чистый. */
  setBadge(count: number) {
    if (!browser) return;
    const nav = navigator as Navigator & {
      setAppBadge?: (n?: number) => Promise<void>;
      clearAppBadge?: () => Promise<void>;
    };
    try {
      if (count > 0) void nav.setAppBadge?.(count)?.catch(() => {});
      else void nav.clearAppBadge?.()?.catch(() => {});
    } catch {
      // значка нет в этом браузере
    }
  }

  // ── Уведомления ──

  async #registration(): Promise<ServiceWorkerRegistration | null> {
    // `ready` не завершается, если Service Worker не зарегистрирован: ждём не дольше 4 с.
    return Promise.race([
      navigator.serviceWorker.ready,
      new Promise<null>((resolve) => setTimeout(() => resolve(null), 4000)),
    ]);
  }

  /** Темы и язык для Service Worker: при смене подписки браузером он сам ничего этого не знает. */
  async #shareWithWorker() {
    try {
      const cache = await caches.open(PUSH_PREFS_CACHE);
      await cache.put(
        PUSH_PREFS_URL,
        new Response(JSON.stringify({ topics: this.pushTopics, lang: get(lang) }), {
          headers: { 'Content-Type': 'application/json' },
        }),
      );
    } catch {
      // без Cache API worker просто не получит тем — сервер возьмёт их из старой записи
    }
  }

  async #sendSubscription(sub: PushSubscription, topics: PushTopic[]) {
    const json = sub.toJSON();
    if (!json.endpoint || !json.keys?.p256dh || !json.keys.auth) throw new Error('bad subscription');
    await api.pushSubscribe({
      endpoint: json.endpoint,
      keys: { p256dh: json.keys.p256dh, auth: json.keys.auth },
      lang: get(lang),
      topics,
    });
    writePush({ endpoint: json.endpoint, syncedAt: Date.now() });
    void this.#shareWithWorker();
  }

  /**
   * Узнаёт, умеют ли уведомления на этом сервере и подписано ли устройство. Зовётся, когда
   * панель уведомлений попала в поле зрения, и сама при запуске у уже подписанного:
   * остальным посетителям этот запрос не нужен, а он шёл бы на каждой загрузке страницы.
   */
  async loadPush() {
    if (this.#pushLoaded) return;
    this.#pushLoaded = true;
    const config = await api.getPushConfig().catch(() => null);
    this.pushConfigured = Boolean(config?.publicKey);
    if (!this.pushSupported) return;
    this.pushPermission = Notification.permission;
    const reg = await this.#registration();
    const sub = reg ? await reg.pushManager.getSubscription().catch(() => null) : null;
    this.pushSubscribed = Boolean(sub) && Notification.permission === 'granted';
    // Подписка есть, а права сняты в настройках браузера — сообщаем серверу, что адрес мёртв.
    if (sub && Notification.permission !== 'granted') {
      void api.pushUnsubscribe(sub.endpoint).catch(() => {});
      void sub.unsubscribe().catch(() => {});
      writePush({ on: false });
      return;
    }
    writePush({ on: this.pushSubscribed });
    // Браузер подписку помнит, а сервер мог её забыть (после пяти отказов подряд, после
    // восстановления базы): без этого панель говорила бы «включено», а уведомления не шли бы.
    // Запись идемпотентна и заодно освежает язык и темы. Не чаще раза в сутки с устройства:
    // за общим адресом (школа, оператор связи) подписчиков много, а лимит записей общий.
    // Другой адрес, чем при прошлой записи, — браузер сам сменил подписку: подтверждаем сразу.
    const saved = readPush();
    if (
      sub &&
      this.pushSubscribed &&
      (saved.endpoint !== sub.endpoint || Date.now() - saved.syncedAt > PUSH_RESYNC_MS)
    ) {
      try {
        await this.#sendSubscription(sub, this.pushTopics);
      } catch {
        // повторится при следующем заходе
      }
    } else if (sub) {
      void this.#shareWithWorker();
    }
  }

  /** Разрешение, подписка в браузере и запись на сервере. Тихо ничего не делает без нажатия. */
  async subscribePush(topics: PushTopic[] = this.pushTopics) {
    if (!this.pushSupported || this.pushBusy || topics.length === 0) return;
    this.pushBusy = true;
    this.pushFailed = false;
    try {
      const config = await api.getPushConfig();
      if (!config?.publicKey) {
        this.pushConfigured = false;
        return;
      }
      const permission = await Notification.requestPermission();
      this.pushPermission = permission;
      if (permission !== 'granted') {
        interact('push', 'denied', null, false);
        return;
      }
      const reg = await this.#registration();
      if (!reg) throw new Error('no service worker');
      const sub =
        (await reg.pushManager.getSubscription()) ??
        (await reg.pushManager.subscribe({
          userVisibleOnly: true,
          applicationServerKey: keyBytes(config.publicKey),
        }));
      this.pushTopics = topics;
      await this.#sendSubscription(sub, topics);
      writePush({ topics, on: true });
      this.pushSubscribed = true;
      interact('push', 'subscribed', topics.length, false);
    } catch {
      this.pushFailed = true;
    } finally {
      this.pushBusy = false;
    }
  }

  /**
   * Другие темы. Хотя бы одна остаётся всегда: подписка без тем ничего не присылала бы, а
   * выключается она своей кнопкой, а не тем, что сняли последнюю галочку.
   */
  async setPushTopics(topics: PushTopic[]) {
    if (topics.length === 0) return;
    this.pushTopics = topics;
    writePush({ topics });
    if (!this.pushSubscribed) return;
    this.pushBusy = true;
    this.pushFailed = false;
    try {
      const reg = await this.#registration();
      const sub = reg ? await reg.pushManager.getSubscription() : null;
      if (!sub) throw new Error('no subscription');
      await this.#sendSubscription(sub, topics);
    } catch {
      this.pushFailed = true;
    } finally {
      this.pushBusy = false;
    }
  }

  async unsubscribePush() {
    if (!this.pushSupported) return;
    this.pushBusy = true;
    this.pushFailed = false;
    try {
      const reg = await this.#registration();
      const sub = reg ? await reg.pushManager.getSubscription() : null;
      if (sub) {
        await api.pushUnsubscribe(sub.endpoint).catch(() => {});
        await sub.unsubscribe();
      }
      this.pushSubscribed = false;
      writePush({ on: false });
      interact('push', 'unsubscribed', null, false);
    } catch {
      this.pushFailed = true;
    } finally {
      this.pushBusy = false;
    }
  }

  /** Язык сайта сменился: уведомления приходят на новом. */
  async syncPushLang() {
    if (!this.pushSubscribed) return;
    await this.setPushTopics(this.pushTopics);
  }
}

export const pwa = new PwaStore();
