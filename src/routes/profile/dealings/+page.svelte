<script lang="ts">
  /**
   * Дела: по карточке на работу, со всеми делами, какие на ней есть.
   *
   * Вид отбора стоит в адресе (`?kind=`), а не только в памяти компонента:
   * привязав расписку по коду, профиль уводит человека именно к тому виду дел,
   * который он только что привязал, — а адрес это единственный способ сказать
   * «открой вот это» другой странице.
   */
  import { page } from '$app/state';
  import { replaceState } from '$app/navigation';
  import { t, lang, brandName } from '$lib/i18n';
  import AppImage from '$lib/components/AppImage.svelte';
  import { userDesk, type FeedKind } from '$lib/stores/user-desk.svelte';
  import { kindLabel, formatDate } from '$lib/profile-labels';

  type Filter = 'all' | FeedKind;
  const FILTERS: Filter[] = ['all', 'booking', 'order', 'commission', 'waitlist'];

  let filter = $derived<Filter>(
    (FILTERS as string[]).includes(page.url.searchParams.get('kind') ?? '')
      ? (page.url.searchParams.get('kind') as Filter)
      : 'all'
  );

  let cards = $derived(
    filter === 'all'
      ? userDesk.workCards
      : userDesk.workCards.filter((c) => c.kinds.includes(filter as FeedKind))
  );

  function countOf(f: Filter): number {
    return f === 'all'
      ? userDesk.workCards.length
      : userDesk.workCards.filter((c) => c.kinds.includes(f as FeedKind)).length;
  }

  function filterLabel(f: Filter): string {
    return f === 'all' ? $t('profileRailAll')
      : f === 'booking' ? $t('profileBookings')
      : f === 'order' ? $t('profileOrders')
      : f === 'commission' ? $t('profileCommissions')
      : $t('profileLinkClaimKindWaitlist');
  }

  /**
   * Отбор — не место, а взгляд на него: он переписывает адрес, а не
   * добавляет запись в историю. Иначе «назад» после пяти нажатий по отборам
   * пять раз возвращало бы отбор вместо ухода из раздела.
   */
  function choose(f: Filter) {
    const url = new URL(page.url);
    if (f === 'all') url.searchParams.delete('kind');
    else url.searchParams.set('kind', f);
    replaceState(url, page.state);
  }
</script>

<svelte:head>
  <title>{$t('profileHubDealings')} — {$brandName}</title>
</svelte:head>

<nav class="filters" aria-label={$t('profileHubDealings')}>
  {#each FILTERS as f (f)}
    {@const n = countOf(f)}
    <button
      type="button"
      class="filter"
      class:on={filter === f}
      aria-pressed={filter === f}
      onclick={() => choose(f)}
    >
      {filterLabel(f)}{#if n > 0}<span class="pf-count">{n}</span>{/if}
    </button>
  {/each}
</nav>

{#if userDesk.loading && !userDesk.loaded}
  <p class="pf-empty">…</p>
{:else if filter === 'commission' && userDesk.commissions.length === 0}
  <p class="pf-empty">{$t('profileCommissionsEmpty')}</p>
  <a class="pf-btn" href="/commission">{$t('profileCommissionsNew')}</a>
{:else if cards.length === 0}
  <p class="pf-empty">{$t('profileEmpty')}</p>
{:else}
  <ul class="grid">
    {#each cards as card (card.key)}
      {@const fig = card.figurineId ? userDesk.figurineById.get(card.figurineId) : undefined}
      <li>
        <a class="work" href="/profile/dealings/{encodeURIComponent(card.key)}">
          <span class="work-thumb">
            {#if fig?.faceImageUrl}
              <AppImage src={fig.faceImageUrl} thumbUrl={fig.thumbUrl} alt={card.title} class="work-img" loading="lazy" />
            {:else}
              <span class="work-ph" aria-hidden="true">{(card.title || '?').charAt(0)}</span>
            {/if}
          </span>
          <span class="work-body">
            <span class="work-name">{card.title || $t('commissionUntitled')}</span>
            <span class="work-kinds">
              {#each card.kinds as k (k)}<span class="work-kind">{kindLabel($t, k)}</span>{/each}
            </span>
            <span class="work-foot">
              <span class="work-date">{formatDate($lang, card.date)}</span>
              {#if card.unread > 0}
                <span class="pf-count pf-count--new">{card.unread}</span>
              {/if}
            </span>
          </span>
        </a>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-bottom: 1.4rem;
  }

  .filter {
    display: inline-flex;
    align-items: baseline;
    gap: 0.35rem;
    font-family: var(--font-body);
    font-size: 0.78rem;
    color: var(--color-ink-tertiary);
    background: transparent;
    border: 1px solid transparent;
    border-radius: 2px;
    padding: 0.3rem 0.65rem;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }
  .filter:hover { color: var(--color-ink-primary); }
  .filter.on {
    color: var(--color-ember-ink);
    border-color: var(--color-border-ember);
    background: var(--color-ember-subtle);
  }

  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 1rem;
  }

  .work {
    display: flex;
    gap: 0.85rem;
    height: 100%;
    padding: 0.75rem;
    background: var(--color-canvas-raised);
    border: 1px solid var(--color-border-subtle);
    text-decoration: none;
    transition: border-color 0.15s;
  }
  .work:hover { border-color: var(--color-border-ember); }

  .work-thumb {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 62px;
    height: 78px;
    flex-shrink: 0;
    overflow: hidden;
    background: var(--color-canvas-sunken);
    border: 1px solid var(--color-border-subtle);
  }
  .work-thumb :global(.work-img) { width: 100%; height: 100%; object-fit: cover; }
  .work-ph {
    font-family: var(--font-display);
    font-size: 1.4rem;
    color: var(--color-ink-tertiary);
  }

  .work-body {
    display: flex;
    flex-direction: column;
    gap: 0.28rem;
    min-width: 0;
    flex: 1;
  }

  .work-name {
    font-family: var(--font-serif);
    font-size: 1rem;
    line-height: 1.25;
    color: var(--color-ink-primary);
    overflow-wrap: anywhere;
  }

  .work-kinds { display: flex; flex-wrap: wrap; gap: 0.3rem; }

  .work-kind {
    font-family: var(--font-body);
    font-size: 0.7rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
    border: 1px solid var(--color-border-subtle);
    padding: 0.08rem 0.35rem;
  }

  .work-foot {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: auto;
    padding-top: 0.35rem;
  }

  .work-date {
    font-family: var(--font-body);
    font-size: 0.72rem;
    color: var(--color-ink-tertiary);
  }
</style>
