<script lang="ts">
  /**
   * Одна работа и всё, что у человека с ней связано.
   *
   * Ключ карточки — это `figurineId`, а у прошения, которое работой ещё не
   * стало, `kind:id`. Он стоит в адресе, поэтому на карточку можно дать ссылку
   * и «назад» возвращает в список дел, а не уводит с профиля.
   */
  import { page } from '$app/state';
  import { t, brandName } from '$lib/i18n';
  import AppImage from '$lib/components/AppImage.svelte';
  import CommissionEditModal from '$lib/components/CommissionEditModal.svelte';
  import DealEntry from '$lib/components/profile/DealEntry.svelte';
  import { userDesk } from '$lib/stores/user-desk.svelte';
  import { kindLabel } from '$lib/profile-labels';
  import type { CommissionDto } from '$lib/types/api';

  let card = $derived(userDesk.cardByKey(page.params.key ?? ''));
  let fig = $derived(card?.figurineId ? userDesk.figurineById.get(card.figurineId) : undefined);

  let editing = $state<CommissionDto | null>(null);

  function onSaved(updated: CommissionDto) {
    userDesk.replaceCommission(updated);
    editing = null;
  }
</script>

<svelte:head>
  <title>{card?.title || $t('profileHubDealings')} — {$brandName}</title>
</svelte:head>

<a class="pf-crumb" href="/profile/dealings">← {$t('profileHubDealings')}</a>

{#if !card}
  {#if userDesk.loading || !userDesk.loaded}
    <p class="pf-empty">…</p>
  {:else}
    <p class="pf-empty">{$t('profileEmpty')}</p>
  {/if}
{:else}
  <header class="head">
    {#if card.figurineId}
      <a class="head-thumb" href="/figurines/{card.figurineId}" aria-label={card.title}>
        {#if fig?.faceImageUrl}
          <AppImage src={fig.faceImageUrl} thumbUrl={fig.thumbUrl} alt={card.title} class="head-img" loading="lazy" />
        {:else}
          <span class="head-ph" aria-hidden="true">{(card.title || '?').charAt(0)}</span>
        {/if}
      </a>
    {/if}
    <div class="head-copy">
      {#if card.figurineId}
        <a class="head-name head-name--link" href="/figurines/{card.figurineId}">{card.title || $t('commissionUntitled')}</a>
      {:else}
        <h1 class="head-name">{card.title || $t('commissionUntitled')}</h1>
      {/if}
      <!-- Виды дел, а не их число: «1 · дел по этой работе» не читается ни на
           одном языке, а перечисление сразу говорит, о чём здесь речь. -->
      <p class="pf-quiet">{card.kinds.map((k) => kindLabel($t, k)).join(' · ')}</p>
    </div>
  </header>

  <ul class="entries">
    {#each card.items as item (item.kind + ':' + item.id)}
      <DealEntry {item} onedit={(c) => (editing = c)} />
    {/each}
  </ul>
{/if}

{#if editing}
  <CommissionEditModal
    commission={editing}
    onClose={() => (editing = null)}
    onSaved={onSaved}
  />
{/if}

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding-bottom: 1.1rem;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 12%, transparent);
  }

  .head-thumb {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 66px;
    height: 84px;
    flex-shrink: 0;
    overflow: hidden;
    background: var(--color-canvas-sunken);
    border: 1px solid var(--color-border-subtle);
  }
  .head-thumb :global(.head-img) { width: 100%; height: 100%; object-fit: cover; }
  .head-ph {
    font-family: var(--font-display);
    font-size: 1.5rem;
    color: var(--color-ink-tertiary);
  }

  .head-copy { min-width: 0; }

  .head-name {
    display: block;
    font-family: var(--font-display);
    font-size: 1.4rem;
    font-weight: 400;
    line-height: 1.2;
    margin: 0;
    color: var(--color-ink-primary);
    text-decoration: none;
    overflow-wrap: anywhere;
  }
  .head-name--link:hover { color: var(--color-ember-deep); }

  .head-copy .pf-quiet { display: block; margin-top: 0.3rem; }

  .entries { list-style: none; margin: 0; padding: 0; }
</style>
