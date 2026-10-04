<script lang="ts">
  /** Слежение за листами вестника: кому дом обещал написать, когда выйдет. */
  import { t, lang, brandName } from '$lib/i18n';
  import { api } from '$lib/api';
  import { userDesk } from '$lib/stores/user-desk.svelte';
  import type { GazetteWatchDto } from '$lib/types/api';

  let releasing = $state<string | null>(null);
  let error = $state('');

  function title(w: GazetteWatchDto): string {
    const ru = $lang === 'ru' && w.titleRu.trim();
    return (ru ? w.titleRu : w.titleEn).trim();
  }

  async function release(w: GazetteWatchDto) {
    if (releasing) return;
    releasing = w.id;
    error = '';
    try {
      await api.leaveGazetteWatchByToken(w.cancelToken);
      userDesk.dropWatch(w.id);
      try { localStorage.removeItem(`gotiga_gazette_watch_${w.leafId}`); } catch { /* ignore */ }
    } catch {
      error = $t('profileActionError');
    } finally {
      releasing = null;
    }
  }
</script>

<svelte:head>
  <title>{$t('profileWatches')} — {$brandName}</title>
</svelte:head>

{#if error}<p class="pf-error" role="alert">{error}</p>{/if}

{#if userDesk.loading && !userDesk.loaded}
  <p class="pf-empty">…</p>
{:else if userDesk.watches.length === 0}
  <p class="pf-empty">{$t('profileWatchEmpty')}</p>
{:else}
  <ul class="pf-rows">
    {#each userDesk.watches as w (w.id)}
      <li class="pf-row row">
        <a class="title" href="/gazette/{w.leafSlug}">{title(w)}</a>
        {#if w.notifiedAt}
          <span class="pf-quiet">{$t('gazetteWatchAlreadyTold')}</span>
        {/if}
        <button class="pf-link pf-link--danger" onclick={() => release(w)} disabled={releasing === w.id}>
          {releasing === w.id ? '…' : $t('profileWatchRelease')}
        </button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.5rem 1rem;
    padding: 0.9rem 0.2rem;
  }

  .title {
    font-family: var(--font-serif);
    font-size: 1.02rem;
    color: var(--color-ink-primary);
    text-decoration: none;
    overflow-wrap: anywhere;
  }
  .title:hover { color: var(--color-ember-deep); }

  .row .pf-link { margin-left: auto; }
</style>
