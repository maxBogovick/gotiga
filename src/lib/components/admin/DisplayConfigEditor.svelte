<script lang="ts">
  import { t } from '$lib/i18n';
  import type { DisplayConfig, DisplayConfigBackground, BlockStyle } from '$lib/types/api';
  import { BLOCK_IDS, UPPER_ZONE_IDS, isBlockVisible, type BlockId, type UpperZoneId, type ZoneId } from '$lib/components/figurine-detail/display-config';
  import { READING_FONTS } from '$lib/stores/reading-font.svelte';

  let {
    value = $bindable<string | null>(null),
  }: {
    value: string | null;
  } = $props();

  // ── Parse / serialise ─────────────────────────────────────────────────────

  function parse(raw: string | null): DisplayConfig {
    if (!raw) return {};
    try { return JSON.parse(raw) as DisplayConfig; } catch { return {}; }
  }

  function serialise(cfg: DisplayConfig): string | null {
    const isEmpty =
      !cfg.background?.preset &&
      !cfg.blockOrder?.length &&
      !cfg.hiddenBlocks?.length &&
      !Object.keys(cfg.blockStyles ?? {}).length;
    return isEmpty ? null : JSON.stringify(cfg);
  }

  let cfg = $state<DisplayConfig>(parse(value));

  $effect(() => {
    cfg = parse(value);
  });

  function commit() {
    value = serialise(cfg);
  }

  // ── Background ────────────────────────────────────────────────────────────

  const BG_PRESETS: { id: DisplayConfigBackground; label: string; color: string }[] = [
    { id: 'parchment', label: $t('adminDisplayConfigBgParchment'), color: '#f8f1e7' },
    { id: 'aged',      label: $t('adminDisplayConfigBgAged'),      color: '#ede3cf' },
    { id: 'linen',     label: $t('adminDisplayConfigBgLinen'),     color: '#f4efe8' },
    { id: 'dark',      label: $t('adminDisplayConfigBgDark'),      color: '#181210' },
    { id: 'slate',     label: $t('adminDisplayConfigBgSlate'),     color: '#dce0e4' },
    { id: 'custom',    label: $t('adminDisplayConfigBgCustom'),    color: '' },
  ];

  let selectedPreset = $derived(cfg.background?.preset ?? null);
  let customColor = $state(cfg.background?.customColor ?? '#c65f3c');

  function selectPreset(id: DisplayConfigBackground | null) {
    if (id === null) {
      cfg = { ...cfg, background: undefined };
    } else {
      cfg = { ...cfg, background: { ...cfg.background, preset: id } };
    }
    commit();
  }

  function onCustomColorChange(e: Event) {
    customColor = (e.target as HTMLInputElement).value;
    cfg = { ...cfg, background: { ...cfg.background, preset: 'custom', customColor } };
    commit();
  }

  function resolvePreviewColor(p: DisplayConfigBackground | null): string {
    if (!p) return '#f8f1e7';
    const found = BG_PRESETS.find(x => x.id === p);
    if (p === 'custom') return cfg.background?.customColor ?? '#f8f1e7';
    return found?.color ?? '#f8f1e7';
  }

  // ── Block order ────────────────────────────────────────────────────────────

  const BLOCK_LABELS: Record<BlockId, string> = {
    description: $t('adminDisplayConfigBlockDescription'),
    making:      $t('adminDisplayConfigBlockMaking'),
    video:       $t('adminDisplayConfigBlockVideo'),
    showings:    $t('adminDisplayConfigBlockShowings'),
    related:     $t('adminDisplayConfigBlockRelated'),
    comments:    $t('adminDisplayConfigBlockComments'),
  };

  const BLOCK_COLORS: Record<BlockId, string> = {
    description: '#8b7355',
    making:      '#6f4e37',
    video:       '#5a4a3a',
    showings:    '#7a6a55',
    related:     '#6b5a47',
    comments:    '#8c7a66',
  };

  let blockOrder = $derived<BlockId[]>(
    (cfg.blockOrder as BlockId[] | undefined) ?? ([...BLOCK_IDS] as BlockId[])
  );

  function moveBlock(index: number, dir: -1 | 1) {
    const newOrder = [...blockOrder];
    const target = index + dir;
    if (target < 0 || target >= newOrder.length) return;
    [newOrder[index], newOrder[target]] = [newOrder[target], newOrder[index]];
    cfg = { ...cfg, blockOrder: newOrder };
    commit();
  }

  // ── Block styles ──────────────────────────────────────────────────────────

  let expandedBlock = $state<BlockId | null>(null);

  const FONT_OPTIONS = [
    { id: '',  label: $t('adminDisplayConfigFontDefault'), family: 'inherit' },
    ...READING_FONTS.map(f => ({ id: f.id, label: f.name, family: f.stack })),
  ];

  const SIZE_OPTIONS: { id: NonNullable<BlockStyle['fontSize']>; label: string }[] = [
    { id: 'sm',   label: 'S' },
    { id: 'base', label: 'M' },
    { id: 'lg',   label: 'L' },
    { id: 'xl',   label: 'XL' },
  ];

  function getZoneStyle(zoneId: ZoneId): BlockStyle {
    return cfg.blockStyles?.[zoneId] ?? {};
  }

  function setZoneStyle(zoneId: ZoneId, patch: Partial<BlockStyle>) {
    const current = cfg.blockStyles?.[zoneId] ?? {};
    const updated: BlockStyle = { ...current, ...patch };
    if (!updated.color) delete updated.color;
    if (!updated.fontSize || updated.fontSize === 'base') delete updated.fontSize;
    if (!updated.font) delete updated.font;

    const blockStyles = { ...(cfg.blockStyles ?? {}) };
    if (!Object.keys(updated).length) {
      delete blockStyles[zoneId];
    } else {
      blockStyles[zoneId] = updated;
    }
    cfg = { ...cfg, blockStyles: Object.keys(blockStyles).length ? blockStyles : undefined };
    commit();
  }

  function clearZoneStyle(zoneId: ZoneId) {
    const blockStyles = { ...(cfg.blockStyles ?? {}) };
    delete blockStyles[zoneId];
    cfg = { ...cfg, blockStyles: Object.keys(blockStyles).length ? blockStyles : undefined };
    commit();
  }

  function toggleBlockVisibility(blockId: BlockId) {
    const hidden = cfg.hiddenBlocks ?? [];
    cfg = {
      ...cfg,
      hiddenBlocks: hidden.includes(blockId)
        ? hidden.filter(id => id !== blockId)
        : [...hidden, blockId],
    };
    commit();
  }

  function resetOrder() {
    cfg = { ...cfg, blockOrder: undefined };
    commit();
  }

  function resetAll() {
    cfg = {};
    value = null;
  }

  // ── Preview ───────────────────────────────────────────────────────────────

  let bgColor = $derived(resolvePreviewColor(selectedPreset));

  const PREVIEW_H = 120;
  const BLOCK_H = 10;
  const BLOCK_GAP = 4;
</script>

{#snippet stylePanel(zoneId: ZoneId, bst: BlockStyle, hasStyle: boolean)}
  <li class="dce-style-panel">
    <div class="dce-style-row">
      <span class="dce-style-label">{$t('adminDisplayConfigTextColor')}</span>
      <div class="dce-color-row">
        <input
          type="color"
          value={bst.color ?? '#34251c'}
          onchange={e => setZoneStyle(zoneId, { color: (e.target as HTMLInputElement).value })}
          class="dce-color-picker dce-color-picker--sm"
        />
        {#if bst.color}
          <span class="dce-color-hex-small">{bst.color}</span>
          <button type="button" class="dce-clear-btn" onclick={() => setZoneStyle(zoneId, { color: undefined })} title="Reset">×</button>
        {:else}
          <span class="dce-style-hint">{$t('adminDisplayConfigFontDefault')}</span>
        {/if}
      </div>
    </div>
    <div class="dce-style-row">
      <span class="dce-style-label">{$t('adminDisplayConfigTextSize')}</span>
      <div class="dce-btn-group">
        {#each SIZE_OPTIONS as sz}
          <button
            type="button"
            class="dce-choice-btn"
            class:dce-choice-btn--active={(bst.fontSize ?? 'base') === sz.id}
            onclick={() => setZoneStyle(zoneId, { fontSize: sz.id })}
          >{sz.label}</button>
        {/each}
      </div>
    </div>
    <div class="dce-style-row">
      <span class="dce-style-label">{$t('adminDisplayConfigTextFont')}</span>
      <div class="dce-btn-group">
        {#each FONT_OPTIONS as f}
          <button
            type="button"
            class="dce-choice-btn"
            class:dce-choice-btn--active={(bst.font ?? '') === f.id}
            style="font-family:{f.family}"
            onclick={() => setZoneStyle(zoneId, { font: f.id || undefined })}
          >{f.label}</button>
        {/each}
      </div>
    </div>
    {#if hasStyle}
      <button type="button" class="dce-reset-btn" onclick={() => clearZoneStyle(zoneId)}>
        {$t('adminDisplayConfigReset')}
      </button>
    {/if}
  </li>
{/snippet}

<div class="dce-root">
  <div class="dce-cols">

    <!-- ── Left: controls ── -->
    <div class="dce-controls">

      <!-- Background -->
      <div class="dce-section">
        <p class="dce-section-label">{$t('adminDisplayConfigBg')}</p>
        <div class="dce-swatches">
          <!-- "None" = parchment default -->
          <button
            type="button"
            class="dce-swatch dce-swatch--none"
            class:dce-swatch--active={!selectedPreset}
            onclick={() => selectPreset(null)}
            title="{$t('adminDisplayConfigBgParchment')} (default)"
          >
            <span class="dce-swatch-dot" style="background:#f8f1e7; border:1px solid #d8c6b1"></span>
          </button>
          {#each BG_PRESETS as p}
            <button
              type="button"
              class="dce-swatch"
              class:dce-swatch--active={selectedPreset === p.id}
              onclick={() => selectPreset(p.id)}
              title={p.label}
            >
              {#if p.id === 'custom'}
                <span class="dce-swatch-dot dce-swatch-dot--custom">
                  <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                    <path d="M5 1v8M1 5h8"/>
                  </svg>
                </span>
              {:else}
                <span class="dce-swatch-dot" style="background:{p.color}"></span>
              {/if}
            </button>
          {/each}
        </div>

        {#if selectedPreset === 'custom'}
          <div class="dce-custom-color">
            <label class="dce-field-label">{$t('adminDisplayConfigCustomColor')}</label>
            <div class="dce-color-row">
              <input type="color" value={customColor} onchange={onCustomColorChange} class="dce-color-picker" />
              <input type="text" value={customColor} onchange={onCustomColorChange} class="dce-color-hex input-gothic" placeholder="#rrggbb" maxlength="7" />
            </div>
          </div>
        {/if}
      </div>

      <!-- Block order -->
      <!-- Upper page elements (name, shortText) — style only, no order/hide -->
      <div class="dce-section">
        <p class="dce-section-label">{$t('adminDisplayConfigUpperElements')}</p>
        <ol class="dce-block-list">
          {#each UPPER_ZONE_IDS as zoneId (zoneId)}
            {@const bst = getZoneStyle(zoneId)}
            {@const isExpanded = expandedBlock === zoneId}
            {@const hasStyle = !!(bst.color || bst.fontSize || bst.font)}
            <li class="dce-block-item">
              <span class="dce-block-color" style="background:{zoneId === 'name' ? '#6f3b24' : '#c65f3c'}"></span>
              <span class="dce-block-name">{$t(zoneId === 'name' ? 'adminDisplayConfigZoneName' : 'adminDisplayConfigZoneShortText')}</span>
              <button
                type="button"
                class="dce-style-btn"
                class:dce-style-btn--active={isExpanded || hasStyle}
                onclick={() => expandedBlock = isExpanded ? null : zoneId}
                aria-label="Text style"
                title={$t('adminDisplayConfigTextStyle')}
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                  <path d="M4 7V4h16v3"/><path d="M9 20h6"/><path d="M12 4v16"/>
                </svg>
              </button>
            </li>
            {#if isExpanded}
              {@render stylePanel(zoneId, bst, hasStyle)}
            {/if}
          {/each}
        </ol>
      </div>

      <!-- Content blocks (order + hide + style) -->
      <div class="dce-section">
        <p class="dce-section-label">{$t('adminDisplayConfigBlockOrder')}</p>
        <ol class="dce-block-list">
          {#each blockOrder as blockId, i (blockId)}
            {@const hidden = !isBlockVisible(cfg, blockId)}
            {@const bst = getZoneStyle(blockId)}
            {@const isExpanded = expandedBlock === blockId}
            {@const hasStyle = !!(bst.color || bst.fontSize || bst.font)}
            <li class="dce-block-item" class:dce-block-item--hidden={hidden}>
              <span class="dce-block-color" style="background:{BLOCK_COLORS[blockId]}"></span>
              <span class="dce-block-name" class:dce-block-name--hidden={hidden}>{BLOCK_LABELS[blockId]}</span>
              <button
                type="button"
                class="dce-eye-btn"
                onclick={() => toggleBlockVisibility(blockId)}
                aria-label={hidden ? 'Show block' : 'Hide block'}
                title={hidden ? 'Show block' : 'Hide block'}
              >
                {#if hidden}
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                    <path d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94"/>
                    <path d="M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19"/>
                    <line x1="1" y1="1" x2="23" y2="23"/>
                  </svg>
                {:else}
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                    <circle cx="12" cy="12" r="3"/>
                  </svg>
                {/if}
              </button>
              <button
                type="button"
                class="dce-style-btn"
                class:dce-style-btn--active={isExpanded || hasStyle}
                onclick={() => expandedBlock = isExpanded ? null : blockId}
                aria-label="Text style"
                title={$t('adminDisplayConfigTextStyle')}
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                  <path d="M4 7V4h16v3"/><path d="M9 20h6"/><path d="M12 4v16"/>
                </svg>
              </button>
              <div class="dce-block-btns">
                <button type="button" class="dce-arrow-btn" onclick={() => moveBlock(i, -1)}
                  disabled={i === 0 || hidden} aria-label="Move up">▲</button>
                <button type="button" class="dce-arrow-btn" onclick={() => moveBlock(i, 1)}
                  disabled={i === blockOrder.length - 1 || hidden} aria-label="Move down">▼</button>
              </div>
            </li>
            {#if isExpanded}
              {@render stylePanel(blockId, bst, hasStyle)}
            {/if}
          {/each}
        </ol>
        <button type="button" class="dce-reset-btn" onclick={resetOrder}>
          {$t('adminDisplayConfigReset')}
        </button>
      </div>

      <button type="button" class="dce-reset-all-btn" onclick={resetAll}>
        {$t('adminDisplayConfigReset')} all
      </button>
    </div>

    <!-- ── Right: preview ── -->
    <div class="dce-preview">
      <p class="dce-section-label">{$t('adminDisplayConfigPreview')}</p>
      <svg
        class="dce-preview-svg"
        viewBox="0 0 160 {PREVIEW_H}"
        xmlns="http://www.w3.org/2000/svg"
        aria-hidden="true"
      >
        <!-- Page bg -->
        <rect x="0" y="0" width="160" height={PREVIEW_H} fill={bgColor} rx="3" />

        <!-- Title bar -->
        <rect x="12" y="10" width="80" height="6" rx="1" fill={bgColor === '#181210' ? '#555' : '#b0a090'} opacity="0.7" />
        <rect x="12" y="19" width="50" height="3" rx="1" fill={bgColor === '#181210' ? '#444' : '#c5b8a8'} opacity="0.5" />

        <!-- Gallery plate (right column) -->
        <rect x="100" y="10" width="50" height="60" rx="2" fill={bgColor === '#181210' ? '#2a2420' : '#e8dfd4'} />
        <rect x="102" y="12" width="46" height="56" rx="1" fill={bgColor === '#181210' ? '#3a3030' : '#ddd5c8'} opacity="0.8" />

        <!-- Lower blocks (stacked, reorderable, hidden ones omitted) -->
        {#each blockOrder.filter(id => isBlockVisible(cfg, id)) as blockId, i}
          {@const y = 76 + i * (BLOCK_H + BLOCK_GAP)}
          {@const bSt = cfg.blockStyles?.[blockId]}
          {#if y + BLOCK_H < PREVIEW_H - 4}
            <rect x="12" y={y} width="130" height={BLOCK_H} rx="1.5"
              fill={bSt?.color ?? BLOCK_COLORS[blockId]} opacity={bSt?.color ? '0.55' : '0.35'} />
            <text x="17" y={y + 7} font-size="5"
              fill={bSt?.color ?? (bgColor === '#181210' ? '#bbb' : '#5a4a3a')}
              opacity={bSt?.color ? '1' : '0.8'}
              font-family="monospace">
              {BLOCK_LABELS[blockId].toUpperCase().slice(0, 18)}
            </text>
          {/if}
        {/each}

        <!-- Border frame -->
        <rect x="1" y="1" width="158" height={PREVIEW_H - 2} fill="none"
          stroke={bgColor === '#181210' ? '#444' : '#c8bba8'} stroke-width="1" rx="3" />
      </svg>
    </div>

  </div>
</div>

<style>
  .dce-root {
    border: 1px solid var(--color-border, #d8c6b1);
    border-radius: 3px;
    padding: 1rem;
    background: color-mix(in srgb, var(--color-canvas-base) 80%, white);
  }
  .dce-cols {
    display: grid;
    grid-template-columns: 1fr 180px;
    gap: 1.5rem;
  }
  @media (max-width: 700px) {
    .dce-cols { grid-template-columns: 1fr; }
  }
  .dce-section { margin-bottom: 1rem; }
  .dce-section-label {
    font-size: 0.7rem;
    letter-spacing: .08em;
    text-transform: uppercase;
    color: var(--color-ink-secondary, #6f3b24);
    margin: 0 0 .5rem;
  }
  .dce-swatches {
    display: flex;
    flex-wrap: wrap;
    gap: .375rem;
  }
  .dce-swatch {
    padding: 0;
    border: 2px solid transparent;
    border-radius: 4px;
    cursor: pointer;
    background: none;
    outline-offset: 2px;
    transition: border-color .15s;
  }
  .dce-swatch--active {
    border-color: var(--color-accent, #c65f3c);
  }
  .dce-swatch-dot {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 2px;
    color: #888;
  }
  .dce-swatch-dot--custom {
    background: conic-gradient(from 0deg, #c65f3c, #f8f1e7, #181210, #dce0e4, #c65f3c);
    color: #34251c;
  }
  .dce-custom-color { margin-top: .5rem; }
  .dce-field-label {
    font-size: 0.7rem;
    color: var(--color-ink-secondary, #6f3b24);
    display: block;
    margin-bottom: .25rem;
  }
  .dce-color-row { display: flex; align-items: center; gap: .5rem; }
  .dce-color-picker {
    width: 36px; height: 36px;
    border: 1px solid var(--color-border, #d8c6b1);
    border-radius: 3px;
    padding: 2px;
    cursor: pointer;
  }
  .dce-color-hex { flex: 1; max-width: 100px; }

  .dce-block-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: .25rem;
  }
  .dce-block-item {
    display: flex;
    align-items: center;
    gap: .5rem;
    padding: .25rem .5rem;
    border: 1px solid var(--color-border, #d8c6b1);
    border-radius: 3px;
    background: color-mix(in srgb, var(--color-canvas-base) 70%, white);
    font-size: .75rem;
  }
  .dce-block-color {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .dce-block-name { flex: 1; color: var(--color-ink-primary, #34251c); }
  .dce-block-name--hidden { text-decoration: line-through; opacity: .45; }
  .dce-block-item--hidden { opacity: .6; }
  .dce-eye-btn, .dce-style-btn {
    padding: 0 3px;
    border: 1px solid var(--color-border, #d8c6b1);
    border-radius: 2px;
    background: none;
    cursor: pointer;
    color: var(--color-ink-secondary, #6f3b24);
    line-height: 1;
    display: flex;
    align-items: center;
    transition: background .1s, color .1s;
  }
  .dce-eye-btn:hover, .dce-style-btn:hover { background: var(--color-canvas-sunken, #e8ddd0); }
  .dce-style-btn--active { color: var(--color-accent, #c65f3c); border-color: var(--color-accent, #c65f3c); }

  .dce-style-panel {
    list-style: none;
    padding: .5rem .5rem .4rem;
    border: 1px solid var(--color-border, #d8c6b1);
    border-top: none;
    border-radius: 0 0 3px 3px;
    background: color-mix(in srgb, var(--color-canvas-base) 60%, white);
    display: flex;
    flex-direction: column;
    gap: .35rem;
    margin-top: -1px;
  }
  .dce-style-row {
    display: flex;
    align-items: center;
    gap: .4rem;
    flex-wrap: wrap;
  }
  .dce-style-label {
    font-size: .65rem;
    letter-spacing: .05em;
    text-transform: uppercase;
    color: var(--color-ink-secondary, #6f3b24);
    min-width: 2.8rem;
    flex-shrink: 0;
  }
  .dce-style-hint {
    font-size: .65rem;
    color: var(--color-ink-secondary, #6f3b24);
    opacity: .6;
  }
  .dce-color-hex-small {
    font-size: .65rem;
    font-family: monospace;
    color: var(--color-ink-primary, #34251c);
  }
  .dce-color-picker--sm { width: 24px; height: 24px; padding: 1px; }
  .dce-clear-btn {
    font-size: .7rem;
    padding: 0 3px;
    border: none;
    background: none;
    cursor: pointer;
    color: var(--color-ink-secondary, #6f3b24);
    opacity: .6;
    line-height: 1;
  }
  .dce-clear-btn:hover { opacity: 1; }
  .dce-btn-group { display: flex; gap: 2px; flex-wrap: wrap; }
  .dce-choice-btn {
    padding: 1px 5px;
    font-size: .65rem;
    border: 1px solid var(--color-border, #d8c6b1);
    border-radius: 2px;
    background: none;
    cursor: pointer;
    color: var(--color-ink-primary, #34251c);
    transition: background .1s, border-color .1s;
    line-height: 1.6;
  }
  .dce-choice-btn:hover { background: var(--color-canvas-sunken, #e8ddd0); }
  .dce-choice-btn--active {
    background: var(--color-accent, #c65f3c);
    border-color: var(--color-accent, #c65f3c);
    color: #fff;
  }
  .dce-block-btns { display: flex; flex-direction: column; gap: 1px; }
  .dce-arrow-btn {
    padding: 0 3px;
    font-size: .6rem;
    line-height: 1.4;
    border: 1px solid var(--color-border, #d8c6b1);
    border-radius: 2px;
    background: none;
    cursor: pointer;
    color: var(--color-ink-primary, #34251c);
    transition: background .1s;
  }
  .dce-arrow-btn:hover:not(:disabled) { background: var(--color-canvas-sunken, #e8ddd0); }
  .dce-arrow-btn:disabled { opacity: .3; cursor: default; }

  .dce-reset-btn, .dce-reset-all-btn {
    margin-top: .5rem;
    font-size: .7rem;
    color: var(--color-ink-secondary, #6f3b24);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .dce-reset-all-btn { display: block; margin-top: .75rem; }

  .dce-preview-svg {
    width: 100%;
    height: auto;
    border-radius: 3px;
    box-shadow: 0 1px 4px rgba(52,37,28,.15);
  }
</style>
