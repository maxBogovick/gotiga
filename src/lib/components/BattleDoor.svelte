<script lang="ts">
  // Одна дверь на три комнаты. Полка, колода и этюды — не три адреса, которые
  // страница объясняет абзацами, а три положения одной руки. Активное — цветом
  // слова, не заливкой: это не вкладки лавки.
  //
  // Кошелёк принадлежит всем трём и стоит в этой строке, не в теле полки.
  // Задания висят на нём одним счётом; список открывается здесь же, чтобы
  // возврат в комнату не начинался с формы поручений.
  import { onMount } from 'svelte';
  import { slide } from 'svelte/transition';
  import { page } from '$app/state';
  import { t, type TranslationKey } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import BattleWallet from '$lib/components/BattleWallet.svelte';
  import BattleErrandList from '$lib/components/BattleErrandList.svelte';
  import BattleGreeting from '$lib/components/BattleGreeting.svelte';
  import type { BattleErrand, BattleMe } from '$lib/types/api';

  type Room = 'shelf' | 'table' | 'etude';

  let {
    me = null,
    settled = null,
    errands,
  }: {
    me?: BattleMe | null;
    settled?: number | null;
    /** Не передали — дверь читает сама. Пустой список — заданий нет. */
    errands?: BattleErrand[];
  } = $props();

  const ROOMS: { id: Room; href: string; key: TranslationKey }[] = [
    { id: 'shelf', href: '/battles', key: 'battlesRoomShelf' },
    { id: 'table', href: '/battles/table', key: 'battlesRoomTable' },
    { id: 'etude', href: '/battles/etude', key: 'battlesRoomEtude' },
  ];

  let here = $derived<Room>(
    page.url.pathname.startsWith('/battles/etude')
      ? 'etude'
      : page.url.pathname.startsWith('/battles/table')
        ? 'table'
        : 'shelf',
  );

  let ownMe = $state<BattleMe | null>(null);
  let ownErrands = $state<BattleErrand[]>([]);

  let book = $derived(me ?? ownMe);
  let tasks = $derived(errands ?? ownErrands);

  let dailyLeft = $derived(
    tasks.filter((e) => !e.byHand && e.period !== 'once' && !e.done).length,
  );
  let onceLeft = $derived(
    tasks.filter((e) => !e.byHand && e.period === 'once' && !e.done).length,
  );
  let left = $derived(dailyLeft + onceLeft);

  let historyOpen = $state(false);
  let errandsOpen = $state(false);
  let about = $state(false);
  let stuck = $state(false);
  let mark = $state<HTMLElement | null>(null);

  $effect(() => {
    const el = mark;
    if (!el || typeof IntersectionObserver === 'undefined') return;
    const watcher = new IntersectionObserver(
      ([entry]) => {
        stuck = !entry.isIntersecting && entry.boundingClientRect.top < 0;
        if (!stuck) historyOpen = false;
      },
      { threshold: 0 },
    );
    watcher.observe(el);
    return () => watcher.disconnect();
  });

  function toggleHistory() {
    historyOpen = !historyOpen;
    if (historyOpen) errandsOpen = false;
  }

  function toggleErrands() {
    errandsOpen = !errandsOpen;
    if (errandsOpen) historyOpen = false;
  }

  onMount(async () => {
    const token = authStore.token;
    if (!token) return;
    if (me == null) {
      try {
        ownMe = await api.getBattleMe(token);
      } catch {
        ownMe = null;
      }
    }
    if (errands === undefined) {
      try {
        ownErrands = await api.getBattleErrands(token);
      } catch {
        ownErrands = [];
      }
    }
  });
</script>

<div class="door" class:door--stuck={stuck}>
  <nav class="row" aria-label={$t('battlesPageTitle')}>
    {#each ROOMS as room (room.id)}
      <a
        href={room.href}
        class="room"
        class:room--here={here === room.id}
        aria-current={here === room.id ? 'page' : undefined}>{$t(room.key)}</a
      >
    {/each}

    {#if book}
      <span class="grow"></span>
      <BattleWallet dust={book.dust} feed={book.feed} {settled} compact={true} />
      {#if tasks.length}
        <button
          type="button"
          class="quiet"
          onclick={toggleErrands}
          aria-expanded={errandsOpen}
        >
          {#if dailyLeft > 0}
            {$t('battlesTasksToday')} {dailyLeft}
          {:else if left > 0}
            {$t('battlesErrands')} {left}
          {:else}
            {$t('battlesErrands')}
          {/if}
        </button>
      {/if}
      {#if book.gifts.length}
        <button
          type="button"
          class="quiet quiet--extra"
          onclick={toggleHistory}
          aria-expanded={historyOpen}>{$t('battlesWalletHistory')}</button
        >
      {/if}
      <button
        type="button"
        class="quiet quiet--extra"
        onclick={() => (about = true)}>{$t('battlesWalletAbout')}</button
      >
    {/if}
  </nav>

  {#if historyOpen && book?.gifts.length}
    <ul class="history" transition:slide={{ duration: 200 }}>
      {#each book.gifts as g (g.at)}
        <li class="gift">
          <span class="gift-why">
            {g.reason === 'welcome' ? $t('battlesGiftWelcome') : (g.note ?? '')}
          </span>
          <span class="gift-sum">
            <i class="gift-dot gift-dot--{g.currency}" aria-hidden="true"></i>{g.amount}
          </span>
        </li>
      {/each}
    </ul>
  {/if}

  {#if errandsOpen && tasks.length}
    <div class="errand-panel" transition:slide={{ duration: 200 }}>
      <BattleErrandList errands={tasks} nest={true} />
    </div>
  {/if}
</div>
<div bind:this={mark} class="door-mark" aria-hidden="true"></div>

{#if about}
  <BattleGreeting mode="about" errands={tasks} onclose={() => (about = false)} />
{/if}

<style>
  .door {
    position: sticky;
    top: 54px;
    z-index: 40;
    margin: 0 -1.5rem 2.5rem;
    padding: 0 1.5rem;
  }

  .door--stuck {
    background: rgba(248, 241, 231, 0.94);
    backdrop-filter: blur(6px);
    border-bottom: 1px solid #d8c6b1;
  }

  .door-mark {
    height: 1px;
    margin-top: -2.5rem;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 1.1rem;
    min-height: 2.6rem;
  }

  .room {
    font-size: 0.72rem;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #8a6a55;
    text-decoration: none;
    border-bottom: 1px solid transparent;
  }

  .room:hover {
    color: #c65f3c;
  }

  .room--here {
    color: #34251c;
    border-bottom-color: rgba(52, 37, 28, 0.35);
  }

  .grow {
    flex: 1 1 auto;
  }

  .quiet {
    padding: 0 0 1px;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #8a6a55;
    background: none;
    border: none;
    border-bottom: 1px solid rgba(138, 106, 85, 0.35);
    cursor: pointer;
    white-space: nowrap;
  }

  .quiet:hover {
    color: #c65f3c;
    border-bottom-color: rgba(198, 95, 60, 0.5);
  }

  .history {
    margin: 0;
    padding: 0.5rem 0 0.7rem;
    list-style: none;
    max-width: 34rem;
  }

  .gift {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.18rem 0;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.78rem;
    color: #6f3b24;
  }

  .gift-why {
    color: #8a6a55;
  }

  .gift-sum {
    display: inline-flex;
    align-items: baseline;
    gap: 0.35rem;
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }

  .gift-dot {
    width: 0.42rem;
    height: 0.42rem;
    border-radius: 50%;
    align-self: center;
  }

  .gift-dot--dust {
    background: #cbb79c;
  }

  .gift-dot--feed {
    background: #6f3b24;
  }

  .errand-panel {
    padding: 0.4rem 0 0.8rem;
    max-height: 60vh;
    overflow-y: auto;
  }

  @media (max-width: 40rem) {
    .door {
      top: 52px;
    }

    .row {
      flex-wrap: wrap;
      gap: 0.35rem 0.8rem;
      padding: 0.3rem 0;
      min-height: 0;
    }

    .grow {
      order: 1;
      flex-basis: 100%;
      height: 0;
    }

    .quiet {
      order: 2;
      font-size: 0.62rem;
    }

    /* Прилипнув, строка оставляет комнаты и числа. Журнал и объяснение
       не открывают на бегу вдоль полки. */
    .door--stuck .quiet--extra {
      display: none;
    }

    .door--stuck .grow {
      display: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .door--stuck {
      backdrop-filter: none;
    }
  }
</style>
