<script lang="ts">
  // Ящик склада, выдвинутый над редактором рамки.
  //
  // Открывается уже отфильтрованным по роли того слота, из которого его
  // позвали: в слот угла незачем показывать двадцать накладок. Но фильтр —
  // не запрет: роль ставит хранитель рукой, и «показать все» стоит рядом,
  // чтобы деталь, помеченную не тем словом, всё равно можно было взять.
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import type { BattleAsset, BattleAssetRole } from '$lib/types/api';

  let { role, onPick, onClose } = $props<{
    role: BattleAssetRole;
    onPick: (asset: BattleAsset) => void;
    onClose: () => void;
  }>();

  const ROLE_LABEL: Record<BattleAssetRole, string> = {
    corner: 'adminAssetsRoleCorner',
    sideH: 'adminAssetsRoleSideH',
    sideV: 'adminAssetsRoleSideV',
    accent: 'adminAssetsRoleAccent',
    art: 'adminAssetsRoleArt',
    other: 'adminAssetsRoleOther',
  };

  let assets = $state<BattleAsset[]>([]);
  let anyRole = $state(false);
  let query = $state('');
  let loading = $state(true);
  let error = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      assets = await api.adminListBattleAssets({
        role: anyRole ? undefined : role,
        q: query,
      });
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    load();
  });
</script>

<div
  class="fixed inset-0 z-[60] bg-[#34251c]/30"
  role="presentation"
  onclick={onClose}
  onkeydown={(e) => e.key === 'Escape' && onClose()}
></div>
<aside
  class="fixed top-0 right-0 z-[61] w-[min(560px,94vw)] h-screen overflow-y-auto bg-[#f8f1e7] border-l border-[#34251c]/20 shadow-[-20px_0_50px_rgba(52,37,28,0.18)]"
  aria-label={$t('adminAssetsPickerTitle')}
>
  <div class="sticky top-0 bg-[#f8f1e7] border-b border-[#34251c]/10 px-4 py-3">
    <div class="flex items-center gap-3">
      <h4 class="text-sm" style="font-family: 'Cormorant Garamond', Georgia, serif;">
        {$t('adminAssetsPickerTitle')}
      </h4>
      <span class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
        {$t(ROLE_LABEL[role as BattleAssetRole] as never)}
      </span>
      <button
        onclick={onClose}
        class="ml-auto px-2 text-[#8a6a55] hover:text-[#c65f3c]"
        aria-label="×">×</button
      >
    </div>
    <div class="flex flex-wrap items-center gap-3 mt-3">
      <input
        bind:value={query}
        oninput={load}
        placeholder={$t('adminAssetsSearch')}
        class="flex-1 min-w-[10rem] px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
      />
      <label class="flex items-center gap-2 text-[11px] text-[#5f4636]">
        <input type="checkbox" bind:checked={anyRole} onchange={load} />
        {$t('adminAssetsPickerAny')}
      </label>
    </div>
  </div>

  <div class="p-4">
    {#if error}
      <p class="text-xs text-[#8f2f22]">{error}</p>
    {:else if loading}
      <p class="text-xs italic text-[#5f4636]">…</p>
    {:else if !assets.length}
      <p class="text-xs italic leading-relaxed text-[#5f4636]">{$t('adminAssetsPickerEmpty')}</p>
    {:else}
      <div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(9rem, 1fr));">
        {#each assets as asset (asset.id)}
          <button
            onclick={() => onPick(asset)}
            class="border border-[#34251c]/12 hover:border-[#c65f3c] text-left"
          >
            <div class="flex items-center justify-center h-28 p-2 tile">
              <img src={asset.url} alt={asset.name} class="max-w-full max-h-full object-contain" />
            </div>
            <div class="px-2 py-1 border-t border-[#34251c]/10">
              <p class="text-[11px] leading-snug truncate">{asset.name}</p>
              <p class="text-[10px] tabular-nums text-[#8a6a55]">
                {asset.width}×{asset.height}
                {#if anyRole}
                  · {$t(ROLE_LABEL[asset.role] as never)}
                {/if}
              </p>
            </div>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</aside>

<style>
  /* Та же клетка, что на складе: без неё край прозрачной детали не виден. */
  .tile {
    background-image:
      linear-gradient(45deg, #e4d7c4 25%, transparent 25%),
      linear-gradient(-45deg, #e4d7c4 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, #e4d7c4 75%),
      linear-gradient(-45deg, transparent 75%, #e4d7c4 75%);
    background-size: 14px 14px;
    background-position:
      0 0,
      0 7px,
      7px -7px,
      -7px 0;
    background-color: #f2e8da;
  }
</style>
