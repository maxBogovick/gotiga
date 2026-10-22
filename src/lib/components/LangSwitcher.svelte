<script lang="ts">
  // Переключатель языка.
  //
  // Два вида одного и того же:
  //   · `seal` — круглая печать в шапке с двумя буквами языка; нажатие
  //     раскрывает листок со строкой на каждый язык (их шесть). Печать мала, потому что в шапке
  //     тесно, а язык выбирают один раз.
  //   · `strip` — клетки в ряд с ползунком под выбранной; для меню на
  //     телефоне, где листку негде раскрыться. Под буквами — метка языка
  //     (ES, DE…), а не название: шесть названий в ширину телефона не
  //     помещаются («Українська» шире клетки). Название — в `aria-label`.
  //
  // Смена языка идёт не скачком, а расходящимся кругом из того места, на
  // которое нажали: новый язык «проступает» поверх старого страницы
  // (`html.gt-lang` в app.css, переход видов браузера). Там, где перехода
  // нет, и при `prefers-reduced-motion` язык меняется сразу, без всего этого.
  import { tick } from 'svelte';
  import { lang, loadLang, t, LANGS, LANG_META, type Lang } from '$lib/i18n';

  let {
    variant = 'light',
    layout = 'seal',
  }: {
    variant?: 'dark' | 'light';
    layout?: 'seal' | 'strip';
  } = $props();

  /** Буквы, по которым язык узнают с первого взгляда: у украинского — «Ї»,
   *  у испанского — «Ñ», у немецкого — «Ä», у итальянского — «À». */
  const SPECIMEN: Record<Lang, string> = { en: 'Aa', ru: 'Яя', uk: 'Її', es: 'Ññ', de: 'Ää', it: 'Àà' };

  let open = $state(false);
  let focusIdx = $state(0);
  let root = $state<HTMLElement | null>(null);
  let trigger = $state<HTMLButtonElement | null>(null);
  let rows = $state<(HTMLButtonElement | null)[]>([]);

  let at = $derived(Math.max(0, LANGS.indexOf($lang)));

  type VTDocument = Document & {
    startViewTransition(cb: () => void | Promise<void>): { finished: Promise<void> };
  };

  async function choose(next: Lang, origin: HTMLElement | null) {
    open = false;
    if (next === $lang) {
      trigger?.focus();
      return;
    }
    const reduce =
      typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
    const canMorph = typeof document !== 'undefined' && 'startViewTransition' in document;
    // Словарь приезжает до снимка, а не внутри него: иначе переход ждал бы
    // сеть, а страница стояла бы замершей.
    await loadLang(next);
    if (reduce || !canMorph || !origin) {
      lang.set(next);
      trigger?.focus();
      return;
    }
    const box = origin.getBoundingClientRect();
    const x = box.left + box.width / 2;
    const y = box.top + box.height / 2;
    const reach = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y));
    const html = document.documentElement;
    html.style.setProperty('--gt-lang-x', `${x}px`);
    html.style.setProperty('--gt-lang-y', `${y}px`);
    html.style.setProperty('--gt-lang-r', `${Math.ceil(reach)}px`);
    html.classList.add('gt-lang');
    const run = (document as VTDocument).startViewTransition(async () => {
      lang.set(next);
      await tick();
    });
    run.finished.finally(() => {
      html.classList.remove('gt-lang');
    });
    trigger?.focus({ preventScroll: true });
  }

  async function show() {
    open = true;
    focusIdx = at;
    await tick();
    rows[focusIdx]?.focus();
  }

  function toggle() {
    if (open) {
      open = false;
    } else {
      void show();
    }
  }

  function triggerKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      void show();
    }
  }

  async function move(delta: number) {
    focusIdx = (focusIdx + delta + LANGS.length) % LANGS.length;
    await tick();
    rows[focusIdx]?.focus();
  }

  function listKey(e: KeyboardEvent) {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        void move(1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        void move(-1);
        break;
      case 'Home':
        e.preventDefault();
        focusIdx = 0;
        rows[0]?.focus();
        break;
      case 'End':
        e.preventDefault();
        focusIdx = LANGS.length - 1;
        rows[LANGS.length - 1]?.focus();
        break;
      case 'Escape':
        e.preventDefault();
        open = false;
        trigger?.focus();
        break;
      case 'Tab':
        open = false;
        break;
    }
  }

  function outside(e: PointerEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }

  function stripKey(e: KeyboardEvent, i: number) {
    if (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft') return;
    e.preventDefault();
    const to = (i + (e.key === 'ArrowRight' ? 1 : -1) + LANGS.length) % LANGS.length;
    (root?.querySelectorAll<HTMLElement>('.chip')[to])?.focus();
  }
</script>

<svelte:window onpointerdown={outside} />

{#if layout === 'strip'}
  <div
    class="ls ls--strip"
    class:ls--dark={variant === 'dark'}
    bind:this={root}
    role="radiogroup"
    aria-label={$t('navLanguage')}
    style="--n: {LANGS.length}; --at: {at}"
  >
    <span class="thumb" aria-hidden="true"></span>
    {#each LANGS as l, i (l)}
      <button
        type="button"
        class="chip"
        role="radio"
        aria-checked={$lang === l}
        tabindex={$lang === l ? 0 : -1}
        lang={l}
        aria-label={LANG_META[l].native}
        title={LANG_META[l].native}
        onclick={(e) => choose(l, e.currentTarget)}
        onkeydown={(e) => stripKey(e, i)}
      >
        <span class="chip-spec" aria-hidden="true">{SPECIMEN[l]}</span>
        <span class="chip-name" aria-hidden="true">{LANG_META[l].mark}</span>
      </button>
    {/each}
  </div>
{:else}
  <div class="ls" class:ls--dark={variant === 'dark'} class:ls--open={open} bind:this={root}>
    <button
      type="button"
      class="seal"
      bind:this={trigger}
      aria-haspopup="menu"
      aria-expanded={open}
      aria-label={`${$t('navLanguage')}: ${LANG_META[$lang].native}`}
      title={`${$t('navLanguage')}: ${LANG_META[$lang].native}`}
      onclick={toggle}
      onkeydown={triggerKey}
    >
      {#key $lang}
        <span class="ripple" aria-hidden="true"></span>
      {/key}
      <span class="mark" aria-hidden="true">
        {#key $lang}<span class="mark-in">{LANG_META[$lang].mark}</span>{/key}
      </span>
    </button>

    {#if open}
      <div class="sheet" role="menu" aria-label={$t('navLanguage')} tabindex="-1" onkeydown={listKey}>
        {#each LANGS as l, i (l)}
          <button
            type="button"
            class="row"
            class:row--on={$lang === l}
            role="menuitemradio"
            aria-checked={$lang === l}
            tabindex={i === focusIdx ? 0 : -1}
            lang={l}
            style="--i: {i}"
            bind:this={rows[i]}
            onmouseenter={() => (focusIdx = i)}
            onclick={(e) => choose(l, trigger ?? e.currentTarget)}
          >
            <span class="spec" aria-hidden="true">{SPECIMEN[l]}</span>
            <span class="name">{LANG_META[l].native}</span>
            <span class="code" aria-hidden="true">{LANG_META[l].mark}</span>
            <span class="tick" aria-hidden="true">✦</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .ls {
    --ls-ink: var(--color-ink-primary, #2c1710);
    --ls-soft: var(--color-ink-tertiary, #7a5035);
    --ls-accent: var(--color-ember-deep, #9a4120);
    --ls-paper: var(--color-canvas-raised, #fdfaf5);
    --ls-deep: var(--color-canvas-sunken, #f0e9da);
    --ls-line: var(--color-border-ember, rgba(192, 88, 44, 0.38));
    position: relative;
    display: inline-flex;
    align-items: center;
    font-family: var(--font-body, system-ui, sans-serif);
  }
  .ls--dark {
    --ls-ink: #fff9f0;
    --ls-soft: rgba(255, 249, 240, 0.72);
    --ls-accent: #f5c5ad;
    --ls-paper: #2a1a12;
    --ls-deep: #1d120c;
    --ls-line: rgba(245, 197, 173, 0.45);
  }

  /* ── Печать ─────────────────────────────────────────────────────────── */
  .seal {
    position: relative;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    padding: 0;
    border-radius: 50%;
    color: var(--ls-accent);
    cursor: pointer;
    background:
      radial-gradient(circle at 32% 26%, color-mix(in srgb, var(--ls-paper) 88%, #fff) 0%, transparent 62%),
      color-mix(in srgb, var(--ls-deep) 70%, transparent);
    border: 1px solid var(--ls-line);
    /* Двойная кайма, как у окон сайта: тонкая снаружи, ещё одна внутри. */
    box-shadow:
      inset 0 0 0 3px var(--ls-paper),
      inset 0 0 0 4px var(--ls-line),
      0 1px 2px rgba(60, 25, 10, 0.14);
    transition:
      transform 0.35s var(--ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1)),
      box-shadow 0.3s ease,
      color 0.25s ease;
  }
  .seal:hover,
  .ls--open .seal {
    transform: rotate(-6deg) scale(1.06);
    box-shadow:
      inset 0 0 0 3px var(--ls-paper),
      inset 0 0 0 4px var(--ls-accent),
      0 3px 10px rgba(60, 25, 10, 0.2);
  }
  .seal:focus-visible,
  .row:focus-visible,
  .chip:focus-visible {
    outline: 2px solid var(--ls-accent);
    outline-offset: 3px;
  }
  .seal:active {
    transform: scale(0.94);
  }

  .mark {
    position: relative;
    display: inline-block;
    perspective: 120px;
    font-family: var(--font-display, Georgia, serif);
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.04em;
    line-height: 1;
    color: var(--ls-ink);
  }
  .mark-in {
    display: inline-block;
    animation: mark-flip 0.5s var(--ease-atelier, cubic-bezier(0.22, 0.1, 0.2, 1)) both;
    transform-origin: 50% 60%;
  }
  @keyframes mark-flip {
    from {
      opacity: 0;
      transform: rotateX(-88deg) translateY(4px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  /* Круг, расходящийся от печати после смены, — отголосок того, что идёт по
     всей странице. */
  .ripple {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    pointer-events: none;
    border: 1px solid var(--ls-accent);
    opacity: 0;
    animation: ripple 0.9s ease-out 0.15s both;
  }
  @keyframes ripple {
    from {
      opacity: 0.7;
      transform: scale(1);
    }
    to {
      opacity: 0;
      transform: scale(2.1);
    }
  }

  /* ── Листок ─────────────────────────────────────────────────────────── */
  .sheet {
    position: absolute;
    z-index: 320;
    top: calc(100% + 12px);
    right: -4px;
    min-width: 230px;
    padding: 6px;
    background: var(--ls-paper);
    border: 1px solid var(--ls-line);
    /* Тот же поворот на градус и двойная граница, что у окон сайта. */
    transform: rotate(0.6deg);
    transform-origin: 88% 0;
    box-shadow:
      inset 0 0 0 4px var(--ls-paper),
      inset 0 0 0 5px color-mix(in srgb, var(--ls-line) 70%, transparent),
      0 14px 40px rgba(52, 37, 28, 0.2),
      0 3px 10px rgba(52, 37, 28, 0.1);
    animation: sheet-in 0.32s var(--ease-atelier, cubic-bezier(0.22, 0.1, 0.2, 1)) both;
  }
  /* Хвостик-указатель к печати. */
  .sheet::before {
    content: '';
    position: absolute;
    top: -6px;
    right: 17px;
    width: 10px;
    height: 10px;
    background: var(--ls-paper);
    border-left: 1px solid var(--ls-line);
    border-top: 1px solid var(--ls-line);
    transform: rotate(45deg);
  }
  @keyframes sheet-in {
    from {
      opacity: 0;
      transform: translateY(-8px) rotate(-1.2deg) scale(0.94);
    }
    to {
      opacity: 1;
      transform: rotate(0.6deg);
    }
  }

  .row {
    position: relative;
    display: grid;
    grid-template-columns: 2.1rem 1fr auto 1rem;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 9px 12px 9px 10px;
    border: 0;
    border-radius: 2px;
    background: transparent;
    color: var(--ls-ink);
    text-align: left;
    cursor: pointer;
    animation: row-in 0.34s var(--ease-atelier, cubic-bezier(0.22, 0.1, 0.2, 1)) both;
    animation-delay: calc(60ms + var(--i) * 55ms);
    transition: background 0.2s ease, padding-left 0.25s ease;
  }
  @keyframes row-in {
    from {
      opacity: 0;
      transform: translateX(10px);
    }
  }
  .row:hover,
  .row:focus-visible {
    background: color-mix(in srgb, var(--ls-accent) 10%, transparent);
    padding-left: 14px;
    outline-offset: -2px;
  }
  .row--on {
    background: color-mix(in srgb, var(--ls-accent) 7%, transparent);
  }
  .spec {
    font-family: var(--font-display, Georgia, serif);
    font-size: 21px;
    font-style: italic;
    line-height: 1;
    color: var(--ls-accent);
  }
  .name {
    font-family: var(--font-title, var(--font-display, Georgia, serif));
    font-size: 16.5px;
    line-height: 1.1;
  }
  .row--on .name {
    font-weight: 600;
  }
  .code {
    font-size: 10px;
    letter-spacing: 0.16em;
    color: var(--ls-soft);
  }
  .tick {
    color: var(--ls-accent);
    font-size: 11px;
    opacity: 0;
    transform: scale(0.4) rotate(-45deg);
    transition: opacity 0.25s ease, transform 0.4s var(--ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1));
  }
  .row--on .tick {
    opacity: 1;
    transform: none;
  }

  /* ── Полоса для меню на телефоне ────────────────────────────────────── */
  .ls--strip {
    position: relative;
    display: grid;
    /* `minmax(0, 1fr)`, а не `1fr`: у `1fr` нижняя граница — ширина самого
       длинного слова, и «Українська» растягивала свою колонку, а ползунок
       (ровно 1/n) съезжал с выбранной клетки. */
    grid-template-columns: repeat(var(--n), minmax(0, 1fr));
    width: min(100%, 24rem);
    padding: 3px;
    background: color-mix(in srgb, var(--ls-deep) 80%, transparent);
    border: 1px solid var(--ls-line);
    border-radius: 999px;
    isolation: isolate;
  }
  .thumb {
    position: absolute;
    z-index: -1;
    top: 3px;
    bottom: 3px;
    left: 3px;
    width: calc((100% - 6px) / var(--n));
    border-radius: 999px;
    background: var(--ls-paper);
    box-shadow:
      0 0 0 1px var(--ls-line),
      0 3px 10px rgba(52, 37, 28, 0.16);
    transform: translateX(calc(var(--at) * 100%));
    transition: transform 0.45s var(--ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1));
  }
  .chip {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    min-height: 44px;
    padding: 6px 4px 7px;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--ls-soft);
    cursor: pointer;
    transition: color 0.25s ease;
  }
  .chip[aria-checked='true'] {
    color: var(--ls-ink);
  }
  .chip-spec {
    font-family: var(--font-display, Georgia, serif);
    font-style: italic;
    font-size: 17px;
    line-height: 1;
    color: var(--ls-accent);
  }
  .chip-name {
    font-size: 10.5px;
    letter-spacing: 0.12em;
    white-space: nowrap;
  }

  @media (prefers-reduced-motion: reduce) {
    .mark-in,
    .ripple,
    .sheet,
    .row {
      animation: none;
    }
    .seal,
    .thumb,
    .tick,
    .row {
      transition: none;
    }
  }
</style>
