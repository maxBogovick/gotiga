<script lang="ts">
  // Две полки в одном окне: библиотека дома и свой ящик.
  //
  // Порознь их показывать нельзя — человек, которому нужен уголок, ищет уголок,
  // а не вспоминает, откуда он взялся. Но и слить их в одну полку нельзя: у
  // домашней детали и своей разная судьба (одну не убрать, другую можно), и
  // подпись должна говорить, что перед тобой.
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import * as local from '$lib/studio/local';
  import { liveUrl } from '$lib/studio/live';
  import type { BattleAsset, StudioAsset } from '$lib/types/api';

  interface Props {
    /** С какой полки начинать: роль детали, которую взяли в руку. */
    role: string;
    onpick: (url: string) => void;
    onclose: () => void;
  }
  let { role, onpick, onclose }: Props = $props();

  let house = $state<BattleAsset[]>([]);
  let mine = $state<StudioAsset[]>([]);
  /** Свои картинки в этом браузере. Третья полка, и без неё склад бессмыслен:
   *  человек принёс картинку на верстак — и не может её надеть. */
  let bench = $state<{ key: string; name: string; url: string }[]>([]);
  let loading = $state(true);

  onMount(async () => {
    const token = authStore.token;
    const [lib, studio] = await Promise.all([
      api.getStudioLibrary(role).catch(() => []),
      token ? api.getStudio(token).then((s) => s.box.assets).catch(() => []) : Promise.resolve([]),
    ]);
    house = lib;
    // Своя полка не сужается ролью: деталей у человека десяток, и прятать от
    // него его же картинку ради стройности списка — значит заставить его
    // грузить её второй раз.
    mine = studio;
    const rows = await local.listAssets();
    const shown: { key: string; name: string; url: string }[] = [];
    for (const one of rows) {
      const url = await liveUrl(one.key);
      // Ключ, для которого блоба уже нет, не показывается: пустая плитка была
      // бы обещанием картинки, которой нет.
      if (url) shown.push({ key: one.key, name: one.name, url });
    }
    bench = shown;
    loading = false;
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-[#34251c]/40 p-4"
  onclick={(e) => e.target === e.currentTarget && onclose()}
>
  <div class="max-h-[80vh] w-full max-w-3xl overflow-y-auto border border-[#d8c6b1] bg-[#f8f1e7] p-5">
    {#if loading}
      <p class="text-sm text-[#8a6a55]">{$t('studioLoading')}</p>
    {:else}
      <!-- Свой верстак первым: чаще всего надевают то, что только что принесли. -->
      <section>
        <h3 class="text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
          {$t('studioMyPieces')}
        </h3>
        {#if bench.length === 0}
          <p class="mt-2 text-xs text-[#b0a08e]">{$t('studioPickEmpty')}</p>
        {:else}
          <div class="mt-3 grid grid-cols-4 gap-2 sm:grid-cols-6">
            {#each bench as one (one.key)}
              <button
                onclick={() => onpick(one.key)}
                title={one.name}
                class="aspect-square border border-[#34251c]/10 bg-[#fdf9f3] p-1 hover:border-[#c65f3c]"
              >
                <img src={one.url} alt={one.name} class="h-full w-full object-contain" />
              </button>
            {/each}
          </div>
        {/if}
      </section>

      <section class="mt-6">
        <h3 class="text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
          {$t('studioPickFromLibrary')}
        </h3>
        {#if house.length === 0}
          <p class="mt-2 text-xs text-[#b0a08e]">{$t('studioPickEmpty')}</p>
        {:else}
          <div class="mt-3 grid grid-cols-4 gap-2 sm:grid-cols-6">
            {#each house as one (one.id)}
              <button
                onclick={() => onpick(one.url)}
                title={one.name}
                class="aspect-square border border-[#34251c]/10 bg-[#fdf9f3] p-1 hover:border-[#c65f3c]"
              >
                <img src={one.url} alt={one.name} class="h-full w-full object-contain" />
              </button>
            {/each}
          </div>
        {/if}
      </section>

      <section class="mt-6">
        <h3 class="text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
          {$t('studioPickFromBox')}
        </h3>
        {#if mine.length === 0}
          <p class="mt-2 text-xs text-[#b0a08e]">{$t('studioPickEmpty')}</p>
        {:else}
          <div class="mt-3 grid grid-cols-4 gap-2 sm:grid-cols-6">
            {#each mine as one (one.id)}
              <button
                onclick={() => onpick(one.url)}
                title={one.name}
                class="aspect-square border border-[#34251c]/10 bg-[#fdf9f3] p-1 hover:border-[#c65f3c]"
              >
                <img src={one.url} alt={one.name} class="h-full w-full object-contain" />
              </button>
            {/each}
          </div>
        {/if}
      </section>

      <div class="mt-6 text-right">
        <button
          onclick={onclose}
          class="border border-[#34251c]/20 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
          >{$t('studioPickClose')}</button
        >
      </div>
    {/if}
  </div>
</div>
