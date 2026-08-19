<script lang="ts">
  import { t } from '$lib/i18n';
  import { detailHeader } from '$lib/stores/detail-header.svelte';

  let active = $derived(detailHeader.active);
  let storySaving = $derived(detailHeader.storySaving);
  let copied = $derived(detailHeader.copied);
</script>

{#if active}
  <div class="leaf-tools">
    <button
      type="button"
      class="leaf-tool"
      class:is-on={storySaving}
      onclick={() => detailHeader.openStoryModal()}
      disabled={storySaving}
      aria-label={storySaving ? $t('figurineStorySaving') : $t('figurineStoryShare')}
      title={storySaving ? $t('figurineStorySaving') : $t('figurineStoryShare')}
    >
      {#if storySaving}
        <span class="leaf-spin" aria-hidden="true"></span>
      {:else}
        <svg width="16" height="16" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.35" aria-hidden="true">
          <rect x="2" y="1.5" width="9" height="10" rx="1.2"/>
          <path d="M4.1 8.5 6 6.8l1.3 1.1 1.6-2 1.1 1.4"/>
          <circle cx="4.6" cy="4.1" r="0.65" fill="currentColor" stroke="none"/>
        </svg>
      {/if}
    </button>

    <button
      type="button"
      class="leaf-tool"
      class:is-on={copied}
      onclick={() => detailHeader.share()}
      aria-label={copied ? $t('figurineCopied') : $t('figurineShare')}
      title={copied ? $t('figurineCopied') : $t('figurineShare')}
    >
      {#if copied}
        <svg width="15" height="15" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
          <path d="M2 6l3 3 5-5"/>
        </svg>
      {:else}
        <svg width="15" height="15" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
          <path d="M9 1.5a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3zM3 4.5a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3zM9 7.5a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3z"/>
          <path d="M7.5 2.7l-3 1.8M7.5 9.3l-3-1.8"/>
        </svg>
      {/if}
    </button>
  </div>
{/if}

<style>
  .leaf-tools {
    display: flex;
    align-items: stretch;
    gap: 0;
  }

  .leaf-tool {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    padding: 0;
    border: none;
    border-radius: 0;
    background: transparent;
    color: #c65f3c;
    cursor: pointer;
    transition: background 0.2s ease, color 0.2s ease;
  }

  .leaf-tool:hover,
  .leaf-tool:focus-visible,
  .leaf-tool.is-on {
    background: #c65f3c;
    color: #fff9f0;
  }

  .leaf-tool:focus-visible {
    outline: 2px solid color-mix(in srgb, #c65f3c 55%, transparent);
    outline-offset: 2px;
  }

  .leaf-tool:disabled {
    cursor: wait;
    opacity: 0.68;
  }

  .leaf-spin {
    width: 13px;
    height: 13px;
    border: 1.5px solid currentColor;
    border-right-color: transparent;
    border-radius: 999px;
    animation: leaf-spin 0.75s linear infinite;
  }

  @keyframes leaf-spin {
    to { transform: rotate(360deg); }
  }

  @media (prefers-reduced-motion: reduce) {
    .leaf-spin { animation: none; }
  }
</style>
