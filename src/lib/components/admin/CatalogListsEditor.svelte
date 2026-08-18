<script lang="ts">
  /**
   * Features / Perfect for checklists for the Specimen catalog leaf.
   *
   * Null JSON → every built-in box is checked (the house default). Custom
   * lines sit under each list and may be added, edited, or removed.
   */
  import { t, type TranslationKey } from '$lib/i18n';
  import { untrack } from 'svelte';
  import type { Figurine } from '$lib/types/api';
  import {
    CATALOG_FEATURE_KEYS,
    CATALOG_PERFECT_KEYS,
    isCatalogKeyOn,
    newCustomLine,
    parseCatalogLists,
    serialiseCatalogLists,
    toggleCatalogKey,
    type CatalogLists,
  } from '$lib/catalog-lists';

  let {
    value = $bindable<string | null>(null),
    figurine,
  }: {
    value: string | null;
    figurine: Figurine;
  } = $props();

  let lists = $state<CatalogLists>(parseCatalogLists(value));

  $effect(() => {
    const incoming = value;
    const current = untrack(() => serialiseCatalogLists(lists));
    if (incoming !== current) lists = parseCatalogLists(incoming);
  });

  function commit() {
    value = serialiseCatalogLists(lists);
  }

  function featureOn(key: string): boolean {
    return isCatalogKeyOn(lists.featuresSelected, key);
  }

  function perfectOn(key: string): boolean {
    return isCatalogKeyOn(lists.perfectSelected, key);
  }

  function toggleFeature(key: string) {
    lists.featuresSelected = toggleCatalogKey(lists.featuresSelected, key, CATALOG_FEATURE_KEYS);
    commit();
  }

  function togglePerfect(key: string) {
    lists.perfectSelected = toggleCatalogKey(lists.perfectSelected, key, CATALOG_PERFECT_KEYS);
    commit();
  }

  function addCustom(kind: 'features' | 'perfect') {
    const line = newCustomLine();
    if (kind === 'features') {
      lists.featuresCustom = [...(lists.featuresCustom ?? []), line];
    } else {
      lists.perfectCustom = [...(lists.perfectCustom ?? []), line];
    }
    commit();
  }

  function removeCustom(kind: 'features' | 'perfect', id: string) {
    if (kind === 'features') {
      lists.featuresCustom = (lists.featuresCustom ?? []).filter((line) => line.id !== id);
    } else {
      lists.perfectCustom = (lists.perfectCustom ?? []).filter((line) => line.id !== id);
    }
    commit();
  }

  function touchCustom() {
    lists.featuresCustom = [...(lists.featuresCustom ?? [])];
    lists.perfectCustom = [...(lists.perfectCustom ?? [])];
    commit();
  }

  function hasText(v: string | null | undefined): boolean {
    return typeof v === 'string' && v.trim().length > 0;
  }

  function includedPreview(): string {
    if (!hasText(figurine.includedItems)) return '';
    return figurine.includedItems!.trim().split(/\n/)[0]?.trim() ?? '';
  }

  const featureMeta: { key: (typeof CATALOG_FEATURE_KEYS)[number]; depends?: boolean }[] = [
    { key: 'unique' },
    { key: 'material', depends: true },
    { key: 'technique', depends: true },
    { key: 'handPainted' },
    { key: 'handFinished' },
    { key: 'recorded', depends: true },
    { key: 'included', depends: true },
    { key: 'quietRoom' },
  ];

  function featureLabel(key: (typeof CATALOG_FEATURE_KEYS)[number]): string {
    switch (key) {
      case 'unique':
        return $t('catalogFeatureUnique');
      case 'material':
        return hasText(figurine.material)
          ? `${$t('catalogFeatureMaterial')} ${figurine.material!.trim()}`
          : $t('catalogFeatureMaterial');
      case 'technique':
        return hasText(figurine.technique) ? figurine.technique!.trim() : $t('adminFieldTechnique');
      case 'handPainted':
        return $t('catalogFeatureHandPainted');
      case 'handFinished':
        return $t('catalogFeatureHandFinished');
      case 'recorded':
        return $t('catalogFeatureRecorded');
      case 'included':
        return includedPreview() || $t('passportIncluded');
      case 'quietRoom':
        return $t('catalogFeatureQuietRoom');
    }
  }

  function featureMissing(key: (typeof CATALOG_FEATURE_KEYS)[number]): boolean {
    switch (key) {
      case 'material':
        return !hasText(figurine.material);
      case 'technique':
        return !hasText(figurine.technique);
      case 'recorded':
        return !hasText(figurine.passportNumber) && !hasText(figurine.authenticityNote);
      case 'included':
        return !hasText(figurine.includedItems);
      default:
        return false;
    }
  }

  const perfectMeta: { key: (typeof CATALOG_PERFECT_KEYS)[number]; labelKey: TranslationKey }[] = [
    { key: 'collectors', labelKey: 'catalogPerfectCollectors' },
    { key: 'cabinet', labelKey: 'catalogPerfectCabinet' },
    { key: 'looking', labelKey: 'catalogPerfectLooking' },
    { key: 'closeWork', labelKey: 'catalogPerfectCloseWork' },
    { key: 'display', labelKey: 'catalogPerfectDisplay' },
    { key: 'gift', labelKey: 'catalogPerfectGift' },
  ];
</script>

<p class="text-xs text-[#5f4636]/70 mb-6 max-w-prose leading-relaxed">{$t('adminCatalogListsHint')}</p>

<div class="grid grid-cols-1 lg:grid-cols-2 gap-10">
  <section>
    <h3 class="label mb-3">{$t('catalogFeaturesTitle')}</h3>
    <ul class="space-y-2">
      {#each featureMeta as item (item.key)}
        <li>
          <label class="flex items-start gap-2.5 cursor-pointer group">
            <input
              type="checkbox"
              checked={featureOn(item.key)}
              onchange={() => toggleFeature(item.key)}
              class="accent-[#34251c] w-3.5 h-3.5 mt-0.5 shrink-0"
            />
            <span class="text-[13px] leading-snug text-[#34251c] group-hover:text-[#6f3b24]">
              {featureLabel(item.key)}
              {#if item.depends && featureMissing(item.key)}
                <span class="block text-[10px] uppercase tracking-wide text-[#5f4636]/55 mt-0.5">{$t('adminCatalogDependsHint')}</span>
              {/if}
            </span>
          </label>
        </li>
      {/each}
    </ul>

    <div class="mt-5 space-y-2">
      {#each lists.featuresCustom ?? [] as line (line.id)}
        <div class="flex items-center gap-2">
          <input
            type="checkbox"
            bind:checked={line.enabled}
            onchange={touchCustom}
            class="accent-[#34251c] w-3.5 h-3.5 shrink-0"
          />
          <input
            bind:value={line.text}
            oninput={touchCustom}
            class="input-gothic py-1.5 text-[13px]"
            placeholder={$t('adminCatalogCustomPlaceholder')}
          />
          <button
            type="button"
            onclick={() => removeCustom('features', line.id)}
            class="text-[#5f4636]/50 hover:text-red-700 text-sm px-1"
            title={$t('adminCatalogRemoveLine')}
          >✕</button>
        </div>
      {/each}
      <button type="button" onclick={() => addCustom('features')} class="btn-gothic text-[10px] mt-1">
        {$t('adminCatalogAddLine')}
      </button>
    </div>
  </section>

  <section>
    <h3 class="label mb-3">{$t('catalogPerfectTitle')}</h3>
    <ul class="space-y-2">
      {#each perfectMeta as item (item.key)}
        <li>
          <label class="flex items-start gap-2.5 cursor-pointer group">
            <input
              type="checkbox"
              checked={perfectOn(item.key)}
              onchange={() => togglePerfect(item.key)}
              class="accent-[#34251c] w-3.5 h-3.5 mt-0.5 shrink-0"
            />
            <span class="text-[13px] leading-snug text-[#34251c] group-hover:text-[#6f3b24]">
              {$t(item.labelKey)}
            </span>
          </label>
        </li>
      {/each}
    </ul>

    <div class="mt-5 space-y-2">
      {#each lists.perfectCustom ?? [] as line (line.id)}
        <div class="flex items-center gap-2">
          <input
            type="checkbox"
            bind:checked={line.enabled}
            onchange={touchCustom}
            class="accent-[#34251c] w-3.5 h-3.5 shrink-0"
          />
          <input
            bind:value={line.text}
            oninput={touchCustom}
            class="input-gothic py-1.5 text-[13px]"
            placeholder={$t('adminCatalogCustomPlaceholder')}
          />
          <button
            type="button"
            onclick={() => removeCustom('perfect', line.id)}
            class="text-[#5f4636]/50 hover:text-red-700 text-sm px-1"
            title={$t('adminCatalogRemoveLine')}
          >✕</button>
        </div>
      {/each}
      <button type="button" onclick={() => addCustom('perfect')} class="btn-gothic text-[10px] mt-1">
        {$t('adminCatalogAddLine')}
      </button>
    </div>
  </section>
</div>

<style>
  .label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #5f4636;
    font-weight: 700;
  }

  .input-gothic {
    width: 100%;
    background-color: #f8f1e7;
    border: 1px solid rgba(198, 95, 60, 0.2);
    padding: 0.65rem 0.75rem;
    font-size: 0.875rem;
    color: #34251c;
    outline: none;
  }
  .input-gothic:focus {
    border-color: rgba(198, 95, 60, 0.55);
  }
</style>
