<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import type {
    HomeLayoutConfig,
    HomeLayoutPreset,
    HomeBlockId,
    HomeMainBlockId,
    HomeBandBlockId,
    HomeShelfBlockId,
    HomeBlockSize,
    HomeBlockStyle,
    HomeBlockPadding,
    HomeDevice,
    HomeElementStyle,
    BlockStyle,
    HomeCardEffect,
  } from '$lib/types/api';
  import {
    HOME_MAIN_BLOCK_IDS,
    HOME_BAND_BLOCK_IDS,
    HOME_SHELF_BLOCK_IDS,
    HOME_BG_PRESETS,
    HOME_BLOCK_ELEMENTS,
    ELEMENT_FONT_RANGE,
    ELEMENT_WIDTH_RANGE,
    type HomeElementDef,
    normalizeHomeOrder,
    isHomeBlockVisible,
  } from '$lib/home-layout';
  import { READING_FONTS } from '$lib/stores/reading-font.svelte';

  // ── Config state ───────────────────────────────────────────────────────────

  let cfg = $state<HomeLayoutConfig>({});
  let loading = $state(true);
  let saving = $state(false);
  let savedOk = $state(false);
  let dirty = $state(false);
  let errorMsg = $state('');

  let mainOrder = $derived(normalizeHomeOrder(cfg.blockOrder, HOME_MAIN_BLOCK_IDS));
  let bandOrder = $derived(normalizeHomeOrder(cfg.bandOrder, HOME_BAND_BLOCK_IDS));
  let shelfOrder = $derived(normalizeHomeOrder(cfg.shelfOrder, HOME_SHELF_BLOCK_IDS));

  type ListId = 'main' | 'band' | 'shelf';

  const BLOCK_LABELS: Record<HomeBlockId, string> = {
    hero:            $t('adminHomeLayoutBlockHero'),
    returningBand:   $t('adminHomeLayoutBlockReturningBand'),
    visitLedger:     $t('adminHomeLayoutBlockVisitLedger'),
    noticeBoard:     $t('adminHomeLayoutBlockNoticeBoard'),
    gallery:         $t('adminHomeLayoutBlockGallery'),
    authorStory:     $t('adminHomeLayoutBlockAuthorStory'),
    correspondence:  $t('adminHomeLayoutBlockCorrespondence'),
    impressions:     $t('adminHomeLayoutBlockImpressions'),
    requestSteps:    $t('adminHomeLayoutBlockRequestSteps'),
    visitorBook:     $t('adminHomeLayoutBlockVisitorBook'),
    latelyShelves:   $t('adminHomeLayoutBlockLatelyShelves'),
    firstLook:       $t('adminHomeLayoutBlockFirstLook'),
    markedByYou:     $t('adminHomeLayoutBlockMarkedByYou'),
    noticedByGuests: $t('adminHomeLayoutBlockNoticedByGuests'),
  };

  // Blocks that also hide themselves at runtime (empty week, unsigned book…) —
  // flagged in the list so the admin doesn't mistake self-hiding for a bug.
  const RUNTIME_HIDING: HomeBlockId[] = ['noticeBoard', 'firstLook', 'markedByYou', 'noticedByGuests'];

  onMount(async () => {
    try {
      cfg = await api.getHomeLayout();
    } catch {
      errorMsg = $t('adminHomeLayoutLoadError');
    } finally {
      loading = false;
    }
    try { presets = await api.getHomeLayoutPresets(); } catch { /* not critical */ }
    finally { presetsLoading = false; }
  });

  function touch() {
    dirty = true;
    savedOk = false;
    sendPreview();
  }

  // ── Order (drag-and-drop + arrow fallback) ────────────────────────────────

  let drag = $state<{ list: ListId; index: number } | null>(null);

  function listOrder(list: ListId): HomeBlockId[] {
    return list === 'main' ? mainOrder : list === 'band' ? bandOrder : shelfOrder;
  }

  function setListOrder(list: ListId, order: HomeBlockId[]) {
    if (list === 'main') cfg.blockOrder = order as HomeMainBlockId[];
    else if (list === 'band') cfg.bandOrder = order as HomeBandBlockId[];
    else cfg.shelfOrder = order as HomeShelfBlockId[];
    touch();
  }

  function reorder(list: ListId, from: number, to: number) {
    const order = [...listOrder(list)];
    const [moved] = order.splice(from, 1);
    order.splice(to, 0, moved);
    setListOrder(list, order);
  }

  function moveBlock(list: ListId, index: number, dir: -1 | 1) {
    const target = index + dir;
    if (target < 0 || target >= listOrder(list).length) return;
    reorder(list, index, target);
  }

  function onDragStart(list: ListId, index: number, e: DragEvent) {
    drag = { list, index };
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      // Firefox needs data set for the drag to start at all.
      e.dataTransfer.setData('text/plain', '');
    }
  }

  function onDragOver(list: ListId, index: number, e: DragEvent) {
    if (!drag || drag.list !== list) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    if (drag.index !== index) {
      reorder(list, drag.index, index);
      drag = { list, index };
    }
  }

  // ── Visibility / size / style ──────────────────────────────────────────────

  function toggleVisible(blockId: HomeBlockId) {
    const hidden = cfg.hiddenBlocks ?? [];
    cfg.hiddenBlocks = hidden.includes(blockId)
      ? hidden.filter((id) => id !== blockId)
      : [...hidden, blockId];
    touch();
  }

  const SIZE_OPTIONS: { id: HomeBlockSize; label: string; title: string }[] = [
    { id: 'full',      label: '⬌', title: $t('adminHomeLayoutSizeFull') },
    { id: 'contained', label: '▭', title: $t('adminHomeLayoutSizeContained') },
    { id: 'compact',   label: '▯', title: $t('adminHomeLayoutSizeCompact') },
  ];

  function blockSize(blockId: HomeBlockId): HomeBlockSize {
    return cfg.sizes?.[blockId] ?? 'contained';
  }

  function setSize(blockId: HomeBlockId, size: HomeBlockSize) {
    const sizes = { ...(cfg.sizes ?? {}) };
    if (size === 'contained') delete sizes[blockId];
    else sizes[blockId] = size;
    cfg.sizes = Object.keys(sizes).length ? sizes : undefined;
    touch();
  }

  let expandedBlock = $state<HomeBlockId | null>(null);

  const FONT_OPTIONS = [
    { id: '', label: $t('adminDisplayConfigFontDefault'), family: 'inherit' },
    ...READING_FONTS.map((f) => ({ id: f.id, label: f.name, family: f.stack })),
  ];

  const TEXT_SIZE_OPTIONS: { id: NonNullable<BlockStyle['fontSize']>; label: string }[] = [
    { id: 'sm', label: 'S' },
    { id: 'base', label: 'M' },
    { id: 'lg', label: 'L' },
    { id: 'xl', label: 'XL' },
  ];

  function blockStyle(blockId: HomeBlockId): HomeBlockStyle {
    return cfg.blockStyles?.[blockId] ?? {};
  }

  function hasStyle(blockId: HomeBlockId): boolean {
    const s = blockStyle(blockId);
    return !!(
      s.color || s.background || s.font || (s.fontSize && s.fontSize !== 'base')
      || (s.paddingY && s.paddingY !== 'base') || s.divider || s.hideOn?.length
    );
  }

  function setStyle(blockId: HomeBlockId, patch: Partial<HomeBlockStyle>) {
    const updated: HomeBlockStyle = { ...blockStyle(blockId), ...patch };
    if (!updated.color) delete updated.color;
    if (!updated.background) delete updated.background;
    if (!updated.fontSize || updated.fontSize === 'base') delete updated.fontSize;
    if (!updated.font) delete updated.font;
    if (!updated.paddingY || updated.paddingY === 'base') delete updated.paddingY;
    if (!updated.divider) delete updated.divider;
    if (!updated.hideOn?.length) delete updated.hideOn;
    const blockStyles = { ...(cfg.blockStyles ?? {}) };
    if (Object.keys(updated).length) blockStyles[blockId] = updated;
    else delete blockStyles[blockId];
    cfg.blockStyles = Object.keys(blockStyles).length ? blockStyles : undefined;
    touch();
  }

  const PADDING_OPTIONS: { id: HomeBlockPadding; label: string }[] = [
    { id: 'tight',    label: $t('adminHomeLayoutPadTight') },
    { id: 'base',     label: $t('adminHomeLayoutPadBase') },
    { id: 'roomy',    label: $t('adminHomeLayoutPadRoomy') },
    { id: 'spacious', label: $t('adminHomeLayoutPadSpacious') },
  ];

  const DEVICES: { id: HomeDevice; label: string }[] = [
    { id: 'desktop', label: $t('adminHomeLayoutBpDesktop') },
    { id: 'tablet',  label: $t('adminHomeLayoutBpTablet') },
    { id: 'mobile',  label: $t('adminHomeLayoutBpMobile') },
  ];

  function toggleDevice(blockId: HomeBlockId, device: HomeDevice) {
    const hideOn = blockStyle(blockId).hideOn ?? [];
    setStyle(blockId, {
      hideOn: hideOn.includes(device)
        ? hideOn.filter((d) => d !== device)
        : [...hideOn, device],
    });
  }

  const BG_SWATCH_LABELS: Record<string, string> = {
    parchment: $t('adminDisplayConfigBgParchment'),
    aged:      $t('adminDisplayConfigBgAged'),
    linen:     $t('adminDisplayConfigBgLinen'),
    dark:      $t('adminDisplayConfigBgDark'),
    slate:     $t('adminDisplayConfigBgSlate'),
  };

  // ── Element-level overrides ────────────────────────────────────────────────

  const ELEMENT_LABELS: Record<string, string> = {
    orn:      $t('adminHomeLayoutElOrnament'),
    title:    $t('adminHomeLayoutElTitle'),
    lead:     $t('adminHomeLayoutElLead'),
    ctas:     $t('adminHomeLayoutElCtas'),
    proof:    $t('adminHomeLayoutElProof'),
    teasers:  $t('adminHomeLayoutElTeasers'),
    visual:   $t('adminHomeLayoutElVisual'),
    header:   $t('adminHomeLayoutElHeader'),
    content:  $t('adminHomeLayoutElContent'),
    more:     $t('adminHomeLayoutElMore'),
    eyebrow:  $t('adminHomeLayoutElEyebrow'),
    desc:     $t('adminHomeLayoutElDesc'),
    guideRow: $t('adminHomeLayoutElGuideRow'),
    name:     $t('adminHomeLayoutElName'),
    body:     $t('adminHomeLayoutElBody'),
    actions:  $t('adminHomeLayoutElActions'),
    portrait: $t('adminHomeLayoutElPortrait'),
    quotes:   $t('adminHomeLayoutElQuotes'),
    steps:    $t('adminHomeLayoutElSteps'),
    form:     $t('adminHomeLayoutElForm'),
    marks:    $t('adminHomeLayoutElMarks'),
    vitrine:  $t('adminHomeLayoutElVitrine'),
    grid:     $t('adminHomeLayoutElGrid'),
  };

  const elKey = (blockId: HomeBlockId, elementId: string) => `${blockId}.${elementId}`;

  function elementStyle(blockId: HomeBlockId, elementId: string): HomeElementStyle {
    return cfg.elements?.[elKey(blockId, elementId)] ?? {};
  }

  function setElement(blockId: HomeBlockId, elementId: string, patch: Partial<HomeElementStyle>) {
    const key = elKey(blockId, elementId);
    const updated: HomeElementStyle = { ...elementStyle(blockId, elementId), ...patch };
    if (!updated.color) delete updated.color;
    if (!updated.sizePx) delete updated.sizePx;
    if (!updated.widthPct) delete updated.widthPct;
    if (!updated.hidden) delete updated.hidden;
    const elements = { ...(cfg.elements ?? {}) };
    if (Object.keys(updated).length) elements[key] = updated;
    else delete elements[key];
    cfg.elements = Object.keys(elements).length ? elements : undefined;
    touch();
  }

  /** Orderable element ids of a block in their effective order. */
  function elementOrderFor(blockId: HomeBlockId): string[] {
    const orderable = (HOME_BLOCK_ELEMENTS[blockId] ?? []).filter((d) => d.orderable).map((d) => d.id);
    const saved = cfg.elementOrder?.[blockId];
    if (!saved?.length) return orderable;
    const known = saved.filter((id) => orderable.includes(id));
    return [...known, ...orderable.filter((id) => !known.includes(id))];
  }

  function moveElement(blockId: HomeBlockId, elementId: string, dir: -1 | 1) {
    const order = elementOrderFor(blockId);
    const i = order.indexOf(elementId);
    const target = i + dir;
    if (i < 0 || target < 0 || target >= order.length) return;
    [order[i], order[target]] = [order[target], order[i]];
    cfg.elementOrder = { ...(cfg.elementOrder ?? {}), [blockId]: order };
    touch();
  }

  /** Registry rows sorted for display: orderable elements in their effective
   *  order first, style-only elements after. */
  function elementRows(blockId: HomeBlockId): HomeElementDef[] {
    const defs = HOME_BLOCK_ELEMENTS[blockId] ?? [];
    const order = elementOrderFor(blockId);
    const orderable = order
      .map((id) => defs.find((d) => d.id === id))
      .filter((d): d is HomeElementDef => !!d);
    return [...orderable, ...defs.filter((d) => !d.orderable)];
  }

  // ── Page background ────────────────────────────────────────────────────────

  function setPageBackground(color: string | undefined) {
    cfg.pageBackground = color;
    touch();
  }

  // ── THE COLLECTION card scroll-reveal effect ──────────────────────────────

  const CARD_EFFECT_OPTIONS: { id: HomeCardEffect; label: string }[] = [
    { id: 'rise', label: 'Восстаёт со дна' },
    { id: 'fog', label: 'Всплывает из тумана' },
    { id: 'hoist', label: 'Поднимается с рывком' },
    { id: 'drift', label: 'Всплывает наискось' },
    { id: 'unfold', label: 'Раскрывается снизу' },
    { id: 'shadow', label: 'Всплывает из тени' },
  ];

  function setCardEffect(id: HomeCardEffect) {
    cfg.cardEffect = id;
    touch();
  }

  // ── Named layout presets ───────────────────────────────────────────────────

  let presets = $state<HomeLayoutPreset[]>([]);
  let presetsLoading = $state(true);
  let showPresetForm = $state(false);
  let presetNameInput = $state('');
  let applyConfirmId = $state<string | null>(null);

  async function persistPresets(list: HomeLayoutPreset[]) {
    presets = list;
    try { await api.saveHomeLayoutPresets($state.snapshot(list)); } catch { /* local state stays */ }
  }

  async function savePreset() {
    const name = presetNameInput.trim();
    if (!name) return;
    await persistPresets([
      ...presets,
      { id: crypto.randomUUID(), name, config: cleanedConfig(), savedAt: new Date().toISOString() },
    ]);
    showPresetForm = false;
    presetNameInput = '';
  }

  function applyPreset(p: HomeLayoutPreset) {
    if (applyConfirmId !== p.id) {
      applyConfirmId = p.id;
      return;
    }
    cfg = structuredClone($state.snapshot(p.config));
    applyConfirmId = null;
    touch();
  }

  async function deletePreset(id: string) {
    if (applyConfirmId === id) applyConfirmId = null;
    await persistPresets(presets.filter((p) => p.id !== id));
  }

  function clearStyle(blockId: HomeBlockId) {
    const blockStyles = { ...(cfg.blockStyles ?? {}) };
    delete blockStyles[blockId];
    cfg.blockStyles = Object.keys(blockStyles).length ? blockStyles : undefined;
    touch();
  }

  // ── Save / reset ───────────────────────────────────────────────────────────

  function cleanedConfig(): HomeLayoutConfig {
    const out: HomeLayoutConfig = {};
    if (cfg.blockOrder?.length) out.blockOrder = cfg.blockOrder;
    if (cfg.bandOrder?.length) out.bandOrder = cfg.bandOrder;
    if (cfg.shelfOrder?.length) out.shelfOrder = cfg.shelfOrder;
    if (cfg.hiddenBlocks?.length) out.hiddenBlocks = cfg.hiddenBlocks;
    if (cfg.sizes && Object.keys(cfg.sizes).length) out.sizes = cfg.sizes;
    if (cfg.blockStyles && Object.keys(cfg.blockStyles).length) out.blockStyles = cfg.blockStyles;
    if (cfg.elements && Object.keys(cfg.elements).length) out.elements = cfg.elements;
    if (cfg.elementOrder && Object.keys(cfg.elementOrder).length) out.elementOrder = cfg.elementOrder;
    if (cfg.pageBackground) out.pageBackground = cfg.pageBackground;
    if (cfg.cardEffect) out.cardEffect = cfg.cardEffect;
    return $state.snapshot(out);
  }

  async function save() {
    saving = true;
    errorMsg = '';
    try {
      cfg = await api.saveHomeLayout(cleanedConfig());
      dirty = false;
      savedOk = true;
      setTimeout(() => (savedOk = false), 2500);
    } catch {
      errorMsg = $t('adminHomeLayoutSaveError');
    } finally {
      saving = false;
    }
  }

  let resetConfirm = $state(false);

  async function resetAll() {
    if (!resetConfirm) {
      resetConfirm = true;
      setTimeout(() => (resetConfirm = false), 3000);
      return;
    }
    resetConfirm = false;
    cfg = {};
    touch();
    await save();
  }

  // ── Live preview iframe ────────────────────────────────────────────────────

  const BREAKPOINTS = [
    { id: 'desktop', width: 1280, label: $t('adminHomeLayoutBpDesktop') },
    { id: 'tablet',  width: 768,  label: $t('adminHomeLayoutBpTablet') },
    { id: 'mobile',  width: 390,  label: $t('adminHomeLayoutBpMobile') },
  ] as const;

  let bp = $state<(typeof BREAKPOINTS)[number]['id']>('desktop');
  let bpWidth = $derived(BREAKPOINTS.find((b) => b.id === bp)?.width ?? 1280);
  let visitorMode = $state<'returning' | 'new'>('returning');

  let iframeEl = $state<HTMLIFrameElement | null>(null);
  let previewContainerWidth = $state(0);
  const PREVIEW_VIEW_HEIGHT = 720; // on-screen px of the preview viewport
  let previewScale = $derived(previewContainerWidth > 0 ? Math.min(1, previewContainerWidth / bpWidth) : 0.5);
  let iframeHeight = $derived(Math.round(PREVIEW_VIEW_HEIGHT / previewScale));

  function sendPreview() {
    iframeEl?.contentWindow?.postMessage(
      {
        type: 'gotiga-home-layout',
        config: $state.snapshot(cfg),
        visitorMode,
      },
      '*',
    );
  }

  function onIframeLoad() {
    // The home page inside the iframe registers its message listener on mount;
    // fire a few times so the draft lands regardless of hydration timing.
    sendPreview();
    setTimeout(sendPreview, 400);
    setTimeout(sendPreview, 1200);
  }

  function setVisitorMode(mode: 'returning' | 'new') {
    visitorMode = mode;
    sendPreview();
  }

  // Hovering a row outlines the block inside the preview (the public layout
  // already handles 'gotiga-highlight' messages); clicking a name scrolls to it.
  function highlightBlock(blockId: HomeBlockId | null) {
    if (blockId) {
      iframeEl?.contentWindow?.postMessage({
        type: 'gotiga-highlight',
        css: `[data-hl="${blockId}"]{outline:2px dashed rgba(198,95,60,.75);outline-offset:-2px}`,
      }, '*');
    } else {
      iframeEl?.contentWindow?.postMessage({ type: 'gotiga-highlight-clear' }, '*');
    }
  }

  function scrollPreviewTo(blockId: HomeBlockId) {
    iframeEl?.contentWindow?.postMessage({ type: 'gotiga-home-layout', scrollTo: blockId }, '*');
  }
</script>

{#snippet blockRow(list: ListId, blockId: HomeBlockId, index: number, count: number, sub: boolean)}
  {@const hidden = !isHomeBlockVisible(cfg, blockId)}
  {@const styled = hasStyle(blockId)}
  {@const isExpanded = expandedBlock === blockId}
  <li
    class="hle-row"
    class:hle-row--sub={sub}
    class:hle-row--hidden={hidden}
    class:hle-row--dragging={drag?.list === list && drag.index === index}
    draggable="true"
    ondragstart={(e) => onDragStart(list, index, e)}
    ondragover={(e) => onDragOver(list, index, e)}
    ondragend={() => (drag = null)}
    onmouseenter={() => highlightBlock(blockId)}
    onmouseleave={() => highlightBlock(null)}
  >
    <span class="hle-grip" aria-hidden="true">⠿</span>
    <button
      type="button"
      class="hle-name"
      class:hle-name--hidden={hidden}
      onclick={() => scrollPreviewTo(blockId)}
      title={$t('adminHomeLayoutLocate')}
    >
      {BLOCK_LABELS[blockId]}
      {#if RUNTIME_HIDING.includes(blockId)}
        <span class="hle-auto-note" title={$t('adminHomeLayoutRuntimeNote')}>◌</span>
      {/if}
    </button>

    <button
      type="button"
      class="hle-icon-btn"
      onclick={() => toggleVisible(blockId)}
      aria-label={hidden ? $t('adminHomeLayoutShowBlock') : $t('adminHomeLayoutHideBlock')}
      title={hidden ? $t('adminHomeLayoutShowBlock') : $t('adminHomeLayoutHideBlock')}
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

    <div class="hle-size-group" role="group" aria-label={$t('adminHomeLayoutSizeLabel')}>
      {#each SIZE_OPTIONS as sz}
        <button
          type="button"
          class="hle-size-btn"
          class:hle-size-btn--active={blockSize(blockId) === sz.id}
          onclick={() => setSize(blockId, sz.id)}
          title={sz.title}
          aria-label={sz.title}
        >{sz.label}</button>
      {/each}
    </div>

    <button
      type="button"
      class="hle-icon-btn"
      class:hle-icon-btn--active={isExpanded || styled}
      onclick={() => (expandedBlock = isExpanded ? null : blockId)}
      aria-label={$t('adminDisplayConfigTextStyle')}
      title={$t('adminDisplayConfigTextStyle')}
    >
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
        <path d="M4 7V4h16v3"/><path d="M9 20h6"/><path d="M12 4v16"/>
      </svg>
    </button>

    <div class="hle-arrows">
      <button type="button" class="hle-arrow-btn" onclick={() => moveBlock(list, index, -1)}
        disabled={index === 0} aria-label={$t('adminHomeLayoutMoveUp')}>▲</button>
      <button type="button" class="hle-arrow-btn" onclick={() => moveBlock(list, index, 1)}
        disabled={index === count - 1} aria-label={$t('adminHomeLayoutMoveDown')}>▼</button>
    </div>
  </li>

  {#if isExpanded}
    {@const bst = blockStyle(blockId)}
    <li class="hle-style-panel" class:hle-style-panel--sub={sub}>
      <div class="hle-style-row">
        <span class="hle-style-label">{$t('adminDisplayConfigTextColor')}</span>
        <div class="hle-color-row">
          <input
            type="color"
            value={bst.color ?? '#34251c'}
            onchange={(e) => setStyle(blockId, { color: (e.target as HTMLInputElement).value })}
            class="hle-color-picker"
          />
          {#if bst.color}
            <span class="hle-color-hex">{bst.color}</span>
            <button type="button" class="hle-clear-btn" onclick={() => setStyle(blockId, { color: undefined })} title={$t('adminDisplayConfigReset')}>×</button>
          {:else}
            <span class="hle-style-hint">{$t('adminDisplayConfigFontDefault')}</span>
          {/if}
        </div>
      </div>
      <div class="hle-style-row">
        <span class="hle-style-label">{$t('adminDisplayConfigBgColor')}</span>
        <div class="hle-color-row">
          {#each HOME_BG_PRESETS as bg}
            <button
              type="button"
              class="hle-swatch"
              class:hle-swatch--active={bst.background === bg.color}
              style="background:{bg.color}"
              onclick={() => setStyle(blockId, { background: bst.background === bg.color ? undefined : bg.color })}
              title={BG_SWATCH_LABELS[bg.id]}
              aria-label={BG_SWATCH_LABELS[bg.id]}
            ></button>
          {/each}
          <input
            type="color"
            value={bst.background ?? '#f8f1e7'}
            onchange={(e) => setStyle(blockId, { background: (e.target as HTMLInputElement).value })}
            class="hle-color-picker"
            title={$t('adminDisplayConfigCustomColor')}
          />
          {#if bst.background}
            <span class="hle-color-hex">{bst.background}</span>
            <button type="button" class="hle-clear-btn" onclick={() => setStyle(blockId, { background: undefined })} title={$t('adminDisplayConfigReset')}>×</button>
          {:else}
            <span class="hle-style-hint">{$t('adminDisplayConfigFontDefault')}</span>
          {/if}
        </div>
      </div>
      <div class="hle-style-row">
        <span class="hle-style-label">{$t('adminHomeLayoutPaddingLabel')}</span>
        <div class="hle-btn-group">
          {#each PADDING_OPTIONS as p}
            <button
              type="button"
              class="hle-choice-btn"
              class:hle-choice-btn--active={(bst.paddingY ?? 'base') === p.id}
              onclick={() => setStyle(blockId, { paddingY: p.id })}
            >{p.label}</button>
          {/each}
        </div>
      </div>
      <div class="hle-style-row">
        <span class="hle-style-label">{$t('adminHomeLayoutDivider')}</span>
        <button
          type="button"
          class="hle-choice-btn"
          class:hle-choice-btn--active={!!bst.divider}
          onclick={() => setStyle(blockId, { divider: !bst.divider })}
        >{bst.divider ? $t('adminHomeLayoutDividerOn') : $t('adminHomeLayoutDividerOff')}</button>
      </div>
      <div class="hle-style-row">
        <span class="hle-style-label">{$t('adminHomeLayoutShowOn')}</span>
        <div class="hle-btn-group">
          {#each DEVICES as d}
            {@const shown = !(bst.hideOn ?? []).includes(d.id)}
            <button
              type="button"
              class="hle-choice-btn"
              class:hle-choice-btn--active={shown}
              onclick={() => toggleDevice(blockId, d.id)}
              title={shown ? $t('adminHomeLayoutHideBlock') : $t('adminHomeLayoutShowBlock')}
            >{d.label}</button>
          {/each}
        </div>
      </div>
      <div class="hle-style-row">
        <span class="hle-style-label">{$t('adminDisplayConfigTextSize')}</span>
        <div class="hle-btn-group">
          {#each TEXT_SIZE_OPTIONS as sz}
            <button
              type="button"
              class="hle-choice-btn"
              class:hle-choice-btn--active={(bst.fontSize ?? 'base') === sz.id}
              onclick={() => setStyle(blockId, { fontSize: sz.id })}
            >{sz.label}</button>
          {/each}
        </div>
      </div>
      <div class="hle-style-row">
        <span class="hle-style-label">{$t('adminDisplayConfigTextFont')}</span>
        <div class="hle-btn-group">
          {#each FONT_OPTIONS as f}
            <button
              type="button"
              class="hle-choice-btn"
              class:hle-choice-btn--active={(bst.font ?? '') === f.id}
              style="font-family:{f.family}"
              onclick={() => setStyle(blockId, { font: f.id || undefined })}
            >{f.label}</button>
          {/each}
        </div>
      </div>
      {#if HOME_BLOCK_ELEMENTS[blockId]?.length}
        <div class="hle-el-section">
          <p class="hle-el-heading">{$t('adminHomeLayoutElements')}</p>
          {#each elementRows(blockId) as def (def.id)}
            {@const est = elementStyle(blockId, def.id)}
            {@const range = def.kind === 'media' ? ELEMENT_WIDTH_RANGE : ELEMENT_FONT_RANGE}
            {@const sizeVal = def.kind === 'media' ? est.widthPct : est.sizePx}
            <div class="hle-el-row" class:hle-el-row--hidden={est.hidden}>
              <span class="hle-el-name" class:hle-el-name--hidden={est.hidden}>{ELEMENT_LABELS[def.id] ?? def.id}</span>

              <div class="hle-el-size">
                <input
                  type="range"
                  min={range.min}
                  max={range.max}
                  step="1"
                  value={sizeVal ?? (def.kind === 'media' ? 100 : 18)}
                  oninput={(e) => setElement(blockId, def.id,
                    def.kind === 'media'
                      ? { widthPct: Number((e.target as HTMLInputElement).value) }
                      : { sizePx: Number((e.target as HTMLInputElement).value) })}
                  class="hle-el-slider"
                  title={def.kind === 'media' ? $t('adminHomeLayoutElWidth') : $t('adminHomeLayoutElFontSize')}
                />
                <span class="hle-el-size-val">
                  {#if sizeVal}
                    {sizeVal}{def.kind === 'media' ? '%' : 'px'}
                    <button type="button" class="hle-clear-btn" onclick={() => setElement(blockId, def.id, def.kind === 'media' ? { widthPct: undefined } : { sizePx: undefined })} title={$t('adminDisplayConfigReset')}>×</button>
                  {:else}
                    —
                  {/if}
                </span>
              </div>

              <div class="hle-el-color">
                <input
                  type="color"
                  value={est.color ?? '#34251c'}
                  onchange={(e) => setElement(blockId, def.id, { color: (e.target as HTMLInputElement).value })}
                  class="hle-color-picker hle-color-picker--xs"
                  title={$t('adminDisplayConfigTextColor')}
                />
                {#if est.color}
                  <button type="button" class="hle-clear-btn" onclick={() => setElement(blockId, def.id, { color: undefined })} title={$t('adminDisplayConfigReset')}>×</button>
                {/if}
              </div>

              <button
                type="button"
                class="hle-icon-btn"
                onclick={() => setElement(blockId, def.id, { hidden: !est.hidden })}
                aria-label={est.hidden ? $t('adminHomeLayoutShowBlock') : $t('adminHomeLayoutHideBlock')}
                title={est.hidden ? $t('adminHomeLayoutShowBlock') : $t('adminHomeLayoutHideBlock')}
              >
                {#if est.hidden}
                  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                    <path d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94"/>
                    <path d="M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19"/>
                    <line x1="1" y1="1" x2="23" y2="23"/>
                  </svg>
                {:else}
                  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                    <circle cx="12" cy="12" r="3"/>
                  </svg>
                {/if}
              </button>

              {#if def.orderable}
                {@const order = elementOrderFor(blockId)}
                {@const pos = order.indexOf(def.id)}
                <div class="hle-arrows">
                  <button type="button" class="hle-arrow-btn" onclick={() => moveElement(blockId, def.id, -1)}
                    disabled={pos <= 0} aria-label={$t('adminHomeLayoutMoveUp')}>▲</button>
                  <button type="button" class="hle-arrow-btn" onclick={() => moveElement(blockId, def.id, 1)}
                    disabled={pos === order.length - 1} aria-label={$t('adminHomeLayoutMoveDown')}>▼</button>
                </div>
              {:else}
                <span class="hle-arrows hle-arrows--ghost" aria-hidden="true"></span>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      {#if styled}
        <button type="button" class="hle-reset-btn" onclick={() => clearStyle(blockId)}>
          {$t('adminDisplayConfigReset')}
        </button>
      {/if}
    </li>
  {/if}
{/snippet}

<div class="hle-root">
  <div class="hle-header">
    <div>
      <h2 class="hle-title">{$t('adminHomeLayoutTitle')}</h2>
      <p class="hle-intro">{$t('adminHomeLayoutIntro')}</p>
    </div>
    <div class="hle-actions">
      {#if errorMsg}<span class="hle-error">{errorMsg}</span>{/if}
      {#if savedOk}<span class="hle-saved">{$t('adminHomeLayoutSaved')}</span>{/if}
      <button type="button" class="hle-reset-all" onclick={resetAll}>
        {resetConfirm ? $t('adminHomeLayoutResetConfirm') : $t('adminHomeLayoutReset')}
      </button>
      <button type="button" class="hle-save" onclick={save} disabled={saving || !dirty}>
        {saving ? '…' : $t('adminHomeLayoutSave')}
      </button>
    </div>
  </div>

  {#if loading}
    <p class="hle-loading">…</p>
  {:else}
    <div class="hle-cols">
      <!-- ── Left: the register of blocks ── -->
      <div class="hle-list-col">

        <!-- Named layout presets -->
        <div class="hle-section hle-section--presets">
          <div class="hle-presets-header">
            <p class="hle-section-label">{$t('adminDisplayConfigPresets')}</p>
            {#if !showPresetForm}
              <button type="button" class="hle-preset-save-btn" onclick={() => { presetNameInput = ''; showPresetForm = true; }}>
                + {$t('adminDisplayConfigPresetSave')}
              </button>
            {/if}
          </div>
          {#if presetsLoading}
            <p class="hle-preset-empty">…</p>
          {:else if presets.length > 0}
            <ul class="hle-preset-list">
              {#each presets as p (p.id)}
                <li class="hle-preset-item" class:hle-preset-item--confirm={applyConfirmId === p.id}>
                  <span class="hle-preset-name" title={p.name}>{p.name}</span>
                  <div class="hle-preset-actions">
                    <button
                      type="button"
                      class="hle-preset-apply-btn"
                      class:hle-preset-apply-btn--confirm={applyConfirmId === p.id}
                      onclick={() => applyPreset(p)}
                      title={applyConfirmId === p.id ? $t('adminDisplayConfigPresetConfirm') : $t('adminDisplayConfigPresetApply')}
                    >
                      {#if applyConfirmId === p.id}
                        {$t('adminDisplayConfigPresetConfirm')}
                      {:else}
                        <svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true">
                          <path d="M2 6h8M6 2l4 4-4 4"/>
                        </svg>
                      {/if}
                    </button>
                    <button
                      type="button"
                      class="hle-preset-del-btn"
                      onclick={() => deletePreset(p.id)}
                      title={$t('adminDisplayConfigPresetDelete')}
                      aria-label={$t('adminDisplayConfigPresetDelete')}
                    >×</button>
                  </div>
                </li>
              {/each}
            </ul>
          {:else if !showPresetForm}
            <p class="hle-preset-empty">{$t('adminDisplayConfigPresetEmpty')}</p>
          {/if}
          {#if showPresetForm}
            <div class="hle-preset-form">
              <input
                type="text"
                bind:value={presetNameInput}
                placeholder={$t('adminDisplayConfigPresetNamePlaceholder')}
                class="hle-preset-input"
                onkeydown={(e) => { if (e.key === 'Enter') savePreset(); if (e.key === 'Escape') { showPresetForm = false; presetNameInput = ''; } }}
              />
              <div class="hle-preset-form-btns">
                <button type="button" class="hle-preset-cancel" onclick={() => { showPresetForm = false; presetNameInput = ''; }}>{$t('adminDisplayConfigPresetCancel')}</button>
                <button type="button" class="hle-preset-confirm-save" onclick={savePreset} disabled={!presetNameInput.trim()}>
                  {$t('adminDisplayConfigPresetSaveBtn')}
                </button>
              </div>
            </div>
          {/if}
        </div>

        <!-- Page background -->
        <div class="hle-section">
          <p class="hle-section-label">{$t('adminHomeLayoutPageBg')}</p>
          <div class="hle-color-row">
            <button
              type="button"
              class="hle-swatch hle-swatch--none"
              class:hle-swatch--active={!cfg.pageBackground}
              onclick={() => setPageBackground(undefined)}
              title={$t('adminDisplayConfigFontDefault')}
              aria-label={$t('adminDisplayConfigFontDefault')}
            >×</button>
            {#each HOME_BG_PRESETS as bg}
              <button
                type="button"
                class="hle-swatch"
                class:hle-swatch--active={cfg.pageBackground === bg.color}
                style="background:{bg.color}"
                onclick={() => setPageBackground(bg.color)}
                title={BG_SWATCH_LABELS[bg.id]}
                aria-label={BG_SWATCH_LABELS[bg.id]}
              ></button>
            {/each}
            <input
              type="color"
              value={cfg.pageBackground ?? '#f8f1e7'}
              onchange={(e) => setPageBackground((e.target as HTMLInputElement).value)}
              class="hle-color-picker"
              title={$t('adminDisplayConfigCustomColor')}
            />
            {#if cfg.pageBackground}
              <span class="hle-color-hex">{cfg.pageBackground}</span>
            {/if}
          </div>
        </div>

        <!-- THE COLLECTION card scroll-reveal effect -->
        <div class="hle-section">
          <p class="hle-section-label">Эффект карточек THE COLLECTION</p>
          <div class="hle-color-row">
            {#each CARD_EFFECT_OPTIONS as opt (opt.id)}
              <button
                type="button"
                class="hle-effect-btn"
                class:hle-effect-btn--active={(cfg.cardEffect ?? 'rise') === opt.id}
                onclick={() => setCardEffect(opt.id)}
              >{opt.label}</button>
            {/each}
          </div>
        </div>

        <p class="hle-note">{$t('adminHomeLayoutRuntimeNoteLegend')}</p>
        <ol class="hle-list">
          {#each mainOrder as blockId, i (blockId)}
            {@render blockRow('main', blockId, i, mainOrder.length, false)}
            {#if blockId === 'returningBand'}
              <ol class="hle-sublist">
                {#each bandOrder as subId, j (subId)}
                  {@render blockRow('band', subId, j, bandOrder.length, true)}
                {/each}
              </ol>
            {:else if blockId === 'latelyShelves'}
              <ol class="hle-sublist">
                {#each shelfOrder as subId, j (subId)}
                  {@render blockRow('shelf', subId, j, shelfOrder.length, true)}
                {/each}
              </ol>
            {/if}
          {/each}
        </ol>
      </div>

      <!-- ── Right: live preview ── -->
      <div class="hle-preview-col">
        <div class="hle-preview-bar">
          <div class="hle-bp-group" role="group" aria-label={$t('adminHomeLayoutPreview')}>
            {#each BREAKPOINTS as b}
              <button
                type="button"
                class="hle-bp-btn"
                class:hle-bp-btn--active={bp === b.id}
                onclick={() => (bp = b.id)}
              >{b.label}</button>
            {/each}
          </div>
          <div class="hle-bp-group" role="group" aria-label={$t('adminHomeLayoutVisitorMode')}>
            <button
              type="button"
              class="hle-bp-btn"
              class:hle-bp-btn--active={visitorMode === 'new'}
              onclick={() => setVisitorMode('new')}
            >{$t('adminHomeLayoutVisitorNew')}</button>
            <button
              type="button"
              class="hle-bp-btn"
              class:hle-bp-btn--active={visitorMode === 'returning'}
              onclick={() => setVisitorMode('returning')}
            >{$t('adminHomeLayoutVisitorReturning')}</button>
          </div>
        </div>

        <div class="hle-preview-stage" bind:clientWidth={previewContainerWidth} style="height:{PREVIEW_VIEW_HEIGHT + 2}px">
          <div
            class="hle-preview-frame"
            style="width:{bpWidth}px; height:{iframeHeight}px; transform:scale({previewScale});"
          >
            <iframe
              bind:this={iframeEl}
              src="/?hlPreview=1"
              title={$t('adminHomeLayoutPreview')}
              onload={onIframeLoad}
            ></iframe>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .hle-root {
    border: 1px solid #d8c6b1;
    border-radius: 3px;
    background: color-mix(in srgb, #f8f1e7 80%, white);
    padding: 1.25rem;
  }

  .hle-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
  }
  .hle-title {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.4rem;
    color: #34251c;
    margin: 0 0 0.25rem;
  }
  .hle-intro {
    font-size: 0.78rem;
    color: #6f3b24;
    max-width: 52ch;
    margin: 0;
  }
  .hle-actions {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex-shrink: 0;
  }
  .hle-save {
    padding: 6px 16px;
    border: 1px solid #c65f3c;
    border-radius: 2px;
    background: #c65f3c;
    color: #fff;
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    cursor: pointer;
  }
  .hle-save:disabled { opacity: 0.4; cursor: default; }
  .hle-save:hover:not(:disabled) { background: #b05535; }
  .hle-reset-all {
    font-size: 0.72rem;
    color: #6f3b24;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .hle-saved { font-size: 0.72rem; color: #6b8a56; }
  .hle-error { font-size: 0.72rem; color: #c0392b; }
  .hle-loading { font-size: 0.8rem; color: #6f3b24; }

  .hle-cols {
    display: grid;
    grid-template-columns: minmax(320px, 420px) minmax(0, 1fr);
    gap: 1.25rem;
    align-items: start;
  }
  @media (max-width: 1100px) {
    .hle-cols { grid-template-columns: 1fr; }
  }

  .hle-note {
    font-size: 0.68rem;
    font-style: italic;
    color: color-mix(in srgb, #6f3b24 75%, transparent);
    margin: 0 0 0.5rem;
  }

  .hle-list, .hle-sublist {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .hle-sublist {
    margin: 0.25rem 0 0.25rem 1.4rem;
    padding-left: 0.6rem;
    border-left: 1px solid #d8c6b1;
  }

  .hle-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.3rem 0.5rem;
    border: 1px solid #d8c6b1;
    border-radius: 3px;
    background: color-mix(in srgb, #f8f1e7 70%, white);
    font-size: 0.78rem;
    cursor: grab;
  }
  .hle-row--dragging { opacity: 0.45; border-style: dashed; }
  .hle-row--hidden { opacity: 0.6; }
  .hle-grip {
    color: #b09a80;
    font-size: 0.8rem;
    line-height: 1;
    flex-shrink: 0;
    cursor: grab;
  }
  .hle-name {
    flex: 1;
    color: #34251c;
    min-width: 0;
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .hle-name:hover { color: #c65f3c; }
  .hle-name--hidden { text-decoration: line-through; opacity: 0.5; }
  .hle-auto-note {
    color: #8b6a45;
    font-size: 0.7rem;
    cursor: help;
    margin-left: 2px;
  }

  .hle-icon-btn {
    padding: 2px 4px;
    border: 1px solid #d8c6b1;
    border-radius: 2px;
    background: none;
    cursor: pointer;
    color: #6f3b24;
    line-height: 1;
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }
  .hle-icon-btn:hover { background: #e8ddd0; }
  .hle-icon-btn--active { color: #c65f3c; border-color: #c65f3c; }

  .hle-size-group { display: flex; gap: 1px; flex-shrink: 0; }
  .hle-size-btn {
    padding: 1px 5px;
    font-size: 0.7rem;
    line-height: 1.5;
    border: 1px solid #d8c6b1;
    background: none;
    cursor: pointer;
    color: #6f3b24;
  }
  .hle-size-btn:hover { background: #e8ddd0; }
  .hle-size-btn--active {
    background: #c65f3c;
    border-color: #c65f3c;
    color: #fff;
  }

  .hle-arrows { display: flex; flex-direction: column; gap: 1px; flex-shrink: 0; }
  .hle-arrow-btn {
    padding: 0 3px;
    font-size: 0.55rem;
    line-height: 1.3;
    border: 1px solid #d8c6b1;
    border-radius: 2px;
    background: none;
    cursor: pointer;
    color: #34251c;
  }
  .hle-arrow-btn:hover:not(:disabled) { background: #e8ddd0; }
  .hle-arrow-btn:disabled { opacity: 0.3; cursor: default; }

  /* ── Style panel ── */
  .hle-style-panel {
    list-style: none;
    padding: 0.5rem 0.5rem 0.4rem;
    border: 1px solid #d8c6b1;
    border-top: none;
    border-radius: 0 0 3px 3px;
    background: color-mix(in srgb, #f8f1e7 60%, white);
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin-top: -0.25rem;
  }
  .hle-style-panel--sub { margin-left: 0; }
  .hle-style-row { display: flex; align-items: center; gap: 0.4rem; flex-wrap: wrap; }
  .hle-style-label {
    font-size: 0.65rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: #6f3b24;
    min-width: 3.2rem;
    flex-shrink: 0;
  }
  .hle-style-hint { font-size: 0.65rem; color: #6f3b24; opacity: 0.6; }
  .hle-color-row { display: flex; align-items: center; gap: 0.4rem; flex-wrap: wrap; }
  .hle-swatch {
    width: 22px;
    height: 22px;
    border: 2px solid transparent;
    border-radius: 3px;
    box-shadow: inset 0 0 0 1px rgba(52, 37, 28, 0.25);
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
  }
  .hle-swatch--active { border-color: #c65f3c; }
  .hle-swatch--none {
    background: none;
    color: #6f3b24;
    font-size: 0.7rem;
    line-height: 1;
  }

  .hle-effect-btn {
    padding: 5px 11px;
    border: 1px solid rgba(52, 37, 28, 0.25);
    border-radius: 999px;
    background: transparent;
    color: #6f3b24;
    font-size: 0.68rem;
    font-weight: 600;
    letter-spacing: 0.02em;
    cursor: pointer;
    transition: color 0.2s ease, border-color 0.2s ease, background 0.2s ease;
  }
  .hle-effect-btn:hover { border-color: #c65f3c; color: #c65f3c; }
  .hle-effect-btn--active {
    background: #c65f3c;
    border-color: #c65f3c;
    color: #fff7ea;
  }

  .hle-section { margin-bottom: 0.9rem; }
  .hle-section-label {
    font-size: 0.7rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #6f3b24;
    margin: 0 0 0.4rem;
  }
  .hle-section--presets {
    border-bottom: 1px solid #d8c6b1;
    padding-bottom: 0.7rem;
  }
  .hle-presets-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.3rem;
  }
  .hle-presets-header .hle-section-label { margin: 0; }
  .hle-preset-save-btn {
    font-size: 0.65rem;
    color: #c65f3c;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    letter-spacing: 0.04em;
  }
  .hle-preset-save-btn:hover { text-decoration: underline; text-underline-offset: 2px; }
  .hle-preset-list {
    list-style: none;
    margin: 0 0 0.3rem;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .hle-preset-item {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0.4rem;
    border: 1px solid #d8c6b1;
    border-radius: 3px;
    background: color-mix(in srgb, #f8f1e7 70%, white);
  }
  .hle-preset-item--confirm { border-color: #c65f3c; }
  .hle-preset-name {
    flex: 1;
    font-size: 0.72rem;
    color: #34251c;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hle-preset-actions { display: flex; gap: 2px; flex-shrink: 0; }
  .hle-preset-apply-btn {
    display: flex;
    align-items: center;
    padding: 1px 5px;
    font-size: 0.62rem;
    border: 1px solid #d8c6b1;
    border-radius: 2px;
    background: none;
    cursor: pointer;
    color: #6f3b24;
    line-height: 1.5;
  }
  .hle-preset-apply-btn:hover { background: #e8ddd0; }
  .hle-preset-apply-btn--confirm {
    background: #c65f3c;
    border-color: #c65f3c;
    color: #fff;
    font-weight: 600;
  }
  .hle-preset-del-btn {
    padding: 1px 4px;
    font-size: 0.72rem;
    border: 1px solid transparent;
    background: none;
    cursor: pointer;
    color: #6f3b24;
    opacity: 0.45;
  }
  .hle-preset-del-btn:hover { opacity: 1; color: #c0392b; }
  .hle-preset-empty {
    font-size: 0.68rem;
    color: #6f3b24;
    opacity: 0.5;
    font-style: italic;
    margin: 0 0 0.25rem;
  }
  .hle-preset-form { display: flex; flex-direction: column; gap: 0.3rem; margin-top: 0.3rem; }
  .hle-preset-input {
    width: 100%;
    font-size: 0.75rem;
    padding: 0.3rem 0.4rem;
    border: 1px solid #d8c6b1;
    border-radius: 3px;
    background: #fffdf9;
    color: #34251c;
  }
  .hle-preset-form-btns { display: flex; justify-content: flex-end; gap: 0.35rem; }
  .hle-preset-cancel {
    font-size: 0.68rem;
    color: #6f3b24;
    background: none;
    border: none;
    padding: 2px 5px;
    cursor: pointer;
    opacity: 0.7;
  }
  .hle-preset-cancel:hover { opacity: 1; }
  .hle-preset-confirm-save {
    font-size: 0.68rem;
    padding: 2px 8px;
    border: 1px solid #c65f3c;
    border-radius: 2px;
    background: #c65f3c;
    color: #fff;
    cursor: pointer;
    font-weight: 600;
  }
  .hle-preset-confirm-save:hover:not(:disabled) { background: #b05535; }
  .hle-preset-confirm-save:disabled { opacity: 0.4; cursor: default; }
  .hle-color-picker {
    width: 24px; height: 24px;
    border: 1px solid #d8c6b1;
    border-radius: 3px;
    padding: 1px;
    cursor: pointer;
  }
  .hle-color-hex { font-size: 0.65rem; font-family: monospace; color: #34251c; }
  .hle-clear-btn {
    font-size: 0.7rem;
    padding: 0 3px;
    border: none;
    background: none;
    cursor: pointer;
    color: #6f3b24;
    opacity: 0.6;
    line-height: 1;
  }
  .hle-clear-btn:hover { opacity: 1; }
  .hle-btn-group { display: flex; gap: 2px; flex-wrap: wrap; }
  .hle-choice-btn {
    padding: 1px 5px;
    font-size: 0.65rem;
    border: 1px solid #d8c6b1;
    border-radius: 2px;
    background: none;
    cursor: pointer;
    color: #34251c;
    line-height: 1.6;
  }
  .hle-choice-btn:hover { background: #e8ddd0; }
  .hle-choice-btn--active {
    background: #c65f3c;
    border-color: #c65f3c;
    color: #fff;
  }
  .hle-reset-btn {
    margin-top: 0.25rem;
    font-size: 0.68rem;
    color: #6f3b24;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 2px;
    align-self: flex-start;
  }

  /* ── Element rows (fine-grained per-element overrides) ── */
  .hle-el-section {
    margin-top: 0.4rem;
    padding-top: 0.4rem;
    border-top: 1px dashed #d8c6b1;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .hle-el-heading {
    font-size: 0.62rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #6f3b24;
    margin: 0 0 0.15rem;
  }
  .hle-el-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 2px 0;
  }
  .hle-el-row--hidden { opacity: 0.55; }
  .hle-el-name {
    flex: 0 0 7.5rem;
    font-size: 0.7rem;
    color: #34251c;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hle-el-name--hidden { text-decoration: line-through; opacity: 0.6; }
  .hle-el-size {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex: 1;
    min-width: 0;
  }
  .hle-el-slider {
    flex: 1;
    min-width: 60px;
    max-width: 130px;
    height: 14px;
    accent-color: #c65f3c;
    cursor: pointer;
  }
  .hle-el-size-val {
    font-size: 0.62rem;
    font-family: monospace;
    color: #6f3b24;
    min-width: 3.4rem;
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  .hle-el-color { display: flex; align-items: center; gap: 2px; flex-shrink: 0; }
  .hle-color-picker--xs { width: 20px; height: 20px; padding: 1px; }
  .hle-arrows--ghost { width: 17px; }

  /* ── Preview ── */
  .hle-preview-col { min-width: 0; }
  .hle-preview-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
    flex-wrap: wrap;
  }
  .hle-bp-group { display: flex; gap: 1px; }
  .hle-bp-btn {
    padding: 3px 10px;
    font-size: 0.68rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    border: 1px solid #d8c6b1;
    background: none;
    cursor: pointer;
    color: #6f3b24;
  }
  .hle-bp-btn:hover { background: #e8ddd0; }
  .hle-bp-btn--active {
    background: #34251c;
    border-color: #34251c;
    color: #f8f1e7;
  }

  .hle-preview-stage {
    position: relative;
    overflow: hidden;
    border: 1px solid #d8c6b1;
    border-radius: 3px;
    background: #efe6d8;
    display: flex;
    justify-content: center;
  }
  .hle-preview-frame {
    transform-origin: top center;
    flex-shrink: 0;
    box-shadow: 0 2px 12px rgba(52, 37, 28, 0.18);
  }
  .hle-preview-frame iframe {
    width: 100%;
    height: 100%;
    border: none;
    background: #f8f1e7;
    display: block;
  }
</style>
