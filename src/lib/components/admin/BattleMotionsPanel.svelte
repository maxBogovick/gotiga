<script lang="ts">
  // Стол движений ХОЗЯИНА: тот же стол, что встанет перед человеком в студии,
  // плюс домашний транспорт и домашний склад.
  //
  // Сам стол лежит в общем месте (`BattleMotionsDesk`) и не знает ни одного
  // адреса. Здесь — только адреса: чем грузить, чем сохранять и откуда брать
  // картинки. Второй такой же стол на странице человека был бы вторым словарём
  // жестов, а движок не знает слов, которых у него нет: разошлись бы они молча.
  import { api } from '$lib/api';
  import BattleMotionsDesk from '$lib/components/BattleMotionsDesk.svelte';
  import BattleAssetPicker from '$lib/components/admin/BattleAssetPicker.svelte';
  import { STRIP_FRAMES, punchStripGround } from '$lib/battles';
  import type { BattleSplitRect, Motion } from '$lib/types/api';

  let {
    flash,
    onSaved,
  }: {
    flash?: (text: string) => void;
    onSaved?: (motions: Motion[]) => void;
  } = $props();

  /** Кому отдать выбранную со склада картинку. Обещание, а не колбэк: стол
   *  ждёт адрес, и ждать его удобнее там, где его попросили. */
  let waiting = $state<((url: string | null) => void) | null>(null);

  function pickFromStore(): Promise<string | null> {
    return new Promise((resolve) => {
      waiting = resolve;
    });
  }

  /** Готовая полоса: кладём её на склад дома целиком и режем на шесть кадров
   *  тем же разрезом, что на вкладке ассетов. */
  async function keepStrip(file: File): Promise<string[]> {
    const punched = await punchStripGround(file);
    const stem = file.name.replace(/\.[^.]+$/, '') || 'strip';
    const parent = await api.adminAddBattleAsset(punched, stem, 'motion');
    const rects: BattleSplitRect[] = Array.from({ length: STRIP_FRAMES }, (_, i) => ({
      x: i / STRIP_FRAMES,
      y: 0,
      w: 1 / STRIP_FRAMES,
      h: 1,
      name: `${stem} ${i + 1}`,
      role: 'motion',
    }));
    const parts = await api.adminSplitBattleAsset(parent.id, rects);
    return [...parts]
      .sort((a, b) => {
        const ao = a.sortOrder ?? 0;
        const bo = b.sortOrder ?? 0;
        if (ao !== bo) return ao - bo;
        return a.name.localeCompare(b.name, undefined, { numeric: true });
      })
      .map((p) => p.url);
  }
</script>

<BattleMotionsDesk
  {flash}
  {onSaved}
  loadMotions={async () => (await api.getBattleMotions()).motions}
  saveMotions={async (motions) => (await api.adminSaveBattleMotions({ motions })).motions}
  loadCards={() => api.adminListBattleCards()}
  loadFrames={() => api.getBattleFrames()}
  loadRaces={() => api.getBattleRaces()}
  uploadArt={async (file) => (await api.adminUploadBattleFrameArt(file)).url}
  {keepStrip}
  {pickFromStore}
/>

{#if waiting}
  <BattleAssetPicker
    role="motion"
    onPick={(asset) => {
      waiting?.(asset.url);
      waiting = null;
    }}
    onClose={() => {
      waiting?.(null);
      waiting = null;
    }}
  />
{/if}
