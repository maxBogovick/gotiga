<script lang="ts">
  /**
   * Первый экран профиля: что произошло и чего от вас ждут.
   *
   * Прежде здесь стояли четыре плитки по 9.5rem с заголовками в 1.85rem — то
   * есть весь экран уходил на меню из четырёх слов, а у человека без дел
   * четыре раза печаталось «Пусто». Всё настоящее — ответ автора, подтверждённая
   * бронь, истекающая резервация — лежало на два нажатия глубже. Теперь меню
   * разделов — рейка в один ряд над этим листом, а лист занят делами.
   */
  import { t, lang, brandName } from '$lib/i18n';
  import { userDesk } from '$lib/stores/user-desk.svelte';
  import { kindLabel, feedStatusLabel, formatDate, cardKeyOf } from '$lib/profile-labels';

  /** Сколько строк печатает первый экран: остальное — в «Делах». */
  const SHOWN = 8;

  let latest = $derived(userDesk.feed.slice(0, SHOWN));
  let untouched = $derived(
    userDesk.loaded &&
    userDesk.feed.length === 0 &&
    userDesk.threads.length === 0 &&
    userDesk.counts.wishlist === 0 &&
    userDesk.counts.watches === 0
  );
</script>

<svelte:head>
  <title>{$t('profileTitle')} — {$brandName}</title>
</svelte:head>

{#if userDesk.loading && !userDesk.loaded}
  <p class="pf-empty">…</p>
{:else if userDesk.error}
  <p class="pf-error" role="alert">{$t('authErrorServer')}</p>
  <button class="pf-btn" onclick={() => userDesk.load({ force: true })}>{$t('profileRetry')}</button>
{:else if untouched}
  <!-- Первый заход: показывать пустую ленту незачем, показывается дорога. -->
  <section class="first">
    <h2 class="pf-h2">{$t('profileFirstTitle')}</h2>
    <p class="pf-note">{$t('profileFirstNote')}</p>
    <div class="first-ways">
      <a class="pf-btn" href="/figurines">{$t('profileFirstArchive')}</a>
      <a class="pf-link" href="/commission">{$t('profileCommissionsNew')} →</a>
    </div>
  </section>
{:else}
  {#if userDesk.counts.unread > 0}
    <!-- Единственная строка на листе, которая просит о чём-то. Поэтому она одна
         и покрашена, а не одна из пяти одинаковых. -->
    <a class="call" href="/profile/messages">
      <span class="call-word">{$t('profileLatestNewReplies')}</span>
      <span class="pf-count pf-count--new">{userDesk.counts.unread}</span>
      <span class="call-go" aria-hidden="true">→</span>
    </a>
  {/if}

  {#if userDesk.feed.length > 0}
    <h2 class="pf-kicker head">{$t('profileTabLatest')}</h2>
    <ul class="pf-rows">
      {#each latest as item (item.kind + ':' + item.id)}
        <li class="pf-row">
          <a class="row" href="/profile/dealings/{encodeURIComponent(cardKeyOf(item))}">
            <span class="row-date">{formatDate($lang, item.date)}</span>
            <span class="row-main">
              <span class="row-kind">{kindLabel($t, item.kind)}</span>
              <span class="row-title">{item.title || $t('commissionUntitled')}</span>
            </span>
            <span class="row-end">
              {#if item.unread > 0}
                <span class="pf-count pf-count--new">{item.unread}</span>
              {/if}
              <span class="pf-tone pf-tone--{item.tone}">{feedStatusLabel($t, item)}</span>
            </span>
          </a>
        </li>
      {/each}
    </ul>
    {#if userDesk.feed.length > SHOWN}
      <a class="pf-link more" href="/profile/dealings">{$t('profileLatestAllDeals')} →</a>
    {/if}
  {:else}
    <p class="pf-empty">{$t('profileLatestNoDeals')}</p>
    <a class="pf-link" href="/figurines">{$t('profileFirstArchive')} →</a>
  {/if}
{/if}

<style>
  .head { display: block; margin: 0 0 0.7rem; }

  .call {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.75rem 1rem;
    margin-bottom: 1.6rem;
    background: var(--color-ember-subtle);
    border: 1px solid var(--color-border-ember);
    text-decoration: none;
    transition: border-color 0.15s;
  }
  .call:hover { border-color: var(--color-ember); }

  .call-word {
    font-family: var(--font-body);
    font-size: 0.88rem;
    color: var(--color-ember-ink);
  }
  .call-go { margin-left: auto; color: var(--color-ember-ink); }

  .row {
    display: grid;
    grid-template-columns: 9.5rem 1fr auto;
    align-items: baseline;
    gap: 0.35rem 1rem;
    padding: 0.85rem 0.2rem;
    text-decoration: none;
    transition: background 0.15s;
  }
  .row:hover { background: var(--color-canvas-raised); }

  .row-date {
    font-family: var(--font-body);
    font-size: 0.78rem;
    color: var(--color-ink-tertiary);
    font-variant-numeric: tabular-nums;
  }

  .row-main { min-width: 0; display: flex; flex-direction: column; gap: 0.15rem; }

  .row-kind {
    font-family: var(--font-body);
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
  }

  .row-title {
    font-family: var(--font-serif);
    font-size: 1.02rem;
    line-height: 1.3;
    color: var(--color-ink-primary);
    overflow-wrap: anywhere;
  }

  .row-end { display: flex; align-items: center; gap: 0.5rem; white-space: nowrap; }

  .more { display: inline-block; margin-top: 1rem; }

  .first { padding: 1.5rem 0 0; max-width: 34rem; }
  .first .pf-note { margin-top: 0.6rem; }
  .first-ways {
    display: flex;
    align-items: center;
    gap: 1.1rem;
    flex-wrap: wrap;
    margin-top: 1.4rem;
  }
  .first-ways .pf-btn { text-decoration: none; }

  @media (max-width: 620px) {
    .row {
      grid-template-columns: 1fr auto;
    }
    .row-date { grid-column: 1 / -1; order: -1; }
  }
</style>
