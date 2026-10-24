<script lang="ts">
  import { onMount } from 'svelte';
  import { api, resolveMediaUrl } from '$lib/api';
  import { t, type TranslationKey } from '$lib/i18n';
  import type { NavSealImage, NavSealKey, NavSealSection, NavSeals } from '$lib/types/api';

  // Картинки на печатях меню: для каждого раздела — одна или серия. Сервер сам
  // уменьшает каждую загруженную картинку до двух размеров, так что здесь только
  // выбор, порядок, форма и срок смены. Раздел без картинок показывает то, что
  // сайт рисует сам.

  const KEYS: NavSealKey[] = ['works', 'gazette', 'tales', 'game', 'studio'];
  const LABEL: Record<NavSealKey, TranslationKey> = {
    works: 'navAllWorks',
    gazette: 'navGazette',
    tales: 'navTales',
    game: 'navGame',
    studio: 'navStudio',
  };
  const SHAPES: { id: NavSealSection['shape']; label: TranslationKey }[] = [
    { id: 'circle', label: 'adminSealsShapeCircle' },
    { id: 'free', label: 'adminSealsShapeFree' },
    { id: 'rect', label: 'adminSealsShapeRect' },
  ];

  function blank(): NavSealSection {
    return { images: [], intervalSec: 20, shape: 'free' };
  }

  let data = $state<Record<NavSealKey, NavSealSection>>({
    works: blank(),
    gazette: blank(),
    tales: blank(),
    game: blank(),
    studio: blank(),
  });
  let saved = $state('');
  let loading = $state(true);
  let saving = $state(false);
  let uploading = $state<NavSealKey | null>(null);
  let message = $state('');
  let messageType = $state<'ok' | 'err'>('ok');

  function payload(): NavSeals {
    const sections: NavSeals['sections'] = {};
    for (const k of KEYS) {
      const sec = $state.snapshot(data[k]) as NavSealSection;
      if (sec.images.length > 0) sections[k] = sec;
    }
    return { sections };
  }

  let dirty = $derived(!loading && JSON.stringify(payload()) !== saved);

  function say(text: string, type: 'ok' | 'err' = 'ok') {
    message = text;
    messageType = type;
  }

  function adopt(res: NavSeals) {
    for (const k of KEYS) data[k] = res.sections[k] ? { ...blank(), ...res.sections[k] } : blank();
    saved = JSON.stringify(payload());
  }

  onMount(async () => {
    try {
      adopt(await api.getNavSeals());
    } catch {
      say($t('adminSealsFailed'), 'err');
    } finally {
      loading = false;
    }
  });

  function add(key: NavSealKey) {
    const input = document.createElement('input');
    input.type = 'file';
    input.multiple = true;
    input.accept = 'image/png,image/webp,image/jpeg';
    input.onchange = async () => {
      const files = [...(input.files ?? [])];
      if (files.length === 0) return;
      uploading = key;
      message = '';
      try {
        for (const file of files) {
          const img: NavSealImage = await api.adminUploadNavSeal(file);
          data[key].images = [...data[key].images, img];
        }
      } catch {
        say($t('adminSealsUploadFailed'), 'err');
      } finally {
        uploading = null;
      }
    };
    input.click();
  }

  function remove(key: NavSealKey, id: string) {
    data[key].images = data[key].images.filter((i) => i.id !== id);
  }

  function move(key: NavSealKey, at: number, by: -1 | 1) {
    const list = [...data[key].images];
    const to = at + by;
    if (to < 0 || to >= list.length) return;
    [list[at], list[to]] = [list[to], list[at]];
    data[key].images = list;
  }

  async function save() {
    saving = true;
    message = '';
    try {
      adopt(await api.saveNavSeals(payload()));
      say($t('adminSealsSaved'));
    } catch {
      say($t('adminSealsFailed'), 'err');
    } finally {
      saving = false;
    }
  }
</script>

<div class="h-full overflow-auto p-4 space-y-4">
  <p class="text-[11px] text-[#5f4636] max-w-[60ch]">{$t('adminSealsHint')}</p>

  {#if loading}
    <p class="text-[11px] text-[#5f4636]">…</p>
  {:else}
    {#each KEYS as key (key)}
      {@const sec = data[key]}
      <section class="border border-[#34251c]/10 p-4 space-y-3">
        <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
          <p class="text-[10px] tracking-[0.08em] uppercase text-[#34251c]">{$t(LABEL[key])}</p>
          <button
            type="button"
            onclick={() => add(key)}
            disabled={uploading !== null}
            class="text-[10px] uppercase tracking-wide border border-[#34251c]/30 px-3 py-1.5 text-[#34251c] hover:bg-[#34251c]/5 disabled:opacity-50"
          >{uploading === key ? $t('adminSealsUploading') : $t('adminSealsAdd')}</button>
        </div>

        {#if sec.images.length === 0}
          <p class="text-[10px] text-[#5f4636]/70">{$t('adminSealsEmpty')}</p>
        {:else}
          <ul class="flex flex-wrap gap-3">
            {#each sec.images as img, i (img.id)}
              <li class="w-[104px] space-y-1">
                <div
                  class="h-[104px] flex items-center justify-center border border-[#34251c]/10 bg-[#f8f1e7]"
                  style="background-image: conic-gradient(#0000000a 25%, transparent 0 50%, #0000000a 0 75%, transparent 0); background-size: 12px 12px;"
                >
                  <img
                    src={resolveMediaUrl(img.url) ?? img.url}
                    alt=""
                    class="max-w-full max-h-full object-contain"
                    width={img.width}
                    height={img.height}
                  />
                </div>
                <div class="flex items-center justify-between gap-1">
                  <button
                    type="button"
                    onclick={() => move(key, i, -1)}
                    disabled={i === 0}
                    title={$t('adminSealsEarlier')}
                    aria-label={$t('adminSealsEarlier')}
                    class="px-1.5 py-0.5 text-[12px] text-[#5f4636] hover:text-[#34251c] disabled:opacity-30"
                  >←</button>
                  <button
                    type="button"
                    onclick={() => remove(key, img.id)}
                    class="text-[10px] uppercase tracking-wide text-[#9b2c1c] hover:underline"
                  >{$t('adminSealsRemove')}</button>
                  <button
                    type="button"
                    onclick={() => move(key, i, 1)}
                    disabled={i === sec.images.length - 1}
                    title={$t('adminSealsLater')}
                    aria-label={$t('adminSealsLater')}
                    class="px-1.5 py-0.5 text-[12px] text-[#5f4636] hover:text-[#34251c] disabled:opacity-30"
                  >→</button>
                </div>
              </li>
            {/each}
          </ul>

          <div class="flex flex-wrap items-center gap-x-6 gap-y-2">
            <label class="flex items-center gap-2 text-[11px] text-[#5f4636]">
              {$t('adminSealsShape')}
              <select
                bind:value={sec.shape}
                class="border border-[#34251c]/20 bg-transparent px-2 py-1 text-[11px] text-[#34251c]"
              >
                {#each SHAPES as s (s.id)}
                  <option value={s.id}>{$t(s.label)}</option>
                {/each}
              </select>
            </label>
            {#if sec.images.length > 1}
              <label class="flex items-center gap-2 text-[11px] text-[#5f4636]">
                {$t('adminSealsInterval')}
                <input
                  type="number"
                  min="5"
                  max="3600"
                  step="1"
                  bind:value={sec.intervalSec}
                  class="w-20 border border-[#34251c]/20 bg-transparent px-2 py-1 text-[11px] text-[#34251c]"
                />
              </label>
            {:else}
              <span class="text-[10px] text-[#5f4636]/70">{$t('adminSealsSingle')}</span>
            {/if}
          </div>
        {/if}
      </section>
    {/each}

    <div class="flex items-center gap-3 pb-6">
      <button
        type="button"
        onclick={save}
        disabled={saving || !dirty}
        class="text-[10px] uppercase tracking-wide bg-[#34251c] text-[#f8f1e7] px-4 py-2 disabled:opacity-40"
      >{saving ? $t('adminSealsSaving') : $t('adminSealsSave')}</button>
      {#if dirty && !saving}
        <span class="text-[11px] text-[#5f4636]">{$t('adminSealsUnsaved')}</span>
      {/if}
      {#if message}
        <span class="text-[11px] {messageType === 'ok' ? 'text-[#5f4636]' : 'text-[#9b2c1c]'}">{message}</span>
      {/if}
    </div>
  {/if}
</div>
