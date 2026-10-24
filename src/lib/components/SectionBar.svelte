<script lang="ts">
  // Нижняя панель разделов — только на телефоне и планшете (до 1024 px, как и
  // переключение шапки). Заменяет гамбургер: разделы всегда на виду и под большим
  // пальцем. На компьютере меню остаётся в шапке, и панель скрыта стилем.
  // Разделы стоят печатями, как в шапке на компьютере: оттиск в бумаге,
  // текущий — сургуч (`SectionSeal`).
  //
  // Прячется вниз, когда страницу листают вперёд, и возвращается при малейшем
  // движении назад, у верха, у низа страницы и когда в неё попал фокус
  // клавиатуры: закреплённая панель не должна отнимать фото у того, кто листает.
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { fade, fly } from 'svelte/transition';
  import { t } from '$lib/i18n';
  import { interact } from '$lib/analytics';
  import { GAME_HREF, isNavActive } from '$lib/nav';
  import LangSwitcher from '$lib/components/LangSwitcher.svelte';
  import NavCount from '$lib/components/NavCount.svelte';
  import SectionSeal from '$lib/components/SectionSeal.svelte';

  let pathname = $derived(page.url.pathname);

  const MORE = [
    { href: '/studio', key: 'navStudio' },
  ] as const;

  let moreOpen = $state(false);
  let away = $state(false);
  let reduceMotion = $state(false);

  // Пока открыт лист «Ещё», горит только он: два горящих пункта сразу не отвечают, где человек.
  let here = $derived(moreOpen ? '' : pathname);

  // «Ещё» горит, когда человек находится в одном из его разделов.
  let moreActive = $derived(MORE.some((m) => isNavActive(pathname, m.href)));

  // Прокрутка вперёд дальше этого порога прячет панель; ближе к верху она нужна всегда.
  const HIDE_AFTER = 160;
  // Шаг меньше этого — дрожь пальца, а не намерение.
  const STEP = 6;
  let lastY = 0;

  // Высота страницы читается при изменении размера, а не на каждом событии
  // прокрутки: `scrollHeight` — принудительный расчёт раскладки.
  let pageHeight = 0;
  let sizeWatch: ResizeObserver | null = null;
  function measure() {
    pageHeight = document.documentElement.scrollHeight;
  }

  let frame = 0;
  function onScroll() {
    if (frame) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      step();
    });
  }

  function step() {
    const y = window.scrollY;
    const delta = y - lastY;
    const atBottom = window.innerHeight + y >= pageHeight - 48;
    if (y < 80 || atBottom) {
      away = false;
      lastY = y;
      return;
    }
    if (Math.abs(delta) < STEP) return;
    away = delta > 0 && y > HIDE_AFTER && !moreOpen;
    lastY = y;
  }

  // Нажатия на панель считаются тем же `header_click`, что и в шапке компьютера:
  // иначе решение о весе пунктов принималось бы по половине посетителей.
  const DOCK_NAME: Record<string, string> = {
    '/figurines': 'works', '/gazette': 'gazette', '/tales': 'tales', [GAME_HREF]: 'game',
  };
  function countClicks(node: HTMLElement) {
    node.addEventListener('click', countClick);
    return { destroy: () => node.removeEventListener('click', countClick) };
  }
  function countClick(e: MouseEvent) {
    const hit = (e.target as Element | null)?.closest('a, button');
    if (!hit) return;
    const name = hit.tagName === 'A' ? DOCK_NAME[hit.getAttribute('href') ?? ''] : 'more';
    if (name) interact('header_click', `dock:${name}`, null, false);
  }

  function closeMore() {
    moreOpen = false;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && moreOpen) closeMore();
  }

  // Новая страница — панель на виду и закрытый лист «Ещё».
  $effect(() => {
    void pathname;
    moreOpen = false;
    away = false;
    lastY = typeof window === 'undefined' ? 0 : window.scrollY;
  });

  onMount(() => {
    reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    lastY = window.scrollY;
    measure();
    window.addEventListener('scroll', onScroll, { passive: true });
    window.addEventListener('resize', measure, { passive: true });
    document.addEventListener('keydown', onKey);
    sizeWatch = new ResizeObserver(measure);
    sizeWatch.observe(document.body);
    return () => {
      window.removeEventListener('scroll', onScroll);
      window.removeEventListener('resize', measure);
      document.removeEventListener('keydown', onKey);
      sizeWatch?.disconnect();
      cancelAnimationFrame(frame);
    };
  });
</script>

{#if moreOpen}
  <button
    type="button"
    class="scrim"
    aria-label={$t('dockClose')}
    onclick={closeMore}
    transition:fade={{ duration: reduceMotion ? 0 : 150 }}
  ></button>
  <div
    id="section-more"
    class="sheet"
    role="dialog"
    aria-label={$t('dockMore')}
    transition:fly={{ y: 24, duration: reduceMotion ? 0 : 200 }}
  >
    <ul class="sheet-list">
      {#each MORE as item (item.href)}
        <li>
          <a
            href={item.href}
            class="sheet-link"
            class:is-active={isNavActive(pathname, item.href)}
            aria-current={isNavActive(pathname, item.href) ? 'page' : undefined}
            onclick={closeMore}>{$t(item.key)}</a
          >
        </li>
      {/each}
    </ul>
    <div class="sheet-lang">
      <LangSwitcher variant="light" layout="strip" />
    </div>
  </div>
{/if}

<nav
  class="bar"
  class:is-away={away && !moreOpen}
  aria-label={$t('dockSections')}
  onfocusin={() => (away = false)}
  use:countClicks
>
  <a
    href="/figurines"
    class="tab seal-host"
    class:is-active={isNavActive(here, '/figurines')}
    aria-current={isNavActive(here, '/figurines') ? 'page' : undefined}
  >
    <SectionSeal icon="works" size={30} />
    <span>{$t('dockWorks')}</span>
    <span class="tab-new"><NavCount section="works" active={isNavActive(here, '/figurines')} compact /></span>
  </a>
  <a
    href="/gazette"
    class="tab seal-host"
    class:is-active={isNavActive(here, '/gazette')}
    aria-current={isNavActive(here, '/gazette') ? 'page' : undefined}
  >
    <SectionSeal icon="gazette" size={30} />
    <span>{$t('navGazette')}</span>
    <span class="tab-new"><NavCount section="gazette" active={isNavActive(here, '/gazette')} compact /></span>
  </a>
  <a
    href="/tales"
    class="tab seal-host"
    class:is-active={isNavActive(here, '/tales')}
    aria-current={isNavActive(here, '/tales') ? 'page' : undefined}
  >
    <SectionSeal icon="tales" size={30} />
    <span>{$t('navTales')}</span>
    <span class="tab-new"><NavCount section="tales" active={isNavActive(here, '/tales')} compact /></span>
  </a>
  <a
    href={GAME_HREF}
    class="tab seal-host"
    class:is-active={isNavActive(here, GAME_HREF)}
    aria-current={isNavActive(here, GAME_HREF) ? 'page' : undefined}
  >
    <SectionSeal icon="game" size={30} />
    <span>{$t('navGame')}</span>
  </a>
  <button
    type="button"
    class="tab seal-host"
    class:is-active={moreOpen || moreActive}
    aria-expanded={moreOpen}
    aria-controls="section-more"
    onclick={() => (moreOpen = !moreOpen)}
  >
    <SectionSeal icon="more" size={30} />
    <span>{$t('dockMore')}</span>
  </button>
</nav>

<style>
  /* Только телефон и планшет; выше 1024 px разделы в шапке. */
  .bar,
  .scrim,
  .sheet {
    display: none;
  }

  @media (max-width: 1024px) {
    .bar {
      position: fixed;
      left: 0;
      right: 0;
      bottom: 0;
      z-index: 190;
      display: flex;
      height: calc(64px + env(safe-area-inset-bottom, 0px));
      padding-bottom: env(safe-area-inset-bottom, 0px);
      background: var(--color-canvas-base, #f8f1e7);
      border-top: 1px solid var(--border, #d8c6b1);
      box-shadow: 0 -6px 20px color-mix(in srgb, var(--color-ink-primary, #34251c) 6%, transparent);
      transition: transform 0.22s ease;
    }

    .bar.is-away {
      transform: translateY(100%);
    }

    .tab {
      flex: 1 1 0;
      min-width: 0;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 1px;
      padding: 0 2px;
      border: 0;
      background: none;
      font: inherit;
      font-family: var(--font-title, Georgia, serif);
      font-size: 12px;
      line-height: 1.1;
      color: var(--color-ink-secondary, #5b4638);
      text-decoration: none;
      cursor: pointer;
      -webkit-tap-highlight-color: transparent;
    }

    .tab {
      position: relative;
    }

    /* «Новое» с прошлого визита — маленькой отметкой у правого верхнего угла
       значка; общее число в тесной панели не печатается. */
    .tab-new {
      position: absolute;
      top: 3px;
      left: calc(50% + 12px);
      line-height: 1;
      pointer-events: none;
    }

    .tab-new :global(.nav-new) {
      margin-left: 0;
    }

    .tab span {
      max-width: 100%;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    /* Активный раздел назван не только цветом: печать становится сургучной
       каплей (другая форма и объём), подпись — курсивом. Один акцентный цвет
       не видит тот, кто не различает красный. */
    .tab.is-active {
      color: var(--color-ember-deep, #6f3b24);
      font-style: italic;
    }

    /* Красная линия над текущим пунктом и тёплый свет вниз от неё. */
    .tab {
      isolation: isolate;
    }
    .tab::before,
    .tab::after {
      content: '';
      position: absolute;
      pointer-events: none;
    }
    .tab::before {
      z-index: -1;
      left: 0;
      right: 0;
      top: 0;
      height: 70%;
      background: radial-gradient(
        ellipse 60% 100% at 50% 0%,
        color-mix(in srgb, var(--color-ember, #c65f3c) 28%, transparent),
        transparent 72%
      );
      opacity: 0;
      transition: opacity 0.5s ease 0.1s;
    }
    .tab::after {
      left: 22%;
      right: 22%;
      top: 0;
      height: 4px;
      border-radius: 0 0 2px 2px;
      background: var(--color-ember, #c65f3c);
      transform: scaleX(0);
      transition: transform 0.4s cubic-bezier(0.2, 0.8, 0.2, 1);
    }
    .tab.is-active::before {
      opacity: 1;
    }
    .tab.is-active::after {
      transform: scaleX(1);
    }
    .tab.is-active :global(.section-icon) {
      color: var(--color-ember-deep, #6f3b24);
    }
    @media (prefers-reduced-motion: reduce) {
      .tab::before,
      .tab::after {
        transition: none;
      }
    }

    .tab:focus-visible,
    .sheet-link:focus-visible {
      outline: 2px solid rgba(198, 95, 60, 0.6);
      outline-offset: -3px;
    }

    .scrim {
      position: fixed;
      inset: 0;
      z-index: 188;
      display: block;
      border: 0;
      padding: 0;
      background: rgba(52, 37, 28, 0.28);
      cursor: default;
    }

    .sheet {
      position: fixed;
      left: 8px;
      right: 8px;
      bottom: calc(64px + env(safe-area-inset-bottom, 0px) + 8px);
      z-index: 189;
      display: block;
      padding: 6px 16px 14px;
      background: var(--color-canvas-base, #f8f1e7);
      border: 1px solid var(--border, #d8c6b1);
      border-radius: 14px;
      box-shadow: 0 14px 34px rgba(52, 37, 28, 0.16);
    }

    .sheet-list {
      margin: 0;
      padding: 0;
      list-style: none;
    }

    .sheet-link {
      display: flex;
      align-items: center;
      height: 48px;
      font-family: var(--font-body, system-ui, sans-serif);
      font-size: 13px;
      font-weight: 500;
      letter-spacing: 0.1em;
      text-transform: uppercase;
      color: var(--color-ink-secondary, #5b4638);
      text-decoration: none;
      border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 8%, transparent);
    }

    .sheet-link.is-active {
      color: var(--color-ember-deep, #6f3b24);
    }

    .sheet-lang {
      padding-top: 12px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .bar {
      transition: none;
    }
  }
</style>
