<script lang="ts">
  /** Отложенное: работы, отмеченные для себя. Живёт и без входа — стор свой. */
  import { t, brandName } from '$lib/i18n';
  import AppImage from '$lib/components/AppImage.svelte';
  import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
  import { userDesk } from '$lib/stores/user-desk.svelte';
  import { wishStatusLabel } from '$lib/profile-labels';

  let ids = $derived(savedFigurines.ids);
</script>

<svelte:head>
  <title>{$t('profileWishlist')} — {$brandName}</title>
</svelte:head>

{#if savedFigurines.syncError}
  <p class="pf-error">{$t('profileWishlistSyncError')}</p>
{/if}

{#if ids.length === 0}
  <p class="pf-empty">{$t('profileEmpty')}</p>
  <a class="pf-link" href="/figurines">{$t('profileFirstArchive')} →</a>
{:else}
  <ul class="grid">
    {#each ids as id (id)}
      {@const item = userDesk.figurineById.get(id)}
      <li class="cell">
        <a class="thumb" href="/figurines/{id}" aria-label={item?.name ?? id}>
          {#if item?.faceImageUrl}
            <AppImage src={item.faceImageUrl} thumbUrl={item.thumbUrl} alt={item.name} class="thumb-img" loading="lazy" />
          {:else}
            <span class="thumb-ph" aria-hidden="true">✦</span>
          {/if}
        </a>
        <div class="copy">
          <a class="name" href="/figurines/{id}">{item?.name ?? id}</a>
          <p class="meta">
            {#if item?.year}<span>{item.year}</span><span class="dot" aria-hidden="true">·</span>{/if}
            {#if item}<span>{wishStatusLabel($t, item.status)}</span>{/if}
          </p>
        </div>
        <button
          class="drop"
          onclick={() => savedFigurines.remove(id)}
          aria-label={$t('profileWishRemove')}
          title={$t('profileWishRemove')}
        >×</button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: 1rem;
  }

  .cell {
    position: relative;
    display: flex;
    flex-direction: column;
    background: var(--color-canvas-raised);
    border: 1px solid var(--color-border-subtle);
  }

  .thumb {
    display: flex;
    align-items: center;
    justify-content: center;
    aspect-ratio: 4 / 5;
    overflow: hidden;
    background: var(--color-canvas-sunken);
  }
  .thumb :global(.thumb-img) { width: 100%; height: 100%; object-fit: cover; }
  .thumb-ph { color: var(--color-ink-tertiary); font-size: 1.2rem; }

  .copy { padding: 0.6rem 0.7rem 0.75rem; }

  .name {
    font-family: var(--font-serif);
    font-size: 1rem;
    line-height: 1.25;
    color: var(--color-ink-primary);
    text-decoration: none;
    overflow-wrap: anywhere;
  }
  .name:hover { color: var(--color-ember-deep); }

  .meta {
    display: flex;
    align-items: baseline;
    gap: 0.3rem;
    font-family: var(--font-body);
    font-size: 0.75rem;
    color: var(--color-ink-tertiary);
    margin: 0.25rem 0 0;
  }
  .dot { color: var(--color-ink-disabled); }

  .drop {
    position: absolute;
    top: 0.35rem;
    right: 0.35rem;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 1px solid var(--color-border-default);
    background: var(--color-canvas-raised);
    color: var(--color-ink-secondary);
    font-size: 0.95rem;
    line-height: 1;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
  }
  .drop:hover { color: var(--color-ember-deep); border-color: var(--color-border-ember); }
</style>
