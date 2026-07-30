<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { FigurineListItem, HomeContent } from '$lib/types/api';
  import { t } from '$lib/i18n';

  let content = $state<HomeContent>({
    title: null,
    kicker: '',
    lead: '',
    heroFigurineId: null,
    heroCaptionTitle: '',
    heroCaptionMeta: '',
    heroCaptionCta: '',
    heroMode: 'auto',
    vitrineFigurineId: null,
  });
  let figurines = $state<FigurineListItem[]>([]);
  let bgImage = $state<string | null>(null);
  let savedSnapshot = $state('');
  let isLoading = $state(true);
  let isSaving = $state(false);
  let isUploadingBg = $state(false);
  let message = $state('');

  let hasUnsaved = $derived(JSON.stringify(content) !== savedSnapshot);
  let selectedHeroFigurine = $derived(
    content.heroFigurineId
      ? figurines.find((item) => item.id === content.heroFigurineId) ?? null
      : null
  );
  let selectedVitrineFigurine = $derived(
    content.vitrineFigurineId
      ? figurines.find((item) => item.id === content.vitrineFigurineId) ?? null
      : null
  );

  let vitrineSearch = $state('');
  let filteredVitrineFigurines = $derived(
    vitrineSearch.trim()
      ? figurines.filter((item) => item.name.toLowerCase().includes(vitrineSearch.trim().toLowerCase()))
      : figurines
  );

  function normalizeHomeContent(loaded: HomeContent): HomeContent {
    const legacyCaption = loaded.heroCaptionTitle ?? loaded.title ?? '';
    return {
      title: null,
      kicker: loaded.kicker ?? '',
      lead: loaded.lead ?? '',
      heroFigurineId: loaded.heroFigurineId ?? null,
      heroCaptionTitle: legacyCaption,
      heroCaptionMeta: loaded.heroCaptionMeta ?? '',
      heroCaptionCta: loaded.heroCaptionCta ?? '',
      heroMode: loaded.heroMode ?? 'auto',
      vitrineFigurineId: loaded.vitrineFigurineId ?? null,
    };
  }

  onMount(async () => {
    try {
      const [loaded, loadedFigurines, loadedBg] = await Promise.all([
        api.getHomeContent(),
        api.getAllFigurinesAdmin().catch(() => api.getAllFigurines()),
        api.getMainBackground().catch(() => null),
      ]);
      figurines = loadedFigurines;
      bgImage = loadedBg;
      content = normalizeHomeContent(loaded);
      savedSnapshot = JSON.stringify(content);
    } finally {
      isLoading = false;
    }
  });

  function selectHeroFigurine(id: string) {
    content.heroFigurineId = id || null;
    const fig = figurines.find((item) => item.id === id) ?? null;
    if (fig && !content.heroCaptionTitle?.trim()) content.heroCaptionTitle = fig.name;
  }

  function clearHeroFigurine() {
    content.heroFigurineId = null;
  }

  function selectVitrineFigurine(id: string) {
    content.vitrineFigurineId = id || null;
  }

  function clearVitrineFigurine() {
    content.vitrineFigurineId = null;
  }

  function useSelectedName() {
    if (!selectedHeroFigurine) return;
    content.heroCaptionTitle = selectedHeroFigurine.name;
  }

  async function changeBackground() {
    isUploadingBg = true;
    message = '';
    try {
      const fileOrPath: string | File = await new Promise<File>((resolve, reject) => {
        const input = document.createElement('input');
        input.type = 'file';
        input.accept = 'image/jpeg,image/png,image/webp';
        input.onchange = () => {
          const file = input.files?.[0];
          if (file) resolve(file); else reject(new Error('no file'));
        };
        input.click();
      });
      bgImage = await api.setMainBackground(fileOrPath);
      message = 'Hero photo updated';
    } catch (e) {
      if (String(e) !== 'Error: no file') {
        console.error('Failed to update hero image:', e);
        message = 'Failed to update hero photo';
      }
    } finally {
      isUploadingBg = false;
    }
  }

  async function save() {
    isSaving = true;
    message = '';
    try {
      const payload = {
        title: null,
        kicker: content.kicker?.trim() || null,
        lead: content.lead?.trim() || null,
        heroFigurineId: content.heroFigurineId || null,
        heroCaptionTitle: content.heroCaptionTitle?.trim() || null,
        heroCaptionMeta: content.heroCaptionMeta?.trim() || null,
        heroCaptionCta: content.heroCaptionCta?.trim() || null,
        heroMode: content.heroMode ?? 'auto',
        vitrineFigurineId: content.vitrineFigurineId || null,
      };
      await api.saveHomeContent(payload);
      content = {
        title: null,
        kicker: payload.kicker ?? '',
        lead: payload.lead ?? '',
        heroFigurineId: payload.heroFigurineId,
        heroCaptionTitle: payload.heroCaptionTitle ?? '',
        heroCaptionMeta: payload.heroCaptionMeta ?? '',
        heroCaptionCta: payload.heroCaptionCta ?? '',
        heroMode: payload.heroMode,
        vitrineFigurineId: payload.vitrineFigurineId,
      };
      savedSnapshot = JSON.stringify(content);
      message = 'Home page updated';
    } catch (e) {
      console.error('Failed to save home content:', e);
      message = 'Failed to save home page text';
    } finally {
      isSaving = false;
    }
  }

  function resetToFallbacks() {
    content = {
      title: null,
      kicker: '',
      lead: '',
      heroFigurineId: null,
      heroCaptionTitle: '',
      heroCaptionMeta: '',
      heroCaptionCta: '',
      heroMode: 'auto',
      vitrineFigurineId: null,
    };
  }
</script>

<section class="h-full overflow-y-auto bg-[#fff9f0]/50 border border-[#34251c]/10">
  <div class="p-8 max-w-4xl">
    <div class="mb-8 flex items-start justify-between gap-6">
      <div>
        <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Homepage</p>
        <h2 class="font-gothic text-3xl text-[#6f3b24]">Home hero section</h2>
        <p class="mt-3 text-sm leading-7 text-[#5f4636] max-w-2xl">
          This links the hero photo, caption, and the work the user will open. That keeps the image and link in sync.
        </p>
      </div>

      {#if hasUnsaved}
        <span class="shrink-0 px-3 py-1.5 text-[10px] uppercase tracking-[0.1em] border border-amber-700/30 bg-amber-50 text-amber-800">
          Unsaved changes
        </span>
      {/if}
    </div>

    {#if isLoading}
      <p class="text-xs uppercase tracking-[0.12em] text-[#5f4636] animate-pulse">Loading…</p>
    {:else}
      <div class="space-y-6">
        <div class="border border-[#34251c]/10 bg-[#f8f1e7]/70 p-6 space-y-4">
          <div>
            <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Hero mode</p>
            <h3 class="font-gothic text-2xl text-[#6f3b24]">Hero mode</h3>
            <p class="mt-2 text-sm leading-6 text-[#5f4636] max-w-2xl">
              Showcase leads to available works. Rare release is better when there are no available works and you need to point to the archive or works in progress.
            </p>
          </div>

          <div class="mode-grid">
            <label class:active={content.heroMode === 'auto'}>
              <input type="radio" bind:group={content.heroMode} value="auto" />
              <span>Auto</span>
              <small>Switches to rare release when nothing is available</small>
            </label>
            <label class:active={content.heroMode === 'showcase'}>
              <input type="radio" bind:group={content.heroMode} value="showcase" />
              <span>Showcase</span>
              <small>Primary CTA leads to available works</small>
            </label>
            <label class:active={content.heroMode === 'release'}>
              <input type="radio" bind:group={content.heroMode} value="release" />
              <span>Rare release</span>
              <small>Primary CTA leads to the archive, secondary to works in progress</small>
            </label>
          </div>
        </div>

        <label class="block">
          <span class="label">Kicker</span>
          <input
            bind:value={content.kicker}
            class="input-gothic"
            placeholder={$t('homeKicker')}
          />
        </label>

        <label class="block">
          <span class="label">Lead</span>
          <textarea
            bind:value={content.lead}
            class="input-gothic h-32"
            placeholder={$t('homeLead')}
          ></textarea>
        </label>

        <div class="border border-[#34251c]/10 bg-[#f8f1e7]/70 p-6 space-y-5">
          <div class="flex items-start justify-between gap-4">
            <div>
              <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Hero work</p>
              <h3 class="font-gothic text-2xl text-[#6f3b24]">Work opened by the caption</h3>
              <p class="mt-2 text-sm leading-6 text-[#5f4636] max-w-2xl">
                Choose the figure matching the hero photo. The home caption will link to this exact work.
              </p>
            </div>
            {#if selectedHeroFigurine}
              <span class="shrink-0 px-3 py-1.5 text-[10px] uppercase tracking-[0.1em] border border-emerald-700/25 bg-emerald-50 text-emerald-800">
                Linked
              </span>
            {:else}
              <span class="shrink-0 px-3 py-1.5 text-[10px] uppercase tracking-[0.1em] border border-amber-700/30 bg-amber-50 text-amber-800">
                Not selected
              </span>
            {/if}
          </div>

          <label class="block">
            <span class="label">Hero figure</span>
            <select
              class="input-gothic"
              value={content.heroFigurineId ?? ''}
              onchange={(e) => selectHeroFigurine((e.currentTarget as HTMLSelectElement).value)}
            >
              <option value="">Do not link to a specific work</option>
              {#each figurines as fig}
                <option value={fig.id}>
                  {fig.name} · {fig.status}{fig.year ? ` · ${fig.year}` : ''}
                </option>
              {/each}
            </select>
          </label>

          {#if selectedHeroFigurine}
            <div class="selected-work">
              {#if selectedHeroFigurine.faceImageUrl}
                <img src={selectedHeroFigurine.faceImageUrl} alt={selectedHeroFigurine.name} />
              {/if}
              <div>
                <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-1">Will open</p>
                <p class="font-gothic text-2xl text-[#2c1710]">{selectedHeroFigurine.name}</p>
                <p class="mt-1 text-xs uppercase tracking-[0.12em] text-[#5f4636]">
                  /figurines/{selectedHeroFigurine.id}
                </p>
              </div>
              <div class="ml-auto flex gap-2">
                <button type="button" onclick={useSelectedName} class="btn-small">Name to caption</button>
                <button type="button" onclick={clearHeroFigurine} class="btn-small">Clear</button>
              </div>
            </div>
          {:else}
            <p class="warning">
              If no work is selected, the caption will not link to a random featured figure. A click opens the archive.
            </p>
          {/if}
        </div>

        <div class="border border-[#34251c]/10 bg-[#f8f1e7]/70 p-6 space-y-5">
          <div class="flex items-start justify-between gap-4">
            <div>
              <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Vitrine</p>
              <h3 class="font-gothic text-2xl text-[#6f3b24]">"Exhibit of the day" pin</h3>
              <p class="mt-2 text-sm leading-6 text-[#5f4636] max-w-2xl">
                Pin a specific work to the home vitrine, independent of the hero figure above. Leave unset to let it rotate daily. If the pinned work is currently sealed behind its showing window, the vitrine falls back to rotation on its own.
              </p>
            </div>
            {#if selectedVitrineFigurine}
              <span class="shrink-0 px-3 py-1.5 text-[10px] uppercase tracking-[0.1em] border border-emerald-700/25 bg-emerald-50 text-emerald-800">
                Pinned
              </span>
            {:else}
              <span class="shrink-0 px-3 py-1.5 text-[10px] uppercase tracking-[0.1em] border border-amber-700/30 bg-amber-50 text-amber-800">
                Daily rotation
              </span>
            {/if}
          </div>

          <div>
            <span class="label">Vitrine figure</span>
            <div class="relative mb-3">
              <input
                bind:value={vitrineSearch}
                type="text"
                placeholder="Search works by name…"
                class="input-gothic pr-8"
              />
              {#if vitrineSearch}
                <button
                  type="button"
                  onclick={() => (vitrineSearch = '')}
                  class="absolute right-2.5 top-1/2 -translate-y-1/2 text-[#5f4636] hover:text-[#34251c] text-xs"
                >✕</button>
              {/if}
            </div>

            <div class="picker-grid">
              <button
                type="button"
                class="picker-tile picker-tile--rotate"
                class:is-selected={!content.vitrineFigurineId}
                onclick={clearVitrineFigurine}
              >
                <span class="picker-tile-rotate-icon" aria-hidden="true">↻</span>
                <span class="picker-tile-name">Rotate daily</span>
              </button>

              {#each filteredVitrineFigurines as fig (fig.id)}
                <button
                  type="button"
                  class="picker-tile"
                  class:is-selected={content.vitrineFigurineId === fig.id}
                  onclick={() => selectVitrineFigurine(fig.id)}
                >
                  {#if fig.faceImageUrl}
                    <img src={fig.faceImageUrl} alt="" class="picker-tile-img" />
                  {:else}
                    <span class="picker-tile-img picker-tile-img--empty"></span>
                  {/if}
                  <span class="picker-tile-name" title={fig.name}>{fig.name}</span>
                  <span class="picker-tile-meta">{fig.status}{fig.year ? ` · ${fig.year}` : ''}</span>
                </button>
              {/each}

              {#if filteredVitrineFigurines.length === 0}
                <p class="col-span-full text-center text-xs italic text-[#5f4636] py-6">No works match "{vitrineSearch}"</p>
              {/if}
            </div>
          </div>

          {#if selectedVitrineFigurine}
            <div class="selected-work">
              {#if selectedVitrineFigurine.faceImageUrl}
                <img src={selectedVitrineFigurine.faceImageUrl} alt={selectedVitrineFigurine.name} />
              {/if}
              <div>
                <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-1">Under glass</p>
                <p class="font-gothic text-2xl text-[#2c1710]">{selectedVitrineFigurine.name}</p>
                <p class="mt-1 text-xs uppercase tracking-[0.12em] text-[#5f4636]">
                  /figurines/{selectedVitrineFigurine.id}
                </p>
              </div>
              <div class="ml-auto flex gap-2">
                <button type="button" onclick={clearVitrineFigurine} class="btn-small">Clear</button>
              </div>
            </div>
          {/if}
        </div>

        <div class="border border-[#34251c]/10 bg-[#fff9f0]/70 p-6 space-y-5">
          <div class="flex items-start justify-between gap-4">
            <div>
              <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Hero image</p>
              <h3 class="font-gothic text-2xl text-[#6f3b24]">Hero photo</h3>
              <p class="mt-2 text-sm leading-6 text-[#5f4636] max-w-2xl">
                This is the same photo used on the home page. It should visually match the selected work.
              </p>
            </div>
            <button type="button" onclick={changeBackground} disabled={isUploadingBg} class="btn-gothic disabled:opacity-40">
              {isUploadingBg ? 'Uploading…' : 'Replace photo'}
            </button>
          </div>

          {#if bgImage}
            <div class="hero-image-preview">
              <img src={bgImage} alt="Hero" />
            </div>
          {:else}
            <p class="warning">No hero photo uploaded. A fallback will be used.</p>
          {/if}
        </div>

        <div class="border border-[#34251c]/10 bg-[#f8f1e7]/70 p-6 space-y-5">
          <div>
            <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Hero caption</p>
            <h3 class="font-gothic text-2xl text-[#6f3b24]">Photo caption</h3>
          </div>

          <label class="block">
            <span class="label">Caption title</span>
            <input
              bind:value={content.heroCaptionTitle}
              class="input-gothic"
              placeholder={selectedHeroFigurine?.name || 'The Fool'}
            />
          </label>

          <label class="block">
            <span class="label">Meta line</span>
            <input
              bind:value={content.heroCaptionMeta}
              class="input-gothic"
              placeholder={$t('homeHeroObjectMeta')}
            />
          </label>

          <label class="block">
            <span class="label">Link text</span>
            <input
              bind:value={content.heroCaptionCta}
              class="input-gothic"
              placeholder={$t('homeHeroObjectOpen')}
            />
          </label>
        </div>

        <div class="border border-[#34251c]/10 bg-[#f8f1e7]/70 p-6">
          <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-3">Preview</p>
          <p class="text-xs uppercase tracking-[0.18em] text-[#6f3b24] mb-3">
            {content.kicker?.trim() || $t('homeKicker')}
          </p>
          <h3 class="font-gothic text-5xl text-[#2c1710] leading-none mb-4">
            {$t('homeTitle')}
          </h3>
          <p class="font-serif text-xl leading-8 text-[#5f4636] max-w-2xl">
            {content.lead?.trim() || $t('homeLead')}
          </p>
          <div class="mt-6 border-l border-[#c65f3c]/50 pl-4">
            <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636]">Photo caption</p>
            <p class="font-gothic text-3xl italic text-[#2c1710]">
              {content.heroCaptionTitle?.trim() || selectedHeroFigurine?.name || '—'}
            </p>
            <p class="text-[10px] uppercase tracking-[0.12em] text-[#5f4636]">
              {content.heroCaptionMeta?.trim() || $t('homeHeroObjectMeta')}
            </p>
            <p class="mt-2 text-[10px] uppercase tracking-[0.12em] text-[#6f3b24]">
              {content.heroCaptionCta?.trim() || (selectedHeroFigurine ? $t('homeHeroObjectOpen') : $t('homeSecondaryCta'))} →
            </p>
          </div>
        </div>

        {#if message}
          <p class="text-xs uppercase tracking-[0.1em] text-[#5f4636]">{message}</p>
        {/if}

        <div class="flex gap-3">
          <button
            onclick={save}
            disabled={isSaving || !hasUnsaved}
            class="btn-gothic disabled:opacity-40"
          >
            {isSaving ? 'Saving…' : 'Save home page'}
          </button>
          <button
            onclick={resetToFallbacks}
            disabled={isSaving}
            class="btn-gothic opacity-70"
          >
            Reset texts to default
          </button>
        </div>
      </div>
    {/if}
  </div>
</section>

<style>
  .label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #5f4636;
    margin-bottom: 0.35rem;
    display: block;
    font-weight: 700;
  }

  .input-gothic {
    width: 100%;
    background-color: #f8f1e7;
    border: 1px solid rgba(52, 37, 28, 0.18);
    padding: 0.65rem 0.75rem;
    color: #34251c;
    outline: none;
    transition: border-color 0.2s, background-color 0.2s;
    font-size: 0.875rem;
    line-height: 1.6;
  }

  .input-gothic:focus {
    border-color: rgba(198, 95, 60, 0.5);
    background-color: #fff9f0;
  }

  textarea.input-gothic {
    resize: vertical;
  }

  .selected-work {
    display: flex;
    align-items: center;
    gap: 1rem;
    border: 1px solid rgba(52, 37, 28, 0.12);
    background: rgba(255, 249, 240, 0.72);
    padding: 0.75rem;
  }

  .selected-work img {
    width: 4.5rem;
    height: 4.5rem;
    object-fit: cover;
    border: 1px solid rgba(52, 37, 28, 0.12);
  }

  .picker-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(6.5rem, 1fr));
    gap: 0.6rem;
    max-height: 19rem;
    overflow-y: auto;
    padding: 0.15rem;
  }

  .picker-tile {
    display: grid;
    justify-items: center;
    gap: 0.3rem;
    padding: 0.5rem;
    border: 1px solid rgba(52, 37, 28, 0.14);
    background: rgba(255, 249, 240, 0.55);
    cursor: pointer;
    text-align: center;
    transition: border-color 0.18s, background-color 0.18s;
  }

  .picker-tile:hover {
    border-color: rgba(198, 95, 60, 0.4);
    background: rgba(255, 249, 240, 0.9);
  }

  .picker-tile.is-selected {
    border-color: rgba(198, 95, 60, 0.75);
    background: rgba(255, 249, 240, 0.98);
    box-shadow: 0 0 0 1px rgba(198, 95, 60, 0.35);
  }

  .picker-tile-img {
    width: 4.5rem;
    height: 4.5rem;
    object-fit: cover;
    border: 1px solid rgba(52, 37, 28, 0.12);
  }

  .picker-tile-img--empty {
    background: repeating-linear-gradient(
      45deg,
      rgba(52, 37, 28, 0.06),
      rgba(52, 37, 28, 0.06) 4px,
      transparent 4px,
      transparent 8px
    );
  }

  .picker-tile-rotate-icon {
    display: grid;
    place-items: center;
    width: 4.5rem;
    height: 4.5rem;
    font-size: 1.6rem;
    color: #6f3b24;
    border: 1px dashed rgba(111, 59, 36, 0.4);
    background: rgba(111, 59, 36, 0.05);
  }

  .picker-tile-name {
    font-size: 0.72rem;
    font-weight: 600;
    color: #34251c;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .picker-tile-meta {
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #5f4636;
  }

  .hero-image-preview {
    position: relative;
    overflow: hidden;
    aspect-ratio: 16 / 7;
    border: 1px solid rgba(52, 37, 28, 0.12);
    background: #f8f1e7;
  }

  .hero-image-preview img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .warning {
    border-left: 1px solid rgba(198, 95, 60, 0.5);
    padding-left: 0.85rem;
    color: #5f4636;
    font-size: 0.82rem;
    line-height: 1.7;
  }

  .btn-small {
    border: 1px solid rgba(52, 37, 28, 0.18);
    background: #f8f1e7;
    color: #34251c;
    padding: 0.45rem 0.65rem;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    transition: border-color 0.2s, background-color 0.2s;
  }

  .btn-small:hover {
    border-color: rgba(198, 95, 60, 0.45);
    background: #fff9f0;
  }

  .mode-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .mode-grid label {
    display: grid;
    gap: 0.45rem;
    align-content: start;
    min-height: 7rem;
    border: 1px solid rgba(52, 37, 28, 0.14);
    background: rgba(255, 249, 240, 0.58);
    padding: 0.85rem;
    color: #5f4636;
    cursor: pointer;
    transition: border-color 0.2s, background-color 0.2s;
  }

  .mode-grid label.active {
    border-color: rgba(198, 95, 60, 0.5);
    background: rgba(255, 249, 240, 0.95);
  }

  .mode-grid input {
    width: 0.85rem;
    height: 0.85rem;
    accent-color: #6f3b24;
  }

  .mode-grid span {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: #34251c;
    font-weight: 700;
  }

  .mode-grid small {
    font-size: 0.78rem;
    line-height: 1.55;
    color: #5f4636;
  }

  @media (max-width: 780px) {
    .mode-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
