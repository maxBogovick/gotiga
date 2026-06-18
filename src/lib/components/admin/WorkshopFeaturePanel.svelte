<script lang="ts">
  import { onMount } from 'svelte';
  import { api, isTauri } from '$lib/api';
  import type { WorkshopFeature } from '$lib/types/api';
  import { t } from '$lib/i18n';

  const EMPTY: WorkshopFeature = {
    visible: true,
    photoBack: null,
    photoFront: null,
    eyebrowEn: null,
    eyebrowRu: null,
    titleEn: null,
    titleRu: null,
    textEn: null,
    textRu: null,
    link1LabelEn: null,
    link1LabelRu: null,
    link1Href: null,
    link2LabelEn: null,
    link2LabelRu: null,
    link2Href: null,
  };

  let feature = $state<WorkshopFeature>({ ...EMPTY });
  let savedSnapshot = $state('');
  let isLoading = $state(true);
  let isSaving = $state(false);
  let uploading = $state<'back' | 'front' | null>(null);
  let message = $state('');

  let hasUnsaved = $derived(JSON.stringify(feature) !== savedSnapshot);

  const photoBackUrl = $derived(feature.photoBack?.trim() || '/images/workshop/master-1.jpg');
  const photoFrontUrl = $derived(feature.photoFront?.trim() || '/images/workshop/master-2.jpg');

  onMount(async () => {
    try {
      const loaded = await api.getWorkshopFeature().catch(() => null);
      if (loaded) feature = { ...EMPTY, ...loaded };
      savedSnapshot = JSON.stringify(feature);
    } finally {
      isLoading = false;
    }
  });

  async function pickFile(): Promise<File | string | null> {
    if (isTauri) {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({ multiple: false, filters: [{ name: 'Images', extensions: ['jpg', 'png', 'webp'] }] });
      return typeof selected === 'string' ? selected : null;
    }
    return new Promise<File | null>((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = 'image/jpeg,image/png,image/webp';
      input.onchange = () => resolve(input.files?.[0] ?? null);
      input.click();
    });
  }

  async function replacePhoto(slot: 'back' | 'front') {
    uploading = slot;
    message = '';
    try {
      const fileOrPath = await pickFile();
      if (!fileOrPath) return;
      const url = await api.importMedia(fileOrPath, 'images');
      if (slot === 'back') feature.photoBack = url;
      else feature.photoFront = url;
    } catch (e) {
      console.error('Failed to upload workshop photo:', e);
      message = 'Failed to upload photo';
    } finally {
      uploading = null;
    }
  }

  function clearPhoto(slot: 'back' | 'front') {
    if (slot === 'back') feature.photoBack = null;
    else feature.photoFront = null;
  }

  function clean(v: string | null): string | null {
    const t = (v ?? '').trim();
    return t.length ? t : null;
  }

  async function save() {
    isSaving = true;
    message = '';
    try {
      const payload: WorkshopFeature = {
        visible: feature.visible,
        photoBack: clean(feature.photoBack),
        photoFront: clean(feature.photoFront),
        eyebrowEn: clean(feature.eyebrowEn),
        eyebrowRu: clean(feature.eyebrowRu),
        titleEn: clean(feature.titleEn),
        titleRu: clean(feature.titleRu),
        textEn: clean(feature.textEn),
        textRu: clean(feature.textRu),
        link1LabelEn: clean(feature.link1LabelEn),
        link1LabelRu: clean(feature.link1LabelRu),
        link1Href: clean(feature.link1Href),
        link2LabelEn: clean(feature.link2LabelEn),
        link2LabelRu: clean(feature.link2LabelRu),
        link2Href: clean(feature.link2Href),
      };
      await api.saveWorkshopFeature(payload);
      feature = payload;
      savedSnapshot = JSON.stringify(feature);
      message = 'Workshop section updated';
    } catch (e) {
      console.error('Failed to save workshop feature:', e);
      message = 'Failed to save workshop section';
    } finally {
      isSaving = false;
    }
  }

  function resetToDefaults() {
    feature = { ...EMPTY };
  }
</script>

<section class="h-full overflow-y-auto bg-[#fff9f0]/50 border border-[#34251c]/10">
  <div class="p-8 max-w-4xl">
    <div class="mb-8 flex items-start justify-between gap-6">
      <div>
        <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Homepage</p>
        <h2 class="font-gothic text-3xl text-[#6f3b24]">Workshop section</h2>
        <p class="mt-3 text-sm leading-7 text-[#5f4636] max-w-2xl">
          The two-photo "Workshop" block on the home page. Photos, texts, and the two links are all
          editable here. Leave a text field blank to use the built-in default.
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
        <!-- Visibility -->
        <label class="flex items-center gap-3 border border-[#34251c]/10 bg-[#f8f1e7]/70 p-5 cursor-pointer">
          <input type="checkbox" bind:checked={feature.visible} class="w-4 h-4 accent-[#6f3b24]" />
          <span>
            <span class="block text-sm font-semibold text-[#34251c]">Show this section on the home page</span>
            <span class="block text-xs text-[#5f4636] mt-0.5">Uncheck to hide the entire Workshop block.</span>
          </span>
        </label>

        <!-- Photos -->
        <div class="border border-[#34251c]/10 bg-[#fff9f0]/70 p-6 space-y-5">
          <div>
            <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Photos</p>
            <h3 class="font-gothic text-2xl text-[#6f3b24]">Two photos</h3>
            <p class="mt-2 text-sm leading-6 text-[#5f4636] max-w-2xl">
              The back photo sits behind, the front photo overlaps it. Both fall back to the bundled images when empty.
            </p>
          </div>

          <div class="photo-grid">
            {#each [{ slot: 'back', label: 'Back photo', url: photoBackUrl, custom: feature.photoBack }, { slot: 'front', label: 'Front photo', url: photoFrontUrl, custom: feature.photoFront }] as p}
              <div class="photo-card">
                <p class="label">{p.label}</p>
                <div class="photo-preview">
                  <img src={p.url} alt={p.label} />
                </div>
                <div class="flex gap-2">
                  <button type="button" onclick={() => replacePhoto(p.slot as 'back' | 'front')} disabled={uploading !== null} class="btn-small">
                    {uploading === p.slot ? 'Uploading…' : 'Replace'}
                  </button>
                  {#if p.custom}
                    <button type="button" onclick={() => clearPhoto(p.slot as 'back' | 'front')} class="btn-small">Use default</button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>

        <!-- Texts -->
        <div class="border border-[#34251c]/10 bg-[#f8f1e7]/70 p-6 space-y-5">
          <div>
            <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Texts</p>
            <h3 class="font-gothic text-2xl text-[#6f3b24]">Copy</h3>
          </div>

          <div class="lang-grid">
            <label class="block">
              <span class="label">Eyebrow · EN</span>
              <input bind:value={feature.eyebrowEn} class="input-gothic" placeholder={$t('homeWorkshopCta')} />
            </label>
            <label class="block">
              <span class="label">Eyebrow · RU</span>
              <input bind:value={feature.eyebrowRu} class="input-gothic" placeholder="Мастерская" />
            </label>

            <label class="block">
              <span class="label">Title · EN</span>
              <input bind:value={feature.titleEn} class="input-gothic" placeholder={$t('homeStudioTitle')} />
            </label>
            <label class="block">
              <span class="label">Title · RU</span>
              <input bind:value={feature.titleRu} class="input-gothic" placeholder="Ручная работа, по одной фигурке за раз" />
            </label>

            <label class="block">
              <span class="label">Text · EN</span>
              <textarea bind:value={feature.textEn} class="input-gothic h-28" placeholder={$t('homeStudioText')}></textarea>
            </label>
            <label class="block">
              <span class="label">Text · RU</span>
              <textarea bind:value={feature.textRu} class="input-gothic h-28" placeholder="Каждая фигурка создаётся, расписывается и доводится автором вручную."></textarea>
            </label>
          </div>
        </div>

        <!-- Links -->
        <div class="border border-[#34251c]/10 bg-[#f8f1e7]/70 p-6 space-y-5">
          <div>
            <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Links</p>
            <h3 class="font-gothic text-2xl text-[#6f3b24]">Two buttons</h3>
            <p class="mt-2 text-sm leading-6 text-[#5f4636] max-w-2xl">
              Each button has a label per language and a destination. Use a path like <code>/workshop</code> or a full URL.
            </p>
          </div>

          <div class="lang-grid">
            <label class="block">
              <span class="label">Button 1 label · EN</span>
              <input bind:value={feature.link1LabelEn} class="input-gothic" placeholder={$t('homeWorkshopCta')} />
            </label>
            <label class="block">
              <span class="label">Button 1 label · RU</span>
              <input bind:value={feature.link1LabelRu} class="input-gothic" placeholder="Мастерская" />
            </label>
            <label class="block sm:col-span-2">
              <span class="label">Button 1 destination</span>
              <input bind:value={feature.link1Href} class="input-gothic" placeholder="/workshop" />
            </label>

            <label class="block">
              <span class="label">Button 2 label · EN</span>
              <input bind:value={feature.link2LabelEn} class="input-gothic" placeholder={$t('navAuthor')} />
            </label>
            <label class="block">
              <span class="label">Button 2 label · RU</span>
              <input bind:value={feature.link2LabelRu} class="input-gothic" placeholder="Автор" />
            </label>
            <label class="block sm:col-span-2">
              <span class="label">Button 2 destination</span>
              <input bind:value={feature.link2Href} class="input-gothic" placeholder="/author" />
            </label>
          </div>
        </div>

        {#if message}
          <p class="text-xs uppercase tracking-[0.1em] text-[#5f4636]">{message}</p>
        {/if}

        <div class="flex gap-3">
          <button onclick={save} disabled={isSaving || !hasUnsaved} class="btn-gothic disabled:opacity-40">
            {isSaving ? 'Saving…' : 'Save section'}
          </button>
          <button onclick={resetToDefaults} disabled={isSaving} class="btn-gothic opacity-70">
            Reset to defaults
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

  .lang-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1rem;
  }

  .photo-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1rem;
  }

  .photo-card {
    display: grid;
    gap: 0.6rem;
  }

  .photo-preview {
    position: relative;
    overflow: hidden;
    aspect-ratio: 3 / 4;
    border: 1px solid rgba(52, 37, 28, 0.12);
    background: #f8f1e7;
  }

  .photo-preview img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
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

  .btn-small:disabled {
    opacity: 0.4;
  }

  code {
    font-size: 0.78rem;
    background: rgba(52, 37, 28, 0.08);
    padding: 0.05rem 0.3rem;
  }

  @media (max-width: 640px) {
    .lang-grid,
    .photo-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
