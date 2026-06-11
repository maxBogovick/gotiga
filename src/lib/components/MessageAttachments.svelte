<script lang="ts">
  import { resolveMediaUrl } from '$lib/api';
  import type { AttachmentDto } from '$lib/types/api';

  let { attachments = [] as AttachmentDto[] } = $props();

  let lightboxUrl = $state<string | null>(null);
</script>

{#if attachments.length > 0}
  <div class="att-grid">
    {#each attachments as att (att.id)}
      <button
        type="button"
        class="att-thumb"
        onclick={() => (lightboxUrl = resolveMediaUrl(att.url))}
        title="Открыть изображение"
      >
        <img src={resolveMediaUrl(att.thumbUrl ?? att.url)} alt="" loading="lazy" />
      </button>
    {/each}
  </div>
{/if}

{#if lightboxUrl}
  <div
    class="att-lightbox"
    role="button"
    tabindex="0"
    aria-label="Закрыть"
    onclick={() => (lightboxUrl = null)}
    onkeydown={(e) => { if (e.key === 'Escape' || e.key === 'Enter' || e.key === ' ') lightboxUrl = null; }}
  >
    <img src={lightboxUrl} alt="" />
  </div>
{/if}

<style>
  .att-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-top: 0.5rem;
  }
  .att-thumb {
    width: 72px;
    height: 72px;
    padding: 0;
    border: 1px solid var(--border, #d8c6b1);
    background: #fff;
    cursor: pointer;
    overflow: hidden;
    transition: border-color 0.2s, transform 0.2s;
  }
  .att-thumb:hover {
    border-color: var(--accent, #c65f3c);
    transform: translateY(-1px);
  }
  .att-thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .att-lightbox {
    position: fixed;
    inset: 0;
    z-index: 300;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    background: rgba(111, 59, 36, 0.6);
    backdrop-filter: blur(4px);
    cursor: zoom-out;
  }
  .att-lightbox img {
    max-width: 90vw;
    max-height: 90vh;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
    border: 4px solid #f8f1e7;
  }
</style>
