<script lang="ts">
  import { onMount } from 'svelte';
  import { api, resolveMediaUrl } from '$lib/api';
  import { t } from '$lib/i18n';
  import type { HeaderDecor, HeaderDecorSide } from '$lib/types/api';

  type Side = 'left' | 'right';

  const DEFAULT_WIDTH = 260;
  const DEFAULT_FADE = 35;

  let decor = $state<HeaderDecor>({ left: {}, right: {} });
  let loading = $state(true);
  let saving = $state(false);
  let uploading = $state<Side | null>(null);
  let dirty = $state(false);
  let message = $state('');
  let messageType = $state<'ok' | 'err'>('ok');

  onMount(async () => {
    try {
      const d = await api.getHeaderDecor();
      decor = { left: { ...d.left }, right: { ...d.right } };
    } catch {
      // пусто — ничего не стоит
    }
    loading = false;
  });

  function edit(side: Side, patch: Partial<HeaderDecorSide>) {
    decor[side] = { ...decor[side], ...patch };
    dirty = true;
    message = '';
  }

  function pick(side: Side) {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'image/png,image/webp,image/jpeg';
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      uploading = side;
      message = '';
      try {
        const art = await api.adminUploadBattleFrameArt(file);
        edit(side, { image: art.path });
      } catch {
        messageType = 'err';
        message = $t('adminHeaderDecorUploadFailed');
      } finally {
        uploading = null;
      }
    };
    input.click();
  }

  async function save() {
    saving = true;
    message = '';
    try {
      const saved = await api.saveHeaderDecor($state.snapshot(decor) as HeaderDecor);
      decor = { left: { ...saved.left }, right: { ...saved.right } };
      dirty = false;
      messageType = 'ok';
      message = $t('adminHeaderDecorSaved');
    } catch {
      messageType = 'err';
      message = $t('adminHeaderDecorSaveFailed');
    } finally {
      saving = false;
    }
  }

  function maskFor(side: Side, fade: number): string {
    const dir = side === 'left' ? 'to right' : 'to left';
    return `linear-gradient(${dir}, #000 calc(100% - ${fade}%), transparent)`;
  }
</script>

<div class="h-full overflow-y-auto p-6">
  <div class="max-w-3xl mx-auto space-y-6">
    <div>
      <h2 class="font-display text-lg text-[#34251c]">{$t('adminHeaderDecorTitle')}</h2>
      <p class="text-[11px] text-[#5f4636] mt-1">{$t('adminHeaderDecorDesc')}</p>
    </div>

    {#if loading}
      <p class="text-[11px] text-[#5f4636]">…</p>
    {:else}
      {#each ['left', 'right'] as const as side (side)}
        {@const cur = decor[side]}
        {@const url = resolveMediaUrl(cur.image?.trim())}
        {@const width = cur.width ?? DEFAULT_WIDTH}
        {@const fade = cur.fade ?? DEFAULT_FADE}
        <section class="border border-[#34251c]/10 p-4 space-y-4">
          <p class="text-[10px] tracking-[0.08em] uppercase text-[#5f4636]">
            {side === 'left' ? $t('adminHeaderDecorLeft') : $t('adminHeaderDecorRight')}
          </p>

          <!-- Предпросмотр: кусок шапки на той высоте, что видит посетитель -->
          <div
            class="relative h-[78px] border border-[#34251c]/10 overflow-hidden"
            style="background: rgba(248, 241, 231, 0.97);"
          >
            {#if url}
              <div
                class="absolute top-0 bottom-0 {side === 'left' ? 'left-0' : 'right-0'}"
                style="
                  width: {width}px;
                  max-width: 100%;
                  background: url('{url}') no-repeat {side} center / cover;
                  -webkit-mask-image: {maskFor(side, fade)};
                  mask-image: {maskFor(side, fade)};
                "
              ></div>
            {:else}
              <span class="absolute inset-0 flex items-center justify-center text-[10px] text-[#5f4636]/60">
                {$t('adminHeaderDecorEmpty')}
              </span>
            {/if}
          </div>

          <div class="flex flex-wrap items-center gap-2">
            <button
              type="button"
              onclick={() => pick(side)}
              disabled={uploading === side}
              class="btn-gothic text-[10px] px-3 py-1.5 disabled:opacity-60"
            >
              {uploading === side ? '…' : url ? $t('adminChangePhoto') : $t('adminUploadPhoto')}
            </button>
            {#if url}
              <button
                type="button"
                onclick={() => edit(side, { image: null })}
                class="text-[10px] uppercase tracking-wide text-[#5f4636] hover:text-[#34251c] px-2 py-1.5"
              >{$t('adminHeaderDecorRemove')}</button>
            {/if}
          </div>

          {#if url}
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label for="decor-{side}-width" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">
                  {$t('adminHeaderDecorWidth')}: {width}px
                </label>
                <input
                  id="decor-{side}-width"
                  type="range"
                  min="120"
                  max="640"
                  step="10"
                  value={width}
                  oninput={(e) => edit(side, { width: Number(e.currentTarget.value) })}
                  class="w-full"
                />
              </div>
              <div>
                <label for="decor-{side}-fade" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">
                  {$t('adminHeaderDecorFade')}: {fade}%
                </label>
                <input
                  id="decor-{side}-fade"
                  type="range"
                  min="0"
                  max="100"
                  step="5"
                  value={fade}
                  oninput={(e) => edit(side, { fade: Number(e.currentTarget.value) })}
                  class="w-full"
                />
              </div>
            </div>
          {/if}
        </section>
      {/each}

      <p class="text-[10px] text-[#5f4636]/80">{$t('adminHeaderDecorHint')}</p>

      <div class="flex items-center gap-3">
        <button
          type="button"
          onclick={save}
          disabled={saving || !dirty}
          class="btn-gothic text-[10px] px-4 py-2 disabled:opacity-50"
        >{saving ? '…' : $t('adminHeaderDecorSave')}</button>
        {#if message}
          <span class="text-[11px] {messageType === 'ok' ? 'text-[#5f4636]' : 'text-[#9b2c1c]'}">{message}</span>
        {/if}
      </div>
    {/if}
  </div>
</div>
