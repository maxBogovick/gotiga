<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { allClaims } from '$lib/stores/all-claims.svelte';
  import { authStore } from '$lib/stores/auth.svelte';
  import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
  import { t, lang, brandName } from '$lib/i18n';
  import { fade } from 'svelte/transition';
  import { api, resolveMediaUrl } from '$lib/api';
  import LangSwitcher from '$lib/components/LangSwitcher.svelte';
  import RavenWatcher from '$lib/components/RavenWatcher.svelte';
  import NeighborPlate from '$lib/components/NeighborPlate.svelte';
  import { afterLoadIdle } from '$lib/after-load-idle';
  import { fitRow } from '$lib/actions/fitRow';
  import { dismissable } from '$lib/actions/dismissable';
  import HeaderNotifications from '$lib/components/HeaderNotifications.svelte';
  import UserMenuPanel from '$lib/components/UserMenuPanel.svelte';
  import { isNavActive, GAME_HREF, WORK_LEAF_RE } from '$lib/nav';
  import { keeper } from '$lib/stores/keeper.svelte';
  import { userDesk } from '$lib/stores/user-desk.svelte';
  import { navCounts, SECTION_HREF, type NavSection } from '$lib/stores/nav-counts.svelte';
  import NavCount from '$lib/components/NavCount.svelte';
  import { interact } from '$lib/analytics';
  import SectionSeal from '$lib/components/SectionSeal.svelte';
  import { navPhoto } from '$lib/stores/nav-photo.svelte';
  import { navSeals, sealSrcset } from '$lib/stores/nav-seals.svelte';
  import type { NavSealKey } from '$lib/types/api';
  import TaleBookFace from '$lib/components/TaleBookFace.svelte';
  import { neighborTitle, leafCoverUrl } from '$lib/gazette';
  import { taleTone } from '$lib/tales';
  import SiteSearch, { SITE_SEARCH_OPEN } from '$lib/components/SiteSearch.svelte';
  import type { SectionIconName } from '$lib/components/SectionIcon.svelte';
  import { ARCHIVE_KEEPER_INPUT_ID } from '$lib/keeper-search';
  import { edgeMask } from '$lib/headerPortrait';
  import { COLLAPSE_AT, restoredCollapsed, restoredScrollY } from '$lib/restored-scroll';
  import type { AuthorProfile, Figurine, FigurineListItem, HeaderDecor, HeaderPortraitShown } from '$lib/types/api';

  type WorkLeafData = {
    figurine?: Figurine | null;
    prev?: FigurineListItem | null;
    next?: FigurineListItem | null;
  };

  // Header-left maker avatar — admin-editable photo + frame styling.
  let authorProfile = $state<AuthorProfile | null>(null);
  let headerAvatarUrl = $derived(authorProfile?.heroPhotoUrl?.trim() || '/images/avatar.jpg');
  let headerAuthorName = $derived(authorProfile?.name?.trim() || $brandName);
  let brandCap = $derived($brandName.trim().charAt(0));
  let brandRest = $derived($brandName.trim().slice(1));
  let avatarShape = $derived(authorProfile?.avatarShape ?? 'circle');
  let avatarRadiusCss = $derived(avatarShape === 'square' ? `${authorProfile?.avatarRadius ?? 12}px` : '50%');
  let avatarBorderWidthCss = $derived(`${authorProfile?.avatarBorderWidth ?? 3.5}px`);
  let avatarBorderColor = $derived(authorProfile?.avatarBorderColor?.trim() || 'var(--copper)');
  let avatarBg = $derived(authorProfile?.avatarBg?.trim() || 'transparent');

  // Картинки по краям шапки — только на широком экране. Ниже порога шапка
  // стоит вплотную к краям и места им нет; там их не запрашивают вовсе, а
  // CSS не подключает картинку, поэтому телефон не качает ни байта.
  const DECOR_MIN_WIDTH = '(min-width: 1440px)';
  let decor = $state<HeaderDecor | null>(null);
  const DECOR_DEFAULT_WIDTH = 260;
  const DECOR_DEFAULT_FADE = 35;
  function decorStyle(side: HeaderDecor['left']): string | null {
    const url = resolveMediaUrl(side.image?.trim());
    if (!url) return null;
    return `--edge-img: url("${url}"); --edge-w: ${side.width ?? DECOR_DEFAULT_WIDTH}px; --edge-fade: ${side.fade ?? DECOR_DEFAULT_FADE}%;`;
  }
  let decorLeft = $derived(decor ? decorStyle(decor.left) : null);
  let decorRight = $derived(decor ? decorStyle(decor.right) : null);

  // Выполняет `run`, когда окно подходит под запрос: сразу, если уже подходит, а
  // если окно растянули позже — один раз при этом. Возвращает отмену: прежде
  // слушатель оставался на `matchMedia` после ухода шапки.
  function whenMedia(query: string, run: () => void): () => void {
    if (typeof window === 'undefined') return () => {};
    const mq = window.matchMedia(query);
    if (mq.matches) {
      run();
      return () => {};
    }
    const onChange = (e: MediaQueryListEvent) => {
      if (!e.matches) return;
      mq.removeEventListener('change', onChange);
      run();
    };
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  }

  function loadDecor(): () => void {
    return whenMedia(DECOR_MIN_WIDTH, () => {
      api.getHeaderDecor().then((d) => { decor = d; }).catch(() => {});
    });
  }

  // Готовая картинка автора для свёрнутой шапки на компьютере (собирается в
  // админке: композиция с фотографией). На телефоне и планшете шапка прежняя,
  // поэтому и спрашивается только на широком окне.
  const PORTRAIT_MIN_WIDTH = '(min-width: 1025px)';
  const PORTRAIT_H = 58;
  let portrait = $state<HeaderPortraitShown | null>(null);
  let portraitUrl = $derived(resolveMediaUrl(portrait?.image?.trim()));
  let portraitMask = $derived(edgeMask(portrait?.edge));
  let portraitW = $derived(
    portrait && portrait.height > 0 ? Math.round((PORTRAIT_H * portrait.width) / portrait.height) : 0,
  );

  function loadPortrait(): () => void {
    return whenMedia(PORTRAIT_MIN_WIDTH, () => {
      api.getHeaderPortrait().then((p) => { portrait = p; }).catch(() => {});
    });
  }

  // Ряд печатей и левая колонка (портрет автора) существуют только на компьютере;
  // на телефоне они скрыты стилем. Скрытая `<img>` всё равно качается, поэтому
  // картинки для них не создаются, пока окно не широкое.
  let wide = $state(false);

  // Фото автора нужно в свёрнутой шапке и на странице работы. В развёрнутой левая
  // колонка скрыта (`display: none`), но файл всё равно качался бы: 55 КБ на каждой
  // странице ради картинки, которой не видно. Запрос начинается с первого движения
  // прокрутки (раньше, чем шапка свернётся за 48 px), поэтому к свёртке файл
  // уже на месте; раз начавшись, он не снимается.
  const AVATAR_PREFETCH_AT = 8;
  let avatarWanted = $state(restoredScrollY() > AVATAR_PREFETCH_AT);

  let rightLinks = $derived([
    // Студия стоит в навигации, а не за дверью, как полка битв: на неё
    // приходят из соцсетей, и комната, которую надо знать по адресу,
    // посетителей не собирает.
    { href: '/studio',   label: $t('navStudio') },
  ]);

  // Ряд печатей на компьютере: все семь разделов на виду, «Ещё» не нужно.
  // Четыре главных — крупные печати с числами, три служебных — малые.
  // Знак у раздела тот же, что на нижней панели телефона и в листе поиска.
  type SealLink = {
    href: string;
    label: string;
    icon: SectionIconName;
    section: NavSection | null;
    minor: boolean;
  };
  /** Поперечник печати в развёрнутой шапке: вторая строка высотой 72 px, оправа и
   *  зазор занимают ещё по 4 px сверху и снизу, так что печать стоит на всю высоту
   *  строки. В свёрнутой шапке размер задаёт `--seal-size`. */
  const SEAL_SIZE = 58;

  /** Готовые картинки-медальоны на печатях (прозрачный фон, `static/seals/`). */
  const SEAL_ART: Partial<Record<SectionIconName, string>> = {
    gazette: '/seals/gazette.webp',
    studio: '/seals/studio.webp',
    game: '/seals/game.webp',
  };
  /** Как выглядит печать раздела. Если автор загрузил для него картинки в админке
   *  («Автор → Печати меню»), стоит текущая из них; иначе то, что сайт рисует сам:
   *  фото случайной работы, обложка последней небылицы или готовая картинка. Пока
   *  ответ с настройкой не пришёл, готовые картинки не показываются: иначе
   *  картинка по умолчанию мелькала бы перед картинкой автора. */
  function sealLook(link: SealLink) {
    // Узкое окно: ряд скрыт, а его картинки всё равно качались бы.
    if (!wide) {
      return {
        photo: null,
        tale: false,
        art: '',
        artSet: '',
        artStyle: 'free' as const,
        shape: link.section === 'tales' || link.href === GAME_HREF ? ('rect' as const) : ('round' as const),
        ratio: link.href === GAME_HREF ? GAME_SEAL_RATIO : 2 / 3,
      };
    }
    const key = link.icon as NavSealKey;
    const cfg = navSeals.of(key);
    const cur = navSeals.current(key);
    if (cfg && cur) {
      const first = cfg.images[0];
      return {
        photo: null,
        tale: false,
        art: resolveMediaUrl(cur.url) ?? cur.url,
        artSet: sealSrcset(cur),
        artStyle: cfg.shape === 'circle' ? ('circle' as const) : ('free' as const),
        shape: cfg.shape === 'rect' ? ('rect' as const) : ('round' as const),
        ratio: first.width / first.height,
      };
    }
    return {
      photo: link.section === 'works' ? navPhoto.works : null,
      tale: link.section === 'tales' && Boolean(navPhoto.tale),
      art: navSeals.ready ? (SEAL_ART[link.icon] ?? '') : '',
      artSet: '',
      artStyle: 'free' as const,
      shape: link.section === 'tales' || link.href === GAME_HREF ? ('rect' as const) : ('round' as const),
      ratio: link.href === GAME_HREF ? GAME_SEAL_RATIO : 2 / 3,
    };
  }
  /** Ширина к высоте у картинки карты на печати «Игра» (`static/seals/game.webp`). */
  const GAME_SEAL_RATIO = 454 / 572;
  let sealLinks = $derived<SealLink[]>([
    { href: '/figurines', label: $t('navAllWorks'), icon: 'works', section: 'works', minor: false },
    { href: '/gazette', label: $t('navGazette'), icon: 'gazette', section: 'gazette', minor: false },
    { href: '/tales', label: $t('navTales'), icon: 'tales', section: 'tales', minor: false },
    { href: GAME_HREF, label: $t('navGame'), icon: 'game', section: null, minor: false },
    { href: '/studio', label: $t('navStudio'), icon: 'studio', section: null, minor: true },
  ]);

  // Объявлен раньше всего, что его читает: при отрисовке на сервере `$derived` считается
  // сразу, и чтение `pathname` до его объявления роняло пререндер главной.
  let pathname = $derived(page.url.pathname);

  let isHome = $derived(pathname === '/');

  /** Раздел, чьё число стоит у пункта: работы, вестник, небылицы. У остальных числа нет. */
  function sectionOf(href: string): NavSection | null {
    const hit = (Object.keys(SECTION_HREF) as NavSection[]).find((s) => SECTION_HREF[s] === href);
    return hit ?? null;
  }

  // Открыт именно список раздела (не страница работы или небылицы): всё, что на
  // нём есть, увидено. Страница работы архив «просмотренным» не делает.
  $effect(() => {
    const here = (Object.keys(SECTION_HREF) as NavSection[]).find(
      (s) => pathname === SECTION_HREF[s] || pathname === `${SECTION_HREF[s]}/`,
    );
    void navCounts.counts;
    if (here) navCounts.markSeen(here);
  });
  let leftLinks = $derived([
    { href: '/figurines', label: $t('navAllWorks') },
    { href: '/gazette', label: $t('navGazette') },
    { href: '/tales', label: $t('navTales') },
    // Игра стоит в навигации наравне с байками: пока её не было в шапке, на
    // неё попадали только по ссылке. Ведёт на карту этюдов — с неё начинают.
    { href: GAME_HREF, label: $t('navGame') },
  ]);
  let leafData = $derived(page.data as WorkLeafData);
  let isWorkLeaf = $derived(WORK_LEAF_RE.test(pathname) && Boolean(leafData.figurine));
  let leafPrev = $derived(leafData.prev ?? null);
  let leafNext = $derived(leafData.next ?? null);

  function jumpToWorkshopStory(event: MouseEvent) {
    if (pathname === '/') {
      event.preventDefault();
      document.getElementById('in-the-workshop')?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
    // Elsewhere: let the link navigate to "/#in-the-workshop" normally —
    // the browser scrolls there once the home page has mounted.
  }

  let mobileNavOpen = $state(false);
  // Возврат на середину страницы: шапка сразу сложена, как скрипт в app.html сложил её в HTML.
  let isScrolled = $state(restoredCollapsed());

  // On the home page, the maker avatar stays out of sight until the visitor
  // has scrolled past the hero photograph — it shouldn't compete with that
  // first image. Everywhere else it's just always there. Seeded from the
  // pathname (not just `true`) so a home-page load doesn't flash the avatar
  // in before the first scroll check hides it again.
  let avatarVisible = $state(pathname !== '/');
  // Neighbour plates / leaf tools are code-split. Don't render the {#await}
  // during prerender or hydration would flash an empty pending state against
  // HTML that already contained them.
  let leafChrome = $state(false);
  let birdReady = $state(false);
  let stopBird: (() => void) | null = null;
  // Всё, что надо снять при уходе шапки: слушатели `matchMedia`, отложенные чтения.
  const cleanups: (() => void)[] = [];

  // Фото автора на главной появляется, когда первый кадр ушёл выше окна. Следит
  // за этим наблюдатель, а не обработчик прокрутки: прежде на каждом событии
  // `scroll` читался `getBoundingClientRect`, то есть принудительный расчёт
  // раскладки.
  let heroObserver: IntersectionObserver | null = null;
  let heroRetry: ReturnType<typeof setTimeout> | undefined;

  function watchHero(attempt = 0) {
    heroObserver?.disconnect();
    heroObserver = null;
    clearTimeout(heroRetry);
    if (pathname !== '/') {
      avatarVisible = true;
      return;
    }
    const frame = document.querySelector('.cine-frame');
    if (!frame) {
      // Разметка главной ещё собирается: спросить позже, но не дольше двух секунд.
      avatarVisible = true;
      if (attempt < 14) heroRetry = setTimeout(() => watchHero(attempt + 1), 150);
      return;
    }
    heroObserver = new IntersectionObserver(([entry]) => {
      avatarVisible = !entry.isIntersecting && entry.boundingClientRect.bottom <= 0;
    });
    heroObserver.observe(frame);
  }

  // Правило «какой пункт горит» общее с нижней панелью разделов на телефоне.
  function isActive(href: string) {
    return isNavActive(pathname, href);
  }

  function toggleMobileNav() {
    mobileNavOpen = !mobileNavOpen;
    if (mobileNavOpen) {
      mobileContactOpen = false;
    }
  }
  function closeMobileNav() { mobileNavOpen = false; }

  // Свёртка с запасом: сворачивается за 48 px, разворачивается ближе к верху (за
  // 16 px). С одним порогом в обе стороны шапка на границе дрожала от каждого
  // толчка колеса.
  const EXPAND_AT = 16;
  // Первый заход не играет смену вида: шапка из разметки уже стоит как надо.
  let settling = $state(false);

  function handleScroll() {
    const y = window.scrollY;
    if (y > AVATAR_PREFETCH_AT) avatarWanted = true;
    const next = isScrolled ? y > EXPAND_AT : y > COLLAPSE_AT;
    if (next !== isScrolled) {
      settling = true;
      isScrolled = next;
    }
  }

  // Одно чтение за кадр, а не за каждое событие прокрутки.
  let scrollFrame = 0;
  function onScroll() {
    if (scrollFrame) return;
    scrollFrame = requestAnimationFrame(() => {
      scrollFrame = 0;
      handleScroll();
    });
  }

  // Высоту шапки знает только CSS (`--site-header-h` в app.css, по ширине окна);
  // здесь выставляются лишь два признака её состояния на <html>. Прежде число
  // писал сюда JS, и до монтирования, а на телефоне и после него, оно расходилось
  // с настоящей высотой (78 px при 58 px).
  $effect(() => {
    if (typeof document === 'undefined') return;
    const root = document.documentElement.classList;
    root.toggle('hdr-leaf', isWorkLeaf);
    root.toggle('hdr-scrolled', isScrolled && !isWorkLeaf);
  });

  // ── Меню вошедшего ──────────────────────────────────────────────────────
  //
  // Прежде в нём стояло два пункта — «Профиль →» и «Выйти», — то есть нажатие
  // покупало один переход, который аватар мог дать сам (на телефоне он так и
  // делает: один элемент, два разных поведения). Теперь меню называет разделы
  // с их числами и работает с любой страницы, минуя первый экран профиля, —
  // ради этого его и открывают.
  let userMenuOpen = $state(false);

  function closeUserMenu() {
    userMenuOpen = false;
  }

  function toggleUserMenu() {
    userMenuOpen = !userMenuOpen;
    // Числа разделов набираются в миг открытия и остаются набранными: тот же
    // стол потом читает и сам профиль, поэтому это не лишний запрос, а
    // заблаговременный.
    if (userMenuOpen) void userDesk.load();
  }

  // "Write to the author" panel — a quill icon always in reach, so the
  // lightweight contact form (see ContactMessageForm) is one click away
  // from any page, not just the home correspondence card.
  let contactPanelOpen = $state(false);

  function toggleContactPanel() { contactPanelOpen = !contactPanelOpen; }
  function closeContactPanel() { contactPanelOpen = false; }

  // Mobile mirror of the "write to the author" quill. The desktop contact button
  // lives in .ghost-right-group (hidden on mobile), so the compact mobile bar gets
  // its own button + panel next to the account icon. Separate open-state and ref so
  // its outside-click handler doesn't fight the desktop one.
  let mobileContactOpen = $state(false);
  let mobileContactRef = $state<HTMLElement | null>(null);

  function toggleMobileContact() {
    mobileContactOpen = !mobileContactOpen;
    if (mobileContactOpen) {
      mobileNavOpen = false;
    }
  }
  function closeMobileContact() { mobileContactOpen = false; }

  function openWriteFromMenu() {
    mobileNavOpen = false;
    mobileContactOpen = true;
  }

  function isArchiveList(path: string) {
    return path === '/figurines' || path === '/figurines/';
  }

  function typingInField(el: EventTarget | null) {
    if (!(el instanceof HTMLElement)) return false;
    const tag = el.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
    return el.isContentEditable;
  }

  // Escape закрывает меню вошедшего; «/» ставит курсор в поиск. На архиве и на
  // первом экране главной у страницы своё поле, и «/» ведёт туда: два поля на
  // одном экране отвечали бы на одну клавишу по-разному.
  // Что в шапке нажимают: одно имя на всё («header_click»), в `detail` — что
  // именно. Раньше считались только поиск и «Спросить сайт», и сказать, нужен
  // ли пункту ряда печатей или значку справа свой вес, было нечем. Отметка
  // идёт по классам, а не вешается на каждый элемент: так новая кнопка
  // не остаётся незамеченной по забывчивости. Повторные нажатия считаются.
  const SEAL_NAME: Record<string, string> = {
    '/figurines': 'works', '/gazette': 'gazette', '/tales': 'tales',
    '/studio': 'studio',
  };
  function headerClickName(el: Element): string | null {
    if (el.matches('.seal-link')) {
      const href = (el as HTMLAnchorElement).getAttribute('href') ?? '';
      return 'seal:' + (SEAL_NAME[href] ?? (href === GAME_HREF ? 'game' : 'other'));
    }
    if (el.matches('.contact-btn, .mobile-contact-btn, .mobile-nav-link--write')) return 'act:write';
    if (el.matches('.mobile-menu-btn')) return 'menu';
    if (el.matches('.mobile-nav-link')) return 'mobile_nav';
    if (el.matches('.user-btn, .mobile-user-btn')) return 'act:profile';
    if (el.matches('.ghost-right-group .seal')) return 'act:lang';
    if (el.matches('.brand')) return 'brand';
    if (el.matches('.header-avatar')) return 'author_photo';
    return null;
  }
  function handleHeaderClick(e: MouseEvent) {
    const hit = (e.target as Element | null)?.closest?.('.site-header a, .site-header button');
    const name = hit ? headerClickName(hit) : null;
    if (name) interact('header_click', name, null, false);
  }

  // Escape закрывает все панели шапки одним действием `dismissable`; здесь
  // осталась только клавиша «/».
  function refreshCounts() {
    if (!document.hidden) void navCounts.load();
  }

  function handleHeaderKey(e: KeyboardEvent) {
    // ⌘K / Ctrl+K открывает поиск с вопросами из любого места, в том числе из поля ввода.
    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      window.dispatchEvent(new Event(SITE_SEARCH_OPEN));
      return;
    }
    if (e.key !== '/' || e.metaKey || e.ctrlKey || e.altKey) return;
    if (typingInField(e.target)) return;
    // Открыто модальное окно: поиск под ним был бы невидим.
    if (document.querySelector('dialog[open]')) return;
    e.preventDefault();
    if (isArchiveList(pathname)) {
      const el = document.getElementById(ARCHIVE_KEEPER_INPUT_ID);
      if (el instanceof HTMLInputElement) {
        el.focus();
        el.select();
        return;
      }
    }
    if (isHome && !keeper.blotterOffscreen) {
      const el = document.querySelector<HTMLInputElement>('.keeper-slot .kn-input');
      if (el) {
        el.focus();
        return;
      }
    }
    userMenuOpen = false;
    contactPanelOpen = false;
    mobileContactOpen = false;
    mobileNavOpen = false;
    window.dispatchEvent(new Event(SITE_SEARCH_OPEN));
  }

  async function handleLogout() {
    userMenuOpen = false;
    mobileNavOpen = false;
    const token = authStore.token;
    if (token) {
      try { await api.userLogout(token); } catch { /* ok */ }
    }
    userDesk.reset();
    authStore.logout();
    goto('/');
  }

  let avatarUrl = $derived(resolveMediaUrl(authStore.user?.avatarUrl));

  onMount(async () => {
    leafChrome = true;
    stopBird = afterLoadIdle(() => { birdReady = true; });
    allClaims.load();
    allClaims.verify();
    allClaims.startPolling();
    // Числа у пунктов — украшение, поэтому спрашиваются после загрузки страницы.
    // Окно широкое сейчас или станет таким: от этого зависят картинки печатей и
    // фото автора. На телефоне настройка печатей не спрашивается вовсе.
    const wideMq = window.matchMedia(PORTRAIT_MIN_WIDTH);
    const onWideChange = () => { wide = wideMq.matches; };
    onWideChange();
    wideMq.addEventListener('change', onWideChange);
    cleanups.push(() => wideMq.removeEventListener('change', onWideChange));
    cleanups.push(whenMedia(PORTRAIT_MIN_WIDTH, () => navSeals.load()));
    cleanups.push(afterLoadIdle(() => { void navCounts.load(); }));
    document.addEventListener('keydown', handleHeaderKey);
    document.addEventListener('click', handleHeaderClick, { capture: true });
    // Вкладка долго открыта: числа у пунктов перечитываются, когда в неё вернулись
    // (чаще раза в пять минут стор не спрашивает).
    document.addEventListener('visibilitychange', refreshCounts);
    window.addEventListener('scroll', onScroll, { passive: true });
    handleScroll();

    api.getAuthorProfile().then((p) => { authorProfile = p; }).catch(() => {});
    cleanups.push(loadDecor(), loadPortrait());

    if (!authStore.isLoggedIn && authStore.token) {
      try {
        const user = await api.userMe(authStore.token);
        authStore.user = user;
      } catch {
        authStore.clearSession();
      }
    }

    if (authStore.token) {
      savedFigurines.load();
      savedFigurines.syncWithServer({ importLocal: false });
      // Одно число, ради которого над аватаром и стоит точка: ответ мастерской
      // прежде был виден только тому, кто сам зашёл в профиль и прокликал два
      // уровня.
      void userDesk.refreshUnread();
    }
  });

  onDestroy(() => {
    stopBird?.();
    heroObserver?.disconnect();
    clearTimeout(heroRetry);
    for (const stop of cleanups) stop();
    allClaims.stopPolling();
    if (typeof document !== 'undefined') {
      document.removeEventListener('keydown', handleHeaderKey);
      document.removeEventListener('click', handleHeaderClick, { capture: true });
      document.removeEventListener('visibilitychange', refreshCounts);
    }
    if (typeof window !== 'undefined') {
      window.removeEventListener('scroll', onScroll);
      cancelAnimationFrame(scrollFrame);
    }
    if (typeof document !== 'undefined') {
      document.documentElement.classList.remove('hdr-leaf', 'hdr-scrolled');
    }
  });

  $effect(() => {
    void pathname;
    mobileNavOpen = false;
    mobileContactOpen = false;
    // Wait a tick so the new page's markup (and its .cine-frame, if any)
    // has mounted before we measure it.
    requestAnimationFrame(() => watchHero());
  });
</script>

<header class="site-header" class:is-scrolled={isScrolled} class:is-settling={settling} class:is-leaf={isWorkLeaf} class:is-home={isHome}>
  {#if birdReady}
    {#await import('$lib/components/HeaderBirdWalk.svelte') then { default: HeaderBirdWalk }}
      <HeaderBirdWalk />
    {/await}
  {/if}
  <!-- Картинки по краям: позади содержимого, мышь пропускают. На странице
       работы шапка — плавающая капсула без краёв, там им нечего украшать. -->
  {#if !isWorkLeaf}
    {#if decorLeft}<span class="edge edge-left" style={decorLeft} aria-hidden="true"></span>{/if}
    {#if decorRight}<span class="edge edge-right" style={decorRight} aria-hidden="true"></span>{/if}
  {/if}
  <div class="header-inner">

    <!-- Левая и правая части равны по ширине, пока хватает места, поэтому логотип
         стоит ровно посередине; ниже этого он смещается так же, как прежде. -->
    <div class="side side-left">
    <!-- ① Ghost-left: maker avatar pinned to left edge -->
    <div class="ghost-left">
      <!-- Maker avatar: large, full colour, fills the header's height —
           a real face, not a muted icon. -->
      <a
        class="header-avatar"
        class:is-hidden={!avatarVisible}
        href="/#in-the-workshop"
        onclick={jumpToWorkshopStory}
        aria-label={$t('headerAuthorAbout').replace('{name}', headerAuthorName)}
        aria-hidden={!avatarVisible}
        tabindex={avatarVisible ? 0 : -1}
        style="
          --avatar-radius: {avatarRadiusCss};
          --avatar-border-w: {avatarBorderWidthCss};
          --avatar-border-color: {avatarBorderColor};
          --avatar-bg: {avatarBg};
          {portraitW > 0 && portraitUrl ? `--portrait-w: ${portraitW}px; --portrait-gap-l: ${portrait?.edge?.gapLeft ?? 0}px; --portrait-gap-r: ${portrait?.edge?.gapRight ?? 0}px;` : ''}
        "
      >
        <!-- На странице работы фото автора стоит в капсуле всегда и должно быть в HTML из пререндера
             (`wide` там ещё ложно): иначе оно появлялось бы только после гидрации. На телефоне
             левая колонка скрыта, а `loading="lazy"` скрытую картинку не качает (на странице
             работы капсула лежит в шапке нулевой высоты, и ленивая картинка там не
             запрашивается вовсе, поэтому для неё загрузка обычная). В остальных случаях —
             только на компьютере и после первого движения прокрутки. -->
        {#if avatarVisible && (isWorkLeaf || (wide && avatarWanted))}
        <img src={headerAvatarUrl} alt="" class="header-avatar-img" class:has-portrait={!!portraitUrl} decoding="async" loading={isWorkLeaf ? 'eager' : 'lazy'} />
        {#if portraitUrl && portraitW > 0}
          <img
            src={portraitUrl}
            alt=""
            class="header-portrait-img"
            width={portraitW}
            height={PORTRAIT_H}
            loading="lazy"
            style={portraitMask ? `-webkit-mask-image: ${portraitMask}; mask-image: ${portraitMask};` : undefined}
            decoding="async"
          />
        {/if}
        {/if}
        <span class="header-avatar-tip">{headerAuthorName}</span>
      </a>
      {#if isWorkLeaf}
        <a href="/figurines" class="leaf-archive">
          <svg class="leaf-archive-arrow" width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true">
            <path d="M6.5 2L3.5 5 6.5 8"/>
          </svg>
          {$t('navAllWorks')}
        </a>
      {/if}
    </div>


    </div>

    <!-- ③ Brand: proscenium centerpiece. On a work leaf, neighbour daguerreotypes
         sit immediately left and right of the raven. -->
    <div class="brand-triptych">
      {#if isWorkLeaf}
        <NeighborPlate side="prev" work={leafPrev} />
      {/if}
      <a
        href="/"
        class="brand"
        aria-label={$brandName}
      >
        <span class="brand-inner">
          <!-- Ворон в кружке; на компьютере он стоит после имени (`order`). -->
          <span class="medal">
            <RavenWatcher />
          </span>
          <span class="brand-words">
            <span class="brand-name">
              <span class="brand-mark">
                <span class="brand-mark-cap">{brandCap}</span><span class="brand-mark-rest">{brandRest}</span>
              </span>
            </span>
            {#if !isWorkLeaf}
              <span class="brand-sub">{$t('brandTagline')}</span>
            {/if}
          </span>
        </span>
      </a>
      {#if isWorkLeaf}
        <NeighborPlate side="next" work={leafNext} />
      {/if}
    </div>

    <!-- Поиск: строка в верхнем ряду; когда шапка свёрнута прокруткой — лупа. -->
    {#if !isWorkLeaf}
      <div class="search-slot">
        <div class="search-field"><SiteSearch variant="field" /></div>
        <div class="search-loupe"><SiteSearch variant="button" /></div>
      </div>
    {/if}

    <div class="side side-right">

    <!-- ⑤ Ghost-right: muted utilities pinned to right edge. The "Write"
         button lives OUTSIDE .ghost-right on purpose: that container (and
         its :has()/:hover states) dims itself down to as little as 0.42
         opacity for the bookings/user icons, and CSS opacity on a parent
         composites its whole subtree as one layer — nesting the accented
         button (and its dropdown) in there made both render translucent no
         matter how opaque their own backgrounds were. -->
    <div class="ghost-right-group">
      <!-- Язык — одна печать с буквами; нажатие раскрывает листок из трёх строк. -->
      <LangSwitcher variant="light" />
      <div class="header-actions">
      {#if isWorkLeaf && leafChrome}
        {#await import('$lib/components/DetailHeaderTools.svelte') then { default: DetailHeaderTools }}
        <DetailHeaderTools />
        {/await}
      {/if}


      <div
        class="contact-anchor"
        use:dismissable={{ open: contactPanelOpen, close: closeContactPanel, focusPanel: true }}
      >
        <button
          class="contact-btn"
          class:is-open={contactPanelOpen}
          onclick={toggleContactPanel}
          aria-label={$t('headerContactLabel')}
          aria-haspopup="dialog"
          aria-expanded={contactPanelOpen}
          aria-controls="contact-panel"
          title={$t('headerContactLabel')}
        >
          <svg width="13" height="13" viewBox="0 0 14 14" fill="none" aria-hidden="true">
            <path d="M12.2 1.8c-3.6 0-8.4 3.4-9.9 7-.3.7.4 1.4 1.1 1.1l1.9-.8" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M12.2 1.8c0 3.6-3.4 8.4-7 9.9-.7.3-1.4-.4-1.1-1.1l.8-1.9" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M5.4 8.6L1.8 12.2" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
          </svg>
          <span class="contact-btn-label">{$t('headerContactShortLabel')}</span>
        </button>

        {#if contactPanelOpen}
          <div
            id="contact-panel"
            class="contact-panel"
            role="dialog"
            aria-label={$t('headerContactLabel')}
            tabindex="-1"
            data-panel
            transition:fade={{ duration: 150 }}
          >
            <div class="panel-head">
              <span class="panel-title">{$t('headerContactLabel')}</span>
              <button class="panel-close" onclick={closeContactPanel} aria-label={$t('uiClose')}>✕</button>
            </div>
            <p class="contact-panel-note">{$t('headerContactNote')}</p>
            {#await import('$lib/components/ContactMessageForm.svelte') then { default: ContactMessageForm }}
            <ContactMessageForm source="header" compact />
            {/await}
          </div>
        {/if}
      </div>

      <div class="ghost-right">
      <div
        class="user-anchor"
        use:dismissable={{ open: userMenuOpen, close: closeUserMenu }}
      >
        <button
          class="user-btn"
          class:logged-in={authStore.isLoggedIn}
          class:has-initial={authStore.isLoggedIn && !avatarUrl}
          class:is-open={userMenuOpen}
          onclick={authStore.isLoggedIn ? toggleUserMenu : () => goto('/login')}
          aria-label={authStore.isLoggedIn
            ? (userDesk.counts.unread > 0
                ? `${authStore.user?.displayName} — ${$t('profileLatestNewReplies')}`
                : authStore.user?.displayName)
            : $t('authLogin')}
          title={authStore.isLoggedIn ? authStore.user?.displayName : $t('authLogin')}
          aria-expanded={authStore.isLoggedIn ? userMenuOpen : undefined}
          aria-controls={authStore.isLoggedIn ? 'user-panel' : undefined}
        >
          {#if authStore.isLoggedIn}
            {#if avatarUrl}
              <img src={avatarUrl} alt="" class="user-avatar" />
            {:else}
              <span class="user-initial">{(authStore.user?.displayName ?? '?')[0].toUpperCase()}</span>
            {/if}
            {#if userDesk.counts.unread > 0}
              <!-- Точка, а не число: в 28 px число нечитаемо, а вопрос у точки
                   один — «есть ли что-то новое». Сколько именно, говорит меню. -->
              <span class="user-dot" aria-hidden="true"></span>
            {/if}
          {:else}
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
              <circle cx="7" cy="4.5" r="2.5" stroke="currentColor" stroke-width="1"/>
              <path d="M1.5 13c0-3 2.5-4.5 5.5-4.5S12 10 12 13" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
            </svg>
          {/if}
        </button>

        {#if userMenuOpen && authStore.isLoggedIn}
          <UserMenuPanel onclose={closeUserMenu} onlogout={handleLogout} />
        {/if}
      </div>
      </div>
      </div>
    </div>

    </div>

    <!-- Предмет на печати «Небылицы»: обложка последней небылицы, та же `TaleBookFace`,
         что на полке. У «Игры» готовая картинка (`SEAL_ART`). -->
    {#snippet taleSeal()}
      {#if navPhoto.tale}
        <TaleBookFace
          title={neighborTitle(navPhoto.tale, $lang)}
          photo={leafCoverUrl(navPhoto.tale)}
          bookCover={navPhoto.tale.bookCover ?? ''}
          tone={taleTone(navPhoto.tale.id)}
          sizes="64px"
          eager
        />
      {/if}
    {/snippet}

    <!-- Ряд печатей: разделы на компьютере. Текущий раздел — сургуч, остальные —
         оттиск в бумаге. На телефоне разделы в нижней панели (`SectionBar`). -->
    {#if !isWorkLeaf}
      <nav class="seal-row" aria-label={$t('dockSections')} use:fitRow>
        {#each sealLinks as link (link.href)}
          {@const on = isActive(link.href)}
          {@const total = link.section ? navCounts.total(link.section) : null}
          {@const fresh = link.section && !on ? navCounts.fresh(link.section) : 0}
          {@const look = sealLook(link)}
          <a
            href={link.href}
            class="seal-link seal-host"
            class:seal-link--minor={link.minor}
            class:is-active={on}
            aria-current={on ? 'page' : undefined}
            data-label={link.label}
            title={link.minor ? link.label : undefined}
          >
            <SectionSeal
              icon={link.icon}
              size={SEAL_SIZE}
              photo={look.photo}
              art={look.art}
              artSet={look.artSet}
              artStyle={look.artStyle}
              piece={look.tale ? taleSeal : undefined}
              shape={look.shape}
              ratio={look.ratio}
            />
            <span class="seal-text">
              <span class="seal-name">{link.label}</span>
              <!-- Вторая строка есть у каждого крупного пункта, даже пустая: пока число
                   не пришло, строка пустая, но держит высоту, и название не прыгает.
                   У всех троих в ней число; у «Игры» это число этюдов. -->
              {#if !link.minor}
                {@const shown = link.href === GAME_HREF ? navCounts.etudes : total}
                <span
                  class="seal-num"
                  class:seal-num--empty={link.href !== GAME_HREF && (shown ?? 0) === 0 && fresh === 0}
                >
                  {#if link.href === GAME_HREF}
                    <!-- Пока число этюдов не пришло (старый сервер, сеть), прежняя подпись:
                         пустая строка под «Игрой» выглядела бы сломанной. -->
                    {shown ? $t('navGameStudies').replace('{n}', String(shown)) : $t('navGameSub')}
                  {:else}
                    {#if total}{total}{/if}
                    {#if fresh > 0}<em title={$t('navNewSince').replace('{n}', String(fresh))}>{$t('navFreshShort').replace('{n}', String(fresh))}</em>{/if}
                  {/if}
                </span>
              {/if}
            </span>
          </a>
          {#if link.href === GAME_HREF}<span class="seal-gap" aria-hidden="true"></span>{/if}
        {/each}
      </nav>
    {/if}

    <!-- ⑥ Mobile-only controls -->
    <div class="mobile-controls">
      {#if !isWorkLeaf}
        <div class="mobile-search"><SiteSearch variant="button" /></div>
      {/if}
      <div class="mobile-contact-anchor" bind:this={mobileContactRef}>
        <button
          class="mobile-contact-btn"
          class:is-open={mobileContactOpen}
          onclick={toggleMobileContact}
          aria-label={$t('headerContactLabel')}
          aria-haspopup="dialog"
          aria-expanded={mobileContactOpen}
          aria-controls="mobile-contact-panel"
          title={$t('headerContactLabel')}
        >
          <svg width="15" height="15" viewBox="0 0 14 14" fill="none" aria-hidden="true">
            <path d="M12.2 1.8c-3.6 0-8.4 3.4-9.9 7-.3.7.4 1.4 1.1 1.1l1.9-.8" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M12.2 1.8c0 3.6-3.4 8.4-7 9.9-.7.3-1.4-.4-1.1-1.1l.8-1.9" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M5.4 8.6L1.8 12.2" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
      <button
        class="mobile-user-btn"
        class:has-initial={authStore.isLoggedIn && !avatarUrl}
        onclick={authStore.isLoggedIn ? () => { closeMobileNav(); goto('/profile'); } : () => goto('/login')}
        aria-label={authStore.isLoggedIn
          ? (userDesk.counts.unread > 0
              ? `${authStore.user?.displayName} — ${$t('profileLatestNewReplies')}`
              : authStore.user?.displayName)
          : $t('authLogin')}
      >
        {#if authStore.isLoggedIn}
          {#if avatarUrl}
            <img src={avatarUrl} alt="" class="user-avatar" />
          {:else}
            <span class="user-initial">{(authStore.user?.displayName ?? '?')[0].toUpperCase()}</span>
          {/if}
          {#if userDesk.counts.unread > 0}
            <span class="user-dot" aria-hidden="true"></span>
          {/if}
        {:else}
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
            <circle cx="7" cy="4.5" r="2.5" stroke="currentColor" stroke-width="1"/>
            <path d="M1.5 13c0-3 2.5-4.5 5.5-4.5S12 10 12 13" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
          </svg>
        {/if}
      </button>
      <!-- Гамбургер только на странице работы: там нижней панели разделов нет
           (внизу закреплена кнопка действия), и разделы остаются в этом меню.
           На всех остальных страницах телефона разделы — в `SectionBar`. -->
      {#if isWorkLeaf}
      <button
        class="mobile-menu-btn"
        class:is-open={mobileNavOpen}
        type="button"
        onclick={toggleMobileNav}
        aria-label={$t('navMenu')}
        aria-expanded={mobileNavOpen}
        aria-controls="site-mobile-nav"
      >
        <span></span>
        <span></span>
        <span></span>
      </button>
      {/if}
    </div>

  </div>


  <!-- Mobile nav dropdown — только на странице работы (см. гамбургер выше). -->
  {#if isWorkLeaf}
  <nav
    id="site-mobile-nav"
    class="mobile-nav"
    class:is-open={mobileNavOpen}
    aria-label={$t('navPrimaryLabel')}
    aria-hidden={!mobileNavOpen}
    inert={!mobileNavOpen}
    use:dismissable={{
      open: mobileNavOpen,
      close: closeMobileNav,
      trigger: () => document.querySelector<HTMLElement>('.mobile-menu-btn'),
      inside: () => [document.querySelector('.mobile-menu-btn')],
    }}
  >
    {#each [...leftLinks, ...rightLinks] as link (link.href)}
      <a
        href={link.href}
        class="mobile-nav-link"
        class:is-active={isActive(link.href)}
        aria-current={isActive(link.href) ? 'page' : undefined}
        onclick={closeMobileNav}
      >{link.label}{#if sectionOf(link.href)}<NavCount section={sectionOf(link.href)!} active={isActive(link.href)} />{/if}</a>
    {/each}
    <div class="mobile-nav-lang">
      <LangSwitcher variant="light" layout="strip" />
    </div>
    <div class="mobile-nav-house">
      <button
        type="button"
        class="mobile-nav-link mobile-nav-link--write"
        onclick={openWriteFromMenu}
      >{$t('headerContactLabel')}</button>
      <a
        href={authStore.isLoggedIn ? '/profile' : '/login'}
        class="mobile-nav-link"
        onclick={closeMobileNav}
      >{authStore.isLoggedIn ? $t('profileTitle') : $t('authLogin')}</a>
    </div>
  </nav>
  {/if}
  {#if mobileContactOpen}
    <div
      id="mobile-contact-panel"
      class="contact-panel mobile-contact-panel"
      role="dialog"
      aria-label={$t('headerContactLabel')}
      tabindex="-1"
      data-panel
      use:dismissable={{
        open: mobileContactOpen,
        close: closeMobileContact,
        focusPanel: true,
        trigger: () => mobileContactRef?.querySelector('button'),
        inside: () => [mobileContactRef, document.querySelector('.mobile-nav-link--write')],
      }}
      transition:fade={{ duration: 150 }}
    >
      <div class="panel-head">
        <span class="panel-title">{$t('headerContactLabel')}</span>
        <button class="panel-close" onclick={closeMobileContact} aria-label={$t('uiClose')}>✕</button>
      </div>
      <p class="contact-panel-note">{$t('headerContactNote')}</p>
      {#await import('$lib/components/ContactMessageForm.svelte') then { default: ContactMessageForm }}
      <ContactMessageForm source="header" compact />
      {/await}
    </div>
  {/if}
</header>

<HeaderNotifications />

<style>
  /* ── Tokens ─────────────────────────────────────────────────── */
  .site-header {
    --ink:    var(--color-ink-primary);
    --mid:    var(--color-ember-deep);
    --copper: var(--color-ember);
    --border: color-mix(in srgb, var(--color-ink-primary) 10%, transparent);
    --ease:   cubic-bezier(0.16, 1, 0.3, 1);

    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    /* Высота — полоса телефона; на компьютере её задаёт блок «Две строки» ниже.
       Высота не анимируется: раскладка внутри меняется сразу, и плавная высота
       показывала бы 0,4 с свёрнутое содержимое в ещё высокой полосе. */
    height: 58px;
    background: rgba(248, 241, 231, 0.97);
    border-bottom: 1px solid var(--border);
    z-index: 200;
    transition: background 0.3s ease;
    /* Cornice of the house: stays while rooms slide (drawer / portiere). */
    view-transition-name: site-header;
  }

  .site-header.is-scrolled {
    background: rgba(248, 241, 231, 0.99);
  }

  /* ── Work leaf: floating island ────────────────────────────────
     The cornice leaves the full-width bar. The inner row becomes a
     parchment capsule that keeps every control visible on scroll —
     no collapse to a raven crumb. */
  .site-header.is-leaf {
    /* Zero-height so the full-width fixed layer has no bottom edge —
       Safari/Chrome otherwise paint a 1px hairline across the viewport
       even when background and border are none. The capsule lives in
       overflow of this box. */
    height: 0;
    overflow: visible;
    padding: 0 14px;
    background: none;
    border: none;
    box-shadow: none;
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
    pointer-events: none;
    transition: none;
    view-transition-name: none;
  }

  .site-header.is-leaf .header-inner,
  .site-header.is-leaf .mobile-controls {
    pointer-events: auto;
  }

  .site-header.is-leaf.is-scrolled {
    height: 0;
    background: none;
    box-shadow: none;
  }

  .site-header.is-leaf .header-inner {
    height: 56px;
    max-width: 1320px;
    padding: 0 8px 0 10px;
    background: rgba(248, 241, 231, 0.94);
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 10%, transparent);
    border-radius: 999px;
    box-shadow: none;
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
    view-transition-name: site-header;
  }

  .site-header.is-leaf.is-scrolled .header-inner {
    background: rgba(248, 241, 231, 0.97);
    box-shadow: none;
  }

  .site-header.is-leaf .header-avatar,
  .site-header.is-leaf.is-scrolled .header-avatar {
    width: 40px;
    height: 40px;
  }

  :global(html:has(.site-header.is-leaf) .with-site-header) {
    padding-top: 56px;
  }

  /* ── 5-column proscenium grid ────────────────────────────────
     ghost-l | nav-l | brand | nav-r | ghost-r
     The two outer minmax(0,1fr) columns absorb remaining space
     symmetrically so the brand stays perfectly centered even on
     ultrawide monitors. max-width caps at 1380px so nav links
     never fly too far from the raven.
  ──────────────────────────────────────────────────────────── */
  .header-inner {
    display: grid;
    /* POINT 1 — overlap made structurally impossible.
       The two outer tracks hold the "ghost" clusters (avatar + language/font
       switchers on the left; Write button + bookings/account icons on the
       right). With minmax(0,1fr) those tracks could shrink to ZERO while their
       flex content stayed full-width, so the content spilled OUT of its track
       and painted over the nav — the switchers on ARCHIVE/IN PROGRESS, the
       Write button on AUTHOR. minmax(auto,1fr) floors each track at its own
       min-content, so a ghost cluster can never be narrower than what it holds
       and therefore never overflows onto a neighbour. (We can't just clip the
       clusters: they host absolutely-positioned popovers — the Write form, the
       bookings/account menus, the font dropdown — that must escape the box.) */
    grid-template-columns: minmax(auto, 1fr) auto minmax(auto, 1fr);
    grid-template-rows: 1fr;
    /* stretch so nav-links can fill the full header height */
    align-items: stretch;
    width: 100%;
    max-width: 1380px;
    margin: 0 auto;
    padding: 0 clamp(20px, 4vw, 52px);
    height: 100%;
  }

  /* ── Картинки по краям шапки ─────────────────────────────────
     Только от 1440 px. Содержимое шапки — колонка до 1380 px с отступом
     52 px по сторонам; картинке отдано то, что остаётся снаружи неё (плюс
     сам отступ), поэтому она не лежит ни под аватаром, ни под пунктами
     меню. Меньше окно — меньше картинка, а ниже порога её нет вовсе: на
     телефоне и планшете шапка и без того занята до краёв. Фон назначается
     только внутри запроса, и в остальных случаях браузер файл не качает. */
  .edge { display: none; }

  @media (min-width: 1440px) {
    .edge {
      display: block;
      position: absolute;
      top: 0;
      bottom: 0;
      width: min(var(--edge-w, 260px), calc((100vw - 1380px) / 2 + 52px));
      background-image: var(--edge-img);
      background-repeat: no-repeat;
      background-size: cover;
      pointer-events: none;
      /* Под содержимым, но над фоном самой шапки (она держит свой контекст). */
      z-index: -1;
      animation: edge-in 0.6s ease both;
    }
    .edge-left {
      left: 0;
      background-position: left center;
      -webkit-mask-image: linear-gradient(to right, #000 calc(100% - var(--edge-fade, 35%)), transparent);
      mask-image: linear-gradient(to right, #000 calc(100% - var(--edge-fade, 35%)), transparent);
    }
    .edge-right {
      right: 0;
      background-position: right center;
      -webkit-mask-image: linear-gradient(to left, #000 calc(100% - var(--edge-fade, 35%)), transparent);
      mask-image: linear-gradient(to left, #000 calc(100% - var(--edge-fade, 35%)), transparent);
    }
  }

  @keyframes edge-in {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  @media (prefers-reduced-motion: reduce) {
    .edge { animation: none; }
  }

  /* ── Ghost zones ──────────────────────────────────────────── */
  .ghost-left,
  .ghost-right {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  /* Holds the always-accented "Write" button alongside the muted
     bookings/user cluster without being subject to the latter's opacity
     rules — see the markup comment by .ghost-right-group. */
  .ghost-right-group {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  /* Flattened on ordinary pages so the existing ghost-right-group flex
     layout is unchanged. On a work leaf this becomes the tool capsule. */
  .header-actions {
    display: contents;
  }

  /* .ghost-left itself stays at full opacity now that it carries the maker
     avatar — that must never look grey/dimmed. .ghost-right (bookings/account
     icons) stays muted until hovered.

     Приглушается ровно КНОПКА, а не `.ghost-right` целиком: прозрачность на
     предке склеивает всё его поддерево в один слой, и выпадающие панели
     (брони, меню вошедшего) рисовались сквозь себя — сквозь меню читался
     заголовок страницы, какой бы непрозрачной ни была его собственная
     бумага. */
  .ghost-right .user-btn {
    opacity: 0.85;
    transition: opacity 0.3s ease;
  }
  .ghost-right:hover .user-btn,
  .ghost-right:focus-within .user-btn {
    opacity: 1;
  }

  .ghost-left  { margin-right: auto; justify-content: flex-start; }
  .ghost-right-group { margin-left: auto; justify-content: flex-end; }

  /* Зазор между действиями справа (прежде здесь был и значок броней с числом):
     число наезжало на аватар. */
  .site-header:not(.is-leaf) .ghost-right {
    gap: 12px;
  }

  /* Две равные части вокруг логотипа: слева аватар и разделы (разделы вплотную к
     логотипу), справа «Студия», «Ещё» и действия (пункты вплотную к логотипу). */
  .side {
    display: flex;
    align-items: stretch;
    min-width: 0;
  }
  .side-left  { grid-column: 1; justify-content: flex-end; }
  .side-right { grid-column: 3; justify-content: flex-start; }

  /* ── Maker avatar — large, full colour, fills the header's height.
     No sepia/grayscale filter: the face must read as a real photo, not a
     muted icon. Sized off the header's own height so it scales down
     automatically with the .is-scrolled 78px→54px shrink. ── */
  .header-avatar {
    /* Admin-editable, overridden inline via style="--avatar-*"; these are
       just the defaults if the profile hasn't set anything yet. */
    --avatar-radius: 50%;
    --avatar-border-w: 3.5px;
    --avatar-border-color: var(--copper);
    --avatar-bg: transparent;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 62px;
    height: 62px;
    box-sizing: border-box;
    padding: 3px;
    flex-shrink: 0;
    border-radius: var(--avatar-radius);
    background: var(--avatar-bg);
    text-decoration: none;
    opacity: 1;
    transform: scale(1);
    transition: width 0.25s ease, height 0.25s ease, opacity 0.35s var(--ease), transform 0.35s var(--ease);
  }

  /* Shrink with the header's own 78px→54px scroll transition. */
  .site-header.is-scrolled .header-avatar {
    width: 44px;
    height: 44px;
  }

  /* Held out of sight on the home page until the hero photograph has
     scrolled past — see watchHero() in the script. */
  .header-avatar.is-hidden {
    opacity: 0;
    transform: scale(0.7);
    pointer-events: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .header-avatar {
      transition: width 0.25s ease, height 0.25s ease;
    }
  }

  .header-avatar-img {
    width: 100%;
    height: 100%;
    border-radius: var(--avatar-radius);
    object-fit: cover;
    object-position: center 20%;
    display: block;
    box-shadow:
      0 0 0 var(--avatar-border-w) var(--avatar-border-color),
      0 4px 14px rgba(52, 37, 28, 0.3);
    transition: box-shadow 0.22s ease, transform 0.22s ease;
  }

  .header-avatar:hover .header-avatar-img,
  .header-avatar:focus-visible .header-avatar-img {
    box-shadow:
      0 0 0 var(--avatar-border-w) var(--avatar-border-color),
      0 6px 20px rgba(52, 37, 28, 0.38);
    transform: scale(1.06);
  }

  .header-avatar:focus-visible {
    outline: 2px solid var(--copper);
    outline-offset: 3px;
  }

  /* Готовая картинка автора показывается только в свёрнутой шапке на
     компьютере (правила ниже); в остальных положениях стоит фото автора. */
  .header-portrait-img {
    display: none;
  }

  .header-avatar-tip {
    position: absolute;
    top: calc(100% + 10px);
    left: 50%;
    transform: translateX(-50%) translateY(-4px);
    z-index: 20;
    white-space: nowrap;
    padding: 4px 10px;
    font-family: 'Fraunces', Georgia, serif;
    font-size: 12px;
    font-style: italic;
    color: #f8f1e7;
    background: rgba(28, 16, 10, 0.85);
    backdrop-filter: blur(6px);
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.18s ease, transform 0.18s ease;
  }

  .header-avatar:hover .header-avatar-tip,
  .header-avatar:focus-visible .header-avatar-tip {
    opacity: 1;
    transform: translateX(-50%) translateY(0);
  }

  @media (prefers-reduced-motion: reduce) {
    .header-avatar:hover .header-avatar-img,
    .header-avatar:focus-visible .header-avatar-img {
      transform: none;
    }
  }

  /* bookings & user sit at normal opacity inside ghost-right
     when they have active state. Красится сама кнопка — по той же причине,
     по которой выше: прозрачность на `.ghost-right` уносила с собой и
     выпадающую панель. Вошедший — обычное положение дел, поэтому именно это
     правило и делало меню вошедшего полупрозрачным всегда. */
  .ghost-right .user-btn.logged-in,
  .ghost-right:hover .user-btn.logged-in,
  .ghost-right:focus-within .user-btn.logged-in {
    opacity: 1;
  }
  /* У кнопки с числом или с вошедшим именем приглушена только картинка:
     прозрачность самой кнопки уносила с собой и число на ней, и точку «есть
     новое», и кружок аватара становился бледным, как отключённый. */
  .ghost-right .user-btn.logged-in .user-avatar {
    opacity: 0.85;
    transition: opacity 0.3s ease;
  }
  .ghost-right:hover .user-btn.logged-in .user-avatar,
  .ghost-right:focus-within .user-btn.logged-in .user-avatar {
    opacity: 1;
  }


  /* ── Brand: theatrical centerpiece ───────────────────────── */
  .brand-triptych {
    grid-column: 2;
    align-self: center;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: clamp(20px, 2.4vw, 32px);
    min-width: 0;
  }

  .site-header.is-leaf .brand-triptych {
    grid-column: 2;
  }

  .brand {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    padding: 0 clamp(14px, 2.2vw, 34px);
    text-decoration: none;
    color: inherit;
    transition: opacity 0.3s;
    min-width: 0;
  }

  .site-header.is-leaf .brand {
    padding: 0 4px;
    flex-direction: row;
    gap: 0;
  }

  .site-header.is-leaf .brand-name {
    font-size: 1.45rem;
    white-space: nowrap;
  }

  .brand:hover { opacity: 1; }
  .site-header.is-leaf .brand:hover { opacity: 1; }

  /* inner row: emblem + wordmark side by side */
  .brand-inner {
    display: flex;
    align-items: center;
    gap: 13px;
  }

  .brand-name {
    font-family: var(--font-title);
    font-style: italic;
    font-weight: 500;
    font-size: clamp(1.7rem, 2.35vw, 2.15rem);
    font-optical-sizing: auto;
    letter-spacing: -0.02em;
    text-transform: none;
    line-height: 1;
    color: var(--ink);
    transition: color 0.45s var(--ease);
  }
  .brand:hover .brand-name {
    color: var(--copper);
  }
  .brand-mark {
    display: inline-flex;
    align-items: baseline;
  }
  .brand-mark-cap {
    font-size: 1.18em;
    letter-spacing: -0.04em;
    margin-right: 0.01em;
    line-height: 0.9;
  }

  @media (prefers-reduced-motion: reduce) {
    .brand-name { transition: none; }
  }

  /* subtitle: fades out on scroll */
  .brand-sub {
    font-family: var(--font-body);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--color-ink-secondary);
    line-height: 1;
    white-space: nowrap;
    /* scroll transition */
    opacity: 1;
    max-height: 14px;
    transform: translateY(0);
    transition:
      opacity 0.38s var(--ease),
      transform 0.38s var(--ease),
      max-height 0.38s var(--ease);
  }

  .site-header.is-scrolled .brand-sub {
    opacity: 0;
    max-height: 0;
    /* Невидимая подпись всё равно занимала свою ширину (238 px): логотип
       раздувался, ряд печатей сжимался и вылезал за свою колонку на черту
       перед поиском. */
    max-width: 0;
    overflow: hidden;
    transform: translateY(-5px);
    pointer-events: none;
  }

  .mobile-nav-lang {
    display: flex;
    justify-content: center;
    padding: 10px 16px 6px;
  }

  .leaf-archive {
    display: inline-flex;
    align-items: center;
    gap: 0.28rem;
    margin-left: 6px;
    padding: 0.15rem 0.2rem;
    color: var(--color-ink-tertiary);
    font-family: var(--font-body);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.10em;
    text-decoration: none;
    text-transform: uppercase;
    white-space: nowrap;
    transition: color 0.25s ease;
  }

  .leaf-archive:hover,
  .leaf-archive:focus-visible {
    color: var(--ink);
  }

  .leaf-archive:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--copper, #c65f3c) 55%, transparent);
    outline-offset: 3px;
  }

  .leaf-archive-arrow {
    flex: 0 0 auto;
    transition: transform 0.2s ease;
  }

  .leaf-archive:hover .leaf-archive-arrow {
    transform: translateX(-2px);
  }

  /* ── Work leaf: one copper capsule for every right-hand action ──
     Story, share, write, bookings and account share size, stroke and
     hover so they read as a single instrument, not mixed chrome. */
  .site-header.is-leaf .header-actions {
    display: flex;
    align-items: stretch;
    flex-shrink: 0;
    height: 40px;
    overflow: visible;
    background: color-mix(in srgb, var(--color-canvas-base, #f8f1e7) 78%, white);
    border: 1px solid #c65f3c;
    border-radius: 999px;
  }

  .site-header.is-leaf .header-actions :global(.leaf-tools),
  .site-header.is-leaf .header-actions .contact-anchor,
  .site-header.is-leaf .header-actions .ghost-right,
  .site-header.is-leaf .header-actions .user-anchor {
    display: flex;
    align-items: stretch;
    height: 100%;
    gap: 0;
    opacity: 1;
  }

  .site-header.is-leaf .ghost-right:hover,
  .site-header.is-leaf .ghost-right:focus-within,
  .site-header.is-leaf .ghost-right:has(.user-btn.logged-in) {
    opacity: 1;
  }

  .site-header.is-leaf .header-actions :global(.leaf-tool),
  .site-header.is-leaf .header-actions .contact-btn,
  .site-header.is-leaf .header-actions .user-btn {
    box-sizing: border-box;
    width: 40px;
    min-width: 40px;
    height: 100%;
    padding: 0;
    border: none;
    border-radius: 0;
    background: transparent;
    color: #c65f3c;
    box-shadow: inset 1px 0 0 color-mix(in srgb, #c65f3c 32%, transparent);
    transition: background 0.2s ease, color 0.2s ease;
  }

  .site-header.is-leaf .header-actions :global(.leaf-tool:first-child),
  .site-header.is-leaf .header-actions > :first-child > button:first-child {
    box-shadow: none;
    border-radius: 999px 0 0 999px;
  }

  .site-header.is-leaf .header-actions .user-btn {
    border-radius: 0 999px 999px 0;
  }

  .site-header.is-leaf .header-actions .contact-btn {
    width: auto;
    min-width: 0;
    padding: 0 14px 0 12px;
    gap: 7px;
  }

  .site-header.is-leaf .header-actions .user-btn.logged-in {
    border: none;
  }

  .site-header.is-leaf .header-actions .user-avatar {
    width: 22px;
    height: 22px;
  }

  .site-header.is-leaf .header-actions .user-initial {
    font-size: 13px;
  }

  .site-header.is-leaf .header-actions :global(.leaf-tool svg),
  .site-header.is-leaf .header-actions .contact-btn svg,
  .site-header.is-leaf .header-actions .user-btn svg {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
  }

  .site-header.is-leaf .header-actions :global(.leaf-tool:hover),
  .site-header.is-leaf .header-actions :global(.leaf-tool:focus-visible),
  .site-header.is-leaf .header-actions :global(.leaf-tool.is-on),
  .site-header.is-leaf .header-actions .contact-btn:hover,
  .site-header.is-leaf .header-actions .contact-btn.is-open,
  .site-header.is-leaf .header-actions .contact-btn:focus-visible,
  .site-header.is-leaf .header-actions .user-btn:hover,
  .site-header.is-leaf .header-actions .user-btn.is-open,
  .site-header.is-leaf .header-actions .user-btn:focus-visible {
    background: #c65f3c;
    color: #fff9f0;
  }

  .site-header.is-leaf .header-actions :global(.leaf-tool:focus-visible),
  .site-header.is-leaf .header-actions .contact-btn:focus-visible,
  .site-header.is-leaf .header-actions .user-btn:focus-visible {
    outline: 2px solid color-mix(in srgb, #c65f3c 55%, transparent);
    outline-offset: -2px;
  }

  /* ── Mobile controls + nav (hidden on desktop) ───────────── */
  .mobile-controls { display: none; }
  .mobile-nav { display: none; }

  /* ── Bookings dropdown panel ─────────────────────────────── */

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px 10px;
    border-bottom: 1px solid rgba(52, 37, 28, 0.08);
  }

  .panel-title {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
  }

  /* Кнопка закрытия: знак 14 px, область нажатия 44 px за счёт отрицательных
     отступов — шапка панели не растёт. */
  .panel-close {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    margin: -12px -14px -12px 0;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 14px;
    color: var(--color-ink-tertiary);
    padding: 0;
    line-height: 1;
    transition: color 0.2s;
  }
  .panel-close:hover { color: var(--color-ember-deep); }
  .panel-close:focus-visible {
    outline: 2px solid var(--color-ember);
    outline-offset: -4px;
  }

  /* ── Contact ("write to the author") button ─────────────────
     Deliberately NOT muted like the bookings/user icons — this is the
     answer to "how do I reach the maker," so it carries a permanent
     copper outline instead of blending in until hovered. Literal colour
     (not the --copper token) so it
     reads the same accent on parchment and over a photograph alike. */
  .contact-anchor { position: relative; }

  .contact-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 36px;
    padding: 0 14px 0 12px;
    background: transparent;
    /* Рамка видна только при наведении и в открытом виде: кнопка не должна
       быть самым громким местом шапки. Цвет текста темнее прежнего #c65f3c
       (3.5:1 на бумаге): #a8482a даёт 5.2:1 при кегле 10.5 px. */
    border: 1px solid transparent;
    border-radius: 2px;
    color: #a8482a;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    white-space: nowrap;
    cursor: pointer;
    transition: background 0.25s ease, color 0.25s ease, border-color 0.25s ease;
  }
  .contact-btn svg { flex-shrink: 0; }
  .contact-btn:hover,
  .contact-btn.is-open {
    background: #a8482a;
    border-color: #a8482a;
    color: #fff9f0;
  }
  .contact-btn:focus-visible {
    outline: 2px solid #a8482a;
    outline-offset: 2px;
  }

  .contact-panel:focus {
    outline: none;
  }

  .contact-panel {
    position: absolute;
    top: calc(100% + 10px);
    right: 0;
    width: 300px;
    padding: 4px 16px 18px;
    background: var(--color-canvas-sunken);
    border: 1px solid var(--color-border-default);
    box-shadow: var(--shadow-md);
    z-index: 300;
    font-family: Georgia, serif;
    color: var(--color-ink-primary);
  }
  .contact-panel .panel-head { margin: 0 -16px 14px; }
  .contact-panel-note {
    margin: 0 0 14px;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-style: italic;
    font-size: 14px;
    line-height: 1.5;
    color: var(--color-ink-secondary, #5f4636);
  }

  /* ── User button ─────────────────────────────────────────── */
  .user-anchor { position: relative; }

  .user-btn {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    background: none;
    border: 1px solid transparent;
    border-radius: 50%;
    padding: 0;
    cursor: pointer;
    color: var(--color-ink-secondary);
    transition: color 0.25s, border-color 0.25s, background 0.25s;
  }

  /* Не вошедший: тёмный круг со светлым знаком. Язык рядом — светлая печать с
     буквами, и два одинаковых контура подряд читались парой; вход — действие,
     и выглядеть должен как действие. */
  .user-btn:not(.logged-in) {
    background: var(--color-ink-secondary);
    color: var(--color-canvas-base);
  }
  .user-btn:not(.logged-in):hover,
  .user-btn:not(.logged-in):focus-visible {
    background: var(--color-ink-primary);
    color: var(--color-canvas-base);
  }
  .user-btn:not(.logged-in) svg {
    width: 16px;
    height: 16px;
  }

  .user-btn:hover,
  .user-btn.is-open { color: var(--mid); }

  /* Рамка и буква у вошедшего без аватара: ink-tertiary на бумаге 6.4:1.
     Прежняя рамка (10 % чернил) и приглушённая кнопка читались как
     отключённая. */
  .user-btn.logged-in {
    border-color: var(--color-ink-tertiary);
    color: var(--color-ink-secondary);
  }
  .user-btn.logged-in:hover,
  .user-btn.logged-in.is-open {
    border-color: var(--copper);
    color: var(--copper);
  }

  .user-avatar {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 50%;
    display: block;
  }

  .user-dot {
    position: absolute;
    top: -1px;
    right: -1px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-ember);
    box-shadow: 0 0 0 1.5px var(--color-canvas-base);
  }

  .user-initial {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 13px;
    font-weight: 600;
    line-height: 1;
  }

  /* ── Mobile breakpoint ───────────────────────────────────────
     The desktop layout is a wide 5-column "proscenium" (avatar + 2 switchers |
     nav | brand + subtitle | nav | Write + icons). Thanks to the minmax(auto,1fr)
     tracks above it no longer OVERLAPS when squeezed — but the content still
     genuinely runs out of room, so below this width we hand over to the
     compact bar (sections move to the bottom `SectionBar`; the hamburger
     remains only on a work page, which has no bottom bar). This is now a cosmetic threshold, not a correctness gate:
     if a longer brand name or locale pushes the fit-point around, the header
     degrades gracefully instead of breaking. 1024px clears the tightest locale
     (RU: МАСТЕРСКАЯ / НАПИСАТЬ), which stops fitting around ~1000px. */
  @media (max-width: 1024px) {
    .site-header {
      height: 58px;
    }

    .site-header.is-leaf {
      height: 0;
      padding: 0 10px;
      background: none;
      border: none;
      box-shadow: none;
    }

    .site-header.is-leaf.is-scrolled {
      height: 0;
      background: none;
    }

    /* On mobile, override the 5-col grid with a simple flex row */
    .header-inner {
      display: flex;
      align-items: center;
      padding: 0 16px;
    }

    .site-header.is-leaf .header-inner {
      position: relative;
      height: 50px;
      padding: 0 4px 0 6px;
    }

    :global(html:has(.site-header.is-leaf) .with-site-header) {
      padding-top: 50px;
    }

    /* Hide desktop-only zones */
    .side,
    .ghost-left,
    .ghost-right-group { display: none; }

    /* Brand: move to flex-start, horizontal layout */
    .brand-triptych {
      gap: 14px;
      min-width: 0;
    }

    .brand {
      flex-direction: row;
      align-items: center;
      gap: 9px;
      padding: 0;
      align-self: auto;
    }

    .site-header.is-leaf .brand {
      padding: 0;
      grid-column: auto;
      justify-self: auto;
    }

    /* Coat of arms, not a stretched grid: prev–raven–next as one lockup,
       optically centered in the pill. The menu sits on top as a bookmark. */
    .site-header.is-leaf:not(.is-scrolled) .brand-triptych {
      position: absolute;
      inset: 0;
      display: flex;
      flex: none;
      justify-content: center;
      align-items: center;
      gap: 14px;
      min-width: 0;
      pointer-events: none;
    }

    .site-header.is-leaf:not(.is-scrolled) .brand-triptych :global(a) {
      pointer-events: auto;
    }

    .site-header.is-leaf:not(.is-scrolled) .brand-name {
      display: none;
    }

    /* After scroll the plates leave — collapse back to a normal app bar. */
    .site-header.is-leaf.is-scrolled .brand-triptych {
      position: static;
      display: flex;
      flex: 1;
      justify-content: flex-start;
      gap: 0;
      pointer-events: auto;
    }

    .brand-inner { gap: 9px; }

    .brand-name { font-size: 1.45rem; }

    /* subtitle always hidden on mobile */
    .brand-sub { display: none; }

    /* Show mobile controls */
    .mobile-controls {
      display: flex;
      align-items: center;
      gap: 8px;
      margin-left: auto;
      flex: 0 0 auto;
      z-index: 2;
    }

    /* Arrival: the spread is the chrome. Write / account / keeper live in
       the page and the menu; they return once the plates leave. */
    .site-header.is-leaf:not(.is-scrolled) .mobile-contact-anchor,
    .site-header.is-leaf:not(.is-scrolled) .mobile-user-btn {
      display: none;
    }

    /* «Написать» на телефоне — как на компьютере: круглая, без рамки, цвет
       акцента. Прежняя квадратная рамка делала её самым громким местом полосы
       рядом с круглыми значками. */
    .mobile-contact-anchor {
      position: relative;
      display: flex;
      align-items: center;
    }

    .mobile-contact-btn {
      position: relative;
      display: flex;
      align-items: center;
      justify-content: center;
      width: 36px;
      height: 36px;
      background: transparent;
      border: 1px solid transparent;
      border-radius: 50%;
      padding: 0;
      cursor: pointer;
      color: #a8482a;
      transition: background 0.2s ease, color 0.2s ease;
    }
    .mobile-contact-btn:hover,
    .mobile-contact-btn.is-open {
      background: #a8482a;
      color: #fff9f0;
    }
    .mobile-contact-btn:focus-visible,
    .mobile-user-btn:focus-visible {
      outline: 2px solid #a8482a;
      outline-offset: 2px;
    }

    /* The shared .contact-panel is right-anchored to its button; on the mobile bar
       that button isn't the rightmost item, so pin the panel to the header edge
       instead and cap its width to the viewport. */
    .mobile-contact-panel {
      position: fixed;
      top: 58px;
      right: 12px;
      left: auto;
      width: min(320px, calc(100vw - 24px));
    }

    .site-header.is-leaf .mobile-nav {
      top: 50px;
    }

    .site-header.is-leaf .mobile-contact-panel {
      top: 50px;
    }

    .mobile-user-btn {
      position: relative;
      display: flex;
      align-items: center;
      justify-content: center;
      width: 36px;
      height: 36px;
      background: none;
      border: 1px solid transparent;
      border-radius: 50%;
      padding: 0;
      cursor: pointer;
      color: var(--color-ink-tertiary);
      transition: color 0.25s;
    }

    .mobile-user-btn:has(.user-avatar) {
      border-color: var(--border);
    }

    .mobile-menu-btn {
      display: flex;
      width: 44px;
      height: 44px;
      align-items: center;
      justify-content: center;
      flex-direction: column;
      gap: 4px;
      border: none;
      background: none;
      cursor: pointer;
      color: var(--color-ink-secondary);
      padding: 0;
    }

    .mobile-menu-btn span {
      width: 16px;
      height: 1px;
      background: currentColor;
      transform-origin: center;
      transition: transform 0.22s ease, opacity 0.22s ease;
    }

    .mobile-menu-btn.is-open span:nth-child(1) { transform: translateY(5px) rotate(45deg); }
    .mobile-menu-btn.is-open span:nth-child(2) { opacity: 0; }
    .mobile-menu-btn.is-open span:nth-child(3) { transform: translateY(-5px) rotate(-45deg); }

    .mobile-menu-btn:focus-visible {
      outline: 2px solid rgba(198, 95, 60, 0.52);
      outline-offset: 3px;
    }

    /* Mobile nav dropdown */
    .mobile-nav {
      display: grid;
      position: fixed;
      top: 58px;
      left: 0;
      right: 0;
      padding: 6px 16px 16px;
      background: rgba(248, 241, 231, 0.99);
      border-bottom: 1px solid var(--border);
      box-shadow: 0 18px 34px rgba(52, 37, 28, 0.10);
      transform: translateY(-6px);
      opacity: 0;
      visibility: hidden;
      pointer-events: none;
      transition: opacity 0.22s ease, transform 0.22s ease, visibility 0.22s ease;
    }

    .mobile-nav.is-open {
      transform: translateY(0);
      opacity: 1;
      visibility: visible;
      pointer-events: auto;
    }

    .mobile-nav-link {
      display: flex;
      align-items: center;
      height: 44px;
      font-family: var(--font-body);
      font-size: 12px;
      font-weight: 500;
      letter-spacing: 0.10em;
      text-transform: uppercase;
      color: var(--color-ink-secondary);
      text-decoration: none;
      border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 8%, transparent);
      transition: color 0.2s;
    }

    .mobile-nav-link.is-active,
    .mobile-nav-link:hover { color: var(--ink); }

    .mobile-nav-house {
      margin-top: 8px;
      padding-top: 8px;
      border-top: 1px solid color-mix(in srgb, var(--color-ink-primary) 10%, transparent);
    }

    .mobile-nav-house .mobile-nav-link {
      width: 100%;
      border-left: none;
      border-right: none;
      border-top: none;
      background: none;
      padding: 0;
      text-align: left;
      cursor: pointer;
    }

    .mobile-nav-link--write {
      color: var(--copper, #c65f3c);
    }

    .mobile-nav-link--write:hover {
      color: var(--mid, #6f3b24);
    }
  }

  /* ── Две строки на компьютере ──────────────────────────────────
     Сверху имя дома, поиск и действия; снизу ряд печатей — все семь
     разделов на виду, поэтому «Ещё» на компьютере нет. Прокрутка сводит
     шапку в одну строку: имя, печати без чисел, лупа, действия. Одна сетка
     с именованными областями, а не две полосы: при сворачивании печати
     встают в верхнюю строку на место поля поиска, ничего не пересобирая. */
  .search-slot,
  .seal-row {
    display: none;
  }

  @keyframes header-settle-a {
    from { opacity: 0.35; }
    to   { opacity: 1; }
  }
  @keyframes header-settle-b {
    from { opacity: 0.35; }
    to   { opacity: 1; }
  }

  /* Смена вида — короткое проявление содержимого вместо плавной высоты. Две
     одинаковые анимации с разными именами: смена имени запускает её заново. */
  @media (min-width: 1025px) and (prefers-reduced-motion: no-preference) {
    .site-header.is-settling:not(.is-leaf) .header-inner {
      animation: header-settle-a 0.16s ease-out;
    }
    .site-header.is-settling.is-scrolled:not(.is-leaf) .header-inner {
      animation-name: header-settle-b;
    }
  }

  /* Названия не поместились в колонку (`fitRow`): остаются знаки, названия — для
     чтеца экрана и подсказкой по наведению. */
  .seal-row:global([data-fit='icons']) .seal-text {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  .seal-row:global([data-fit='icons']) .seal-gap {
    display: none;
  }

  @media (min-width: 1025px) {
    .site-header:not(.is-leaf) {
      height: 150px;
    }
    .site-header:not(.is-leaf).is-scrolled {
      height: 58px;
    }
    .site-header:not(.is-leaf) .header-inner {
      grid-template-columns: auto auto minmax(0, 1fr) auto auto;
      grid-template-rows: 78px 72px;
      grid-template-areas:
        'lead brand search search acts'
        'seals seals seals seals seals';
      column-gap: 20px;
    }
    .site-header:not(.is-leaf).is-scrolled .header-inner {
      grid-template-rows: 58px;
      grid-template-areas: 'lead brand seals search acts';
    }
    .site-header:not(.is-leaf) .side-left {
      grid-area: lead;
    }
    .site-header:not(.is-leaf) .brand-triptych {
      grid-area: brand;
      justify-content: flex-start;
    }
    .site-header:not(.is-leaf) .brand {
      align-items: flex-start;
      padding: 0;
    }
    .site-header:not(.is-leaf) .side-right {
      grid-area: acts;
      justify-content: flex-end;
    }

    .search-slot {
      grid-area: search;
      display: flex;
      align-items: center;
      justify-content: center;
      min-width: 0;
    }
    .search-field {
      width: 100%;
      max-width: 620px;
    }
    .search-loupe {
      display: none;
    }
    .site-header.is-scrolled .search-field {
      display: none;
    }
    .site-header.is-scrolled .search-loupe {
      display: block;
    }

    .seal-row {
      grid-area: seals;
      container: seals / inline-size;
      display: flex;
      align-items: center;
      /* Четыре главные печати по центру ряда; «Студия» прижата к правому краю
         и в центровку не входит. */
      justify-content: safe center;
      position: relative;
      gap: clamp(18px, 2.6vw, 48px);
      min-width: 0;
      border-top: 1px solid color-mix(in srgb, var(--color-ink-primary) 12%, transparent);
    }
    .site-header:not(.is-scrolled) .seal-link--minor {
      position: absolute;
      right: 0;
      top: 50%;
      transform: translateY(-50%);
    }
    .seal-gap {
      display: none;
    }
    .site-header.is-scrolled .seal-row {
      --seal-size: 30px;
      /* `safe`: если ряд всё же шире колонки, он прижимается к левому краю, а не
         вылезает в обе стороны и не ложится на логотип и поиск. */
      justify-content: safe center;
      gap: clamp(10px, 1.4vw, 22px);
      border-top: 0;
    }
    /* В свёрнутой шапке распорки нет: вместе с двумя промежутками она
       давала между «Игрой» и «Ещё» 44 px вместо 20 у остальных. */
    .site-header.is-scrolled .seal-gap {
      display: none;
    }
  }

  /* Пункт ряда не сжимается: сжатый пункт обрезал своё название и наезжал
     на соседа. Нехватку места решает ряд целиком (контейнерные запросы ниже). */
  .seal-link {
    position: relative;
    isolation: isolate;
    display: flex;
    flex: none;
    align-items: center;
    gap: 12px;
    color: var(--ink);
    text-decoration: none;
  }
  /* Текущий раздел: красная линия под пунктом и тёплый свет, поднимающийся от
     неё. Линия растёт от середины к краям, свет проявляется следом. Оправы и
     сургучной капли у печати нет: отметка одна. Цвет не единственный признак:
     название курсивом, у «Игры» и остальных. Движения при смене раздела одно,
     ни пульсации, ни мерцания. */
  .seal-link::before,
  .seal-link::after {
    content: '';
    position: absolute;
    pointer-events: none;
  }
  .seal-link::before {
    z-index: -1;
    left: -14px;
    right: -14px;
    bottom: -6px;
    height: 78%;
    background: radial-gradient(
      ellipse 62% 100% at 50% 100%,
      color-mix(in srgb, var(--color-ember) 30%, transparent),
      transparent 72%
    );
    opacity: 0;
    transition: opacity 0.5s ease 0.1s;
  }
  .seal-link::after {
    left: -2px;
    right: -2px;
    bottom: -6px;
    height: 4px;
    border-radius: 2px;
    background: var(--color-ember);
    transform: scaleX(0);
    opacity: 0.55;
    transition: transform 0.4s cubic-bezier(0.2, 0.8, 0.2, 1), opacity 0.2s ease;
  }
  .seal-link:hover::after,
  .seal-link:focus-visible::after {
    transform: scaleX(0.3);
  }
  .seal-link.is-active::before {
    opacity: 1;
  }
  .seal-link.is-active::after {
    transform: scaleX(1);
    opacity: 1;
  }
  .seal-link.is-active :global(.section-icon) {
    color: var(--color-ember-deep);
  }
  .site-header.is-scrolled .seal-link::before,
  .site-header.is-scrolled .seal-link::after {
    bottom: -4px;
  }
  .site-header.is-scrolled .seal-link::after {
    height: 3px;
  }
  @media (prefers-reduced-motion: reduce) {
    .seal-link::before,
    .seal-link::after {
      transition: none;
    }
  }
  .seal-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .seal-name {
    font-family: var(--font-title);
    font-size: 18px;
    line-height: 1.1;
    white-space: nowrap;
    text-decoration: underline 1px transparent;
    text-underline-offset: 5px;
    transition: text-decoration-color 0.2s ease, color 0.2s ease;
  }
  .seal-link:hover .seal-name {
    text-decoration-color: color-mix(in srgb, var(--color-ink-primary) 35%, transparent);
  }
  /* Подпись второй строки: 11 px и вес 500 вместо 10 px и 400 — при тех же
     6.2:1 тонкий мелкий капслок с широкой разрядкой читался бледнее своего цвета.
     Цвет — ink-secondary (10:1). Высота держится и у пустой строки. */
  .seal-num {
    font-family: var(--font-body);
    font-size: 11px;
    font-weight: 500;
    line-height: 1.2;
    min-height: 1.2em;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--color-ink-secondary);
    white-space: nowrap;
  }
  .seal-num--empty {
    visibility: hidden;
  }
  .seal-num em {
    margin-left: 8px;
    font-style: normal;
    font-weight: 600;
    color: var(--color-ember-deep);
  }
  .seal-link.is-active .seal-name {
    font-style: italic;
    color: var(--color-ember-deep);
  }
  .seal-link--minor {
    gap: 9px;
  }
  .seal-link--minor .seal-name {
    font-family: var(--font-body);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--color-ink-secondary);
  }
  .seal-link--minor.is-active .seal-name {
    font-style: normal;
    color: var(--color-ember-deep);
  }
  .seal-link:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--color-ember) 60%, transparent);
    outline-offset: 4px;
    border-radius: 4px;
  }
  .site-header.is-scrolled .seal-num {
    display: none;
  }
  .site-header.is-scrolled .seal-link {
    gap: 8px;
  }
  .site-header.is-scrolled .seal-name {
    font-size: 15px;
  }

  /* Подпись малой печати уходит, когда места нет, но остаётся для чтеца
     экрана и подсказкой по наведению: сама печать с подписью не спорит. */
  .site-header.is-scrolled .seal-link--minor .seal-text {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  /* Вторая строка «Игры» («Этюды и карты») шире прежнего «Игра»: у самого узкого
     окна компьютера ряд выходил за колонку на 4 px, потому что промежутки между
     печатями ещё росли от ширины окна. */
  @media (min-width: 1025px) and (max-width: 1120px) {
    .site-header:not(.is-scrolled) .seal-row {
      gap: 16px;
    }
  }
  @media (min-width: 1025px) and (max-width: 1359px) {
    .seal-link--minor .seal-text {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip-path: inset(50%);
      white-space: nowrap;
    }
  }
  @media (min-width: 1025px) and (max-width: 1279px) {
    .site-header:not(.is-leaf).is-scrolled .brand-name,
    .site-header:not(.is-leaf).is-scrolled .brand-words {
      display: none;
    }
  }

  /* ── Левая часть шапки: портрет, черта, имя, медаль ─────────────────
     Фото автора — без рамки и контура, во всю высоту шапки и у самого
     левого края окна; за ним волосяная черта, имя, и последним — ворон в
     кружке. Прежде фото стояло в арке, рядом с вороном в двойной оправе, и
     две рамки разной формы спорили друг с другом. Фото есть только в
     свёрнутой шапке; в развёрнутой слева имя и ворон. На странице работы и
     на телефоне всё как было. */
  .medal {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
  }
  .brand-words {
    display: contents;
  }

  @media (min-width: 1025px) {
    /* Развёрнутая шапка — без фото: слева имя и ворон. */
    .site-header:not(.is-leaf):not(.is-scrolled) .header-inner {
      grid-template-columns: auto minmax(0, 1fr) auto;
      grid-template-areas:
        'brand search acts'
        'seals seals seals';
    }
    .site-header:not(.is-leaf):not(.is-scrolled) .side-left {
      display: none;
    }

    /* Свёрнутая: ряд на всю ширину окна, чтобы фото встало у края. Правый
       отступ считается так, чтобы действия остались там же, где в
       развёрнутой шапке (колонка до 1380 px по центру). */
    .site-header:not(.is-leaf).is-scrolled .header-inner {
      max-width: none;
      margin: 0;
      padding-left: 0;
      padding-right: calc(max(0px, (100% - 1380px) / 2) + clamp(20px, 4vw, 52px));
    }
    .site-header:not(.is-leaf).is-scrolled .header-avatar {
      width: 52px;
      height: 58px;
      padding: 0;
      border-radius: 0;
      background: none;
    }
    .site-header:not(.is-leaf).is-scrolled .header-avatar.is-hidden {
      display: none;
    }
    .site-header:not(.is-leaf).is-scrolled .header-avatar-img {
      border-radius: 0;
      box-shadow: none;
      object-position: 50% 26%;
      transform: none;
      transition: opacity 0.2s ease;
    }
    .site-header:not(.is-leaf).is-scrolled .header-avatar:hover .header-avatar-img,
    .site-header:not(.is-leaf).is-scrolled .header-avatar:focus-visible .header-avatar-img {
      box-shadow: none;
      transform: none;
      opacity: 0.88;
    }
    /* Готовая картинка вместо фото автора: высота шапки, ширина по форме
       кадра; цвета не трогаются. */
    .site-header:not(.is-leaf).is-scrolled .header-avatar:has(.header-portrait-img) {
      width: var(--portrait-w, 52px);
      margin-left: var(--portrait-gap-l, 0px);
      margin-right: var(--portrait-gap-r, 0px);
    }
    .site-header:not(.is-leaf).is-scrolled .header-avatar-img.has-portrait {
      display: none;
    }
    .site-header:not(.is-leaf).is-scrolled .header-portrait-img {
      display: block;
      width: 100%;
      height: 100%;
      object-fit: cover;
      transition: opacity 0.2s ease;
    }
    .site-header:not(.is-leaf).is-scrolled .header-avatar:hover .header-portrait-img,
    .site-header:not(.is-leaf).is-scrolled .header-avatar:focus-visible .header-portrait-img {
      opacity: 0.88;
    }
    /* Черта между фото и именем. Без фото (на главной оно прячется, пока
       виден первый экран) черты нет: она бы стояла у края без пары. */
    .site-header:not(.is-leaf).is-scrolled .brand-triptych {
      align-self: center;
      height: 28px;
      padding-left: 20px;
      border-left: 1px solid color-mix(in srgb, var(--color-ink-primary) 22%, transparent);
    }
    .site-header:not(.is-leaf).is-scrolled .side-left:has(.header-avatar.is-hidden) + .brand-triptych {
      padding-left: 0;
      border-left: 0;
    }

    .site-header:not(.is-leaf) .brand-inner {
      gap: 16px;
    }
    .site-header:not(.is-leaf) .brand-words {
      display: flex;
      flex-direction: column;
      align-items: flex-start;
      gap: 6px;
    }
    /* Ворон — после имени. */
    .site-header:not(.is-leaf) .medal {
      order: 2;
    }
    .site-header:not(.is-leaf) .medal :global(.raven-circle) {
      --raven-h: 42px;
      transition: height 0.3s var(--ease), transform 0.4s var(--ease);
    }
    .site-header:not(.is-leaf) .brand-name {
      font-size: clamp(2.1rem, 2.9vw, 2.65rem);
    }
    .site-header:not(.is-leaf).is-scrolled .medal :global(.raven-circle) {
      --raven-h: 28px;
    }
    .site-header:not(.is-leaf).is-scrolled .brand-name {
      font-size: 1.6rem;
    }
    .site-header:not(.is-leaf).is-scrolled .brand-words {
      gap: 0;
    }
  }

  @media (min-width: 1025px) {

    /* Черта между разделами и действиями: слева «куда идти», справа «что сделать». */
    .site-header.is-scrolled .search-slot {
      align-self: center;
      height: 28px;
      padding-left: 18px;
      border-left: 1px solid color-mix(in srgb, var(--color-ink-primary) 18%, transparent);
    }

    /* Язык в свёрнутой шапке — буквами, без кружка: круг рядом с лупой и
       профилем делал из действий ещё один ряд печатей. Кружок проступает
       при наведении и пока открыт листок языков. */
    .site-header.is-scrolled .ghost-right-group :global(.ls .seal) {
      background: none;
      border-color: transparent;
      box-shadow: none;
    }
    .site-header.is-scrolled .ghost-right-group :global(.ls .mark) {
      letter-spacing: 0.14em;
      color: var(--color-ink-secondary);
    }
    .site-header.is-scrolled .ghost-right-group :global(.ls .seal:hover),
    .site-header.is-scrolled .ghost-right-group :global(.ls--open .seal),
    .site-header.is-scrolled .ghost-right-group :global(.ls .seal:focus-visible) {
      border-color: color-mix(in srgb, var(--color-ember) 38%, transparent);
    }
  }

  /* Профиль без фотографии — светлая буква на тёмном круге, а не буква в
     рамке: в рамке он был близнецом печати языка. */
  .site-header:not(.is-leaf) .user-btn.has-initial {
    width: 30px;
    height: 30px;
    border-color: transparent;
    background: var(--color-ember-ink, #6f3b24);
    color: var(--color-canvas-base, #f8f1e7);
    box-shadow:
      0 0 0 1px var(--color-canvas-base, #f8f1e7),
      0 0 0 2px color-mix(in srgb, var(--color-ink-primary) 28%, transparent);
  }
  .site-header:not(.is-leaf) .user-btn.has-initial:hover,
  .site-header:not(.is-leaf) .user-btn.has-initial.is-open {
    border-color: transparent;
    background: var(--color-ember-deep, #9a4120);
    color: var(--color-canvas-base, #f8f1e7);
  }
  .site-header:not(.is-leaf) .user-btn.has-initial .user-initial {
    font-style: italic;
    font-weight: 500;
    font-size: 15px;
  }
  .mobile-user-btn.has-initial {
    width: 32px;
    height: 32px;
    background: var(--color-ember-ink, #6f3b24);
    color: var(--color-canvas-base, #f8f1e7);
  }
  .mobile-user-btn.has-initial .user-initial {
    font-style: italic;
    font-weight: 500;
    font-size: 15px;
  }

  /* Лупа в полосе телефона. */
  .mobile-search {
    display: grid;
  }

  /* Область нажатия 44 px без изменения вида: на телефоне и на экранах с
     пальцем (планшет с шапкой компьютера) кнопки шапки были 30–36 px. */
  .contact-btn,
  .mobile-contact-btn {
    position: relative;
  }
  @media (max-width: 1024px), (pointer: coarse) {
    .contact-btn::after,
  .user-btn::after,
  .mobile-contact-btn::after,
  .mobile-user-btn::after {
      content: '';
      position: absolute;
      left: 50%;
      top: 50%;
      width: max(100%, 44px);
      height: max(100%, 44px);
      transform: translate(-50%, -50%);
    }
  }

</style>
