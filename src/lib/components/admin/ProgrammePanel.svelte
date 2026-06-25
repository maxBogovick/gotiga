<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { ProgrammeSettings } from '$lib/types/api';

  // ── Background presets ──────────────────────────────────────────────────────
  interface BgPreset {
    id: string;
    label: string;
    value: string;
  }

  const PRESETS: BgPreset[] = [
    {
      id: 'vellum',
      label: 'Deep Vellum',
      value:
        'radial-gradient(ellipse at 30% 15%, rgba(160,110,50,0.12) 0%, transparent 55%), radial-gradient(ellipse at 75% 85%, rgba(80,45,15,0.18) 0%, transparent 50%), linear-gradient(170deg, #2e2014 0%, #241a0e 55%, #1c1208 100%)',
    },
    {
      id: 'slate',
      label: 'Burnt Slate',
      value: 'linear-gradient(160deg, #1e1812 0%, #2a211a 50%, #1a1410 100%)',
    },
    {
      id: 'vitrine',
      label: 'Museum Vitrine',
      value: 'linear-gradient(175deg, #161f18 0%, #0f1a12 40%, #0a1209 100%)',
    },
    {
      id: 'parchment',
      label: 'Light Parchment',
      value: 'linear-gradient(180deg, #efe4d2, #e7dac3)',
    },
    {
      id: 'custom',
      label: 'Custom colour',
      value: '',
    },
  ];

  // ── Helpers ─────────────────────────────────────────────────────────────────

  /** Build a CSS gradient from a single base hex colour. */
  function buildCustomBg(hex: string): string {
    return [
      `radial-gradient(ellipse at 28% 18%, ${hexToRgba(hex, 0.45)} 0%, transparent 52%)`,
      `radial-gradient(ellipse at 72% 82%, ${hexToRgba(hex, 0.25)} 0%, transparent 48%)`,
      `linear-gradient(170deg, ${darken(hex, 0.28)} 0%, ${darken(hex, 0.38)} 55%, ${darken(hex, 0.46)} 100%)`,
    ].join(', ');
  }

  function hexToRgba(hex: string, a: number): string {
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return `rgba(${r},${g},${b},${a.toFixed(2)})`;
  }

  function darken(hex: string, amount: number): string {
    const r = Math.round(parseInt(hex.slice(1, 3), 16) * (1 - amount));
    const g = Math.round(parseInt(hex.slice(3, 5), 16) * (1 - amount));
    const b = Math.round(parseInt(hex.slice(5, 7), 16) * (1 - amount));
    return `rgb(${r},${g},${b})`;
  }

  // ── Bronze molding — derived from one base tone (mirrors HouseNoticeBoard) ───
  function shade(hex: string, amount: number): string {
    const h = hex.replace('#', '');
    const n = h.length === 3 ? h.split('').map((c) => c + c).join('') : h;
    const r = parseInt(n.slice(0, 2), 16);
    const g = parseInt(n.slice(2, 4), 16);
    const b = parseInt(n.slice(4, 6), 16);
    const adj = (v: number) =>
      Math.max(0, Math.min(255, Math.round(amount >= 0 ? v + (255 - v) * amount : v * (1 + amount))));
    return `rgb(${adj(r)},${adj(g)},${adj(b)})`;
  }
  const BUILTIN_BRONZE =
    'linear-gradient(146deg, #e9d199 0%, #c39d59 14%, #8a6738 32%, #4a341c 52%, #2c1d0f 64%, #6e4f2a 82%, #cba869 100%)';
  function buildFrameGradient(hex: string): string {
    return `linear-gradient(146deg, ${shade(hex, 0.42)} 0%, ${shade(hex, 0.14)} 14%, ${shade(hex, -0.18)} 32%, ${shade(hex, -0.58)} 52%, ${shade(hex, -0.74)} 64%, ${shade(hex, -0.34)} 82%, ${shade(hex, 0.3)} 100%)`;
  }

  interface FrameTone {
    id: string;
    label: string;
    hex: string | null; // null = built-in bronze
  }
  const FRAME_TONES: FrameTone[] = [
    { id: 'bronze', label: 'Bronze', hex: null },
    { id: 'gold', label: 'Old Gold', hex: '#c9a14f' },
    { id: 'patina', label: 'Verdigris', hex: '#6f8a5e' },
    { id: 'pewter', label: 'Pewter', hex: '#8c8a86' },
    { id: 'custom', label: 'Custom', hex: null },
  ];

  // ── State ────────────────────────────────────────────────────────────────────

  const EMPTY: ProgrammeSettings = {
    maxShowings: 0,
    caseBg: null,
    curatorNoteEn: null,
    curatorNoteRu: null,
    curatorSignEn: null,
    curatorSignRu: null,
    frameTone: null,
    frameThickness: null,
    frameMode: null,
  };

  const FRAME_MODES = [
    { id: 'gradient', label: 'Beveled' },
    { id: 'flat', label: 'Flat' },
    { id: 'none', label: 'None' },
  ] as const;

  let settings = $state<ProgrammeSettings>({ ...EMPTY });
  let savedSnapshot = $state('');
  let isLoading = $state(true);
  let isSaving = $state(false);
  let saveMsg = $state<'ok' | 'err' | null>(null);

  let customColor = $state('#2e2014');
  let activePresetId = $state('vellum');

  let customTone = $state('#b88f4f');
  let activeToneId = $state('bronze');

  let hasUnsaved = $derived(JSON.stringify(settings) !== savedSnapshot);

  const previewBg = $derived(
    settings.caseBg?.trim() || PRESETS.find((p) => p.id === 'vellum')!.value
  );

  const frameMode = $derived(settings.frameMode || 'gradient');

  // The molding fill shown in the live preview (and what gets saved).
  const previewFrame = $derived.by(() => {
    const tone = settings.frameTone?.trim();
    if (frameMode === 'flat') return tone ? shade(tone, -0.18) : '#8a6738';
    return tone ? buildFrameGradient(tone) : BUILTIN_BRONZE;
  });

  function selectMode(id: string) {
    settings = { ...settings, frameMode: id === 'gradient' ? null : id };
  }
  const previewThickness = $derived(
    settings.frameThickness != null && settings.frameThickness > 0
      ? `${settings.frameThickness}px`
      : '14px'
  );
  const isAutoThickness = $derived(settings.frameThickness == null);

  function detectTone(hex: string | null): string {
    if (!hex) return 'bronze';
    const match = FRAME_TONES.find((tone) => tone.hex && tone.hex === hex);
    return match ? match.id : 'custom';
  }

  function selectTone(tone: FrameTone) {
    activeToneId = tone.id;
    if (tone.id === 'bronze') {
      settings = { ...settings, frameTone: null };
    } else if (tone.id === 'custom') {
      settings = { ...settings, frameTone: customTone };
    } else {
      settings = { ...settings, frameTone: tone.hex };
    }
  }

  function onCustomToneChange(e: Event) {
    customTone = (e.target as HTMLInputElement).value;
    if (activeToneId === 'custom') {
      settings = { ...settings, frameTone: customTone };
    }
  }

  function onThicknessInput(e: Event) {
    settings = { ...settings, frameThickness: Number((e.target as HTMLInputElement).value) };
  }

  function setAutoThickness(auto: boolean) {
    settings = { ...settings, frameThickness: auto ? null : 14 };
  }

  function detectPreset(bg: string | null): string {
    if (!bg) return 'vellum';
    const match = PRESETS.find((p) => p.id !== 'custom' && p.value === bg);
    return match ? match.id : 'custom';
  }

  function selectPreset(preset: BgPreset) {
    activePresetId = preset.id;
    if (preset.id === 'vellum') {
      settings = { ...settings, caseBg: null };
    } else if (preset.id === 'custom') {
      settings = { ...settings, caseBg: buildCustomBg(customColor) };
    } else {
      settings = { ...settings, caseBg: preset.value };
    }
  }

  function onCustomColorChange(e: Event) {
    customColor = (e.target as HTMLInputElement).value;
    if (activePresetId === 'custom') {
      settings = { ...settings, caseBg: buildCustomBg(customColor) };
    }
  }

  onMount(async () => {
    try {
      const loaded = await api.getProgrammeSettings().catch(() => null);
      if (loaded) {
        settings = { ...EMPTY, ...loaded };
        activePresetId = detectPreset(settings.caseBg);
        activeToneId = detectTone(settings.frameTone);
        if (activeToneId === 'custom' && settings.frameTone) customTone = settings.frameTone;
      }
    } finally {
      isLoading = false;
      savedSnapshot = JSON.stringify(settings);
    }
  });

  async function save() {
    isSaving = true;
    saveMsg = null;
    try {
      const saved = await api.saveProgrammeSettings(settings);
      settings = { ...EMPTY, ...saved };
      savedSnapshot = JSON.stringify(settings);
      saveMsg = 'ok';
      setTimeout(() => { saveMsg = null; }, 3000);
    } catch {
      saveMsg = 'err';
    } finally {
      isSaving = false;
    }
  }
</script>

<div class="wrap">
  {#if isLoading}
    <div class="loading">Loading…</div>
  {:else}
    <!-- ── Display ─────────────────────────────────────────────── -->
    <section class="section">
      <h3 class="section-title">Display</h3>
      <div class="field-row">
        <label class="field" for="max-showings">
          <span class="label">Max showings visible</span>
          <div class="input-with-hint">
            <input
              id="max-showings"
              class="input narrow"
              type="number"
              min="0"
              max="20"
              bind:value={settings.maxShowings}
            />
            <span class="hint">
              {settings.maxShowings === 0
                ? 'All eligible showings are shown'
                : `Only the ${settings.maxShowings} nearest showing${settings.maxShowings === 1 ? '' : 's'} are shown`}
            </span>
          </div>
        </label>
      </div>
    </section>

    <!-- ── Panel background ───────────────────────────────────── -->
    <section class="section">
      <h3 class="section-title">Panel background</h3>

      <!-- Preview — composites panel background, molding tone and thickness -->
      <div
        class="bg-preview"
        class:framed={frameMode !== 'none'}
        style="background: {previewBg}; --frame-gradient: {previewFrame}; --frame-thickness: {previewThickness};"
        aria-hidden="true"
      >
        <div class="bg-preview-inner">
          <span class="bg-preview-eyebrow">PROGRAMME</span>
          <span class="bg-preview-title">Opening soon</span>
          <div class="bg-preview-screws">
            <span></span><span></span><span></span><span></span>
          </div>
        </div>
      </div>

      <!-- Preset swatches -->
      <div class="presets">
        {#each PRESETS as preset (preset.id)}
          <button
            class="preset-btn"
            class:active={activePresetId === preset.id}
            onclick={() => selectPreset(preset)}
            title={preset.label}
          >
            {#if preset.id === 'custom'}
              <span class="preset-swatch custom-swatch" style="background: {buildCustomBg(customColor)};">
                <span class="custom-swatch-icon">✎</span>
              </span>
            {:else}
              <span class="preset-swatch" style="background: {preset.value};"></span>
            {/if}
            <span class="preset-label">{preset.label}</span>
          </button>
        {/each}
      </div>

      <!-- Custom colour picker — shown only when Custom preset is active -->
      {#if activePresetId === 'custom'}
        <div class="custom-row">
          <label class="custom-label" for="custom-color">Base colour</label>
          <input
            id="custom-color"
            type="color"
            class="color-picker"
            value={customColor}
            oninput={onCustomColorChange}
          />
          <span class="custom-hint">A gradient is generated automatically from this colour.</span>
        </div>
      {/if}
    </section>

    <!-- ── Case frame (bronze molding) ─────────────────────────── -->
    <section class="section">
      <h3 class="section-title">Case frame</h3>
      <p class="section-hint">The cast-bronze molding around the programme case.</p>

      <!-- Mode: beveled gradient · flat solid · no frame -->
      <div class="mode-row">
        {#each FRAME_MODES as mode (mode.id)}
          <button
            class="mode-btn"
            class:active={frameMode === mode.id}
            onclick={() => selectMode(mode.id)}
          >{mode.label}</button>
        {/each}
      </div>

      {#if frameMode !== 'none'}
      <!-- Tone swatches -->
      <div class="presets">
        {#each FRAME_TONES as tone (tone.id)}
          <button
            class="preset-btn"
            class:active={activeToneId === tone.id}
            onclick={() => selectTone(tone)}
            title={tone.label}
          >
            {#if tone.id === 'custom'}
              <span class="preset-swatch custom-swatch" style="background: {buildFrameGradient(customTone)};">
                <span class="custom-swatch-icon">✎</span>
              </span>
            {:else}
              <span class="preset-swatch" style="background: {tone.hex ? buildFrameGradient(tone.hex) : BUILTIN_BRONZE};"></span>
            {/if}
            <span class="preset-label">{tone.label}</span>
          </button>
        {/each}
      </div>

      {#if activeToneId === 'custom'}
        <div class="custom-row">
          <label class="custom-label" for="frame-color">Base tone</label>
          <input
            id="frame-color"
            type="color"
            class="color-picker"
            value={customTone}
            oninput={onCustomToneChange}
          />
          <span class="custom-hint">The light→dark bevel is generated from this colour.</span>
        </div>
      {/if}

      <!-- Thickness -->
      <div class="thickness-row">
        <span class="label">Thickness</span>
        <label class="auto-toggle">
          <input
            type="checkbox"
            checked={isAutoThickness}
            onchange={(e) => setAutoThickness((e.target as HTMLInputElement).checked)}
          />
          Automatic
        </label>
        {#if !isAutoThickness}
          <input
            class="thickness-range"
            type="range"
            min="8"
            max="26"
            step="1"
            value={settings.frameThickness ?? 14}
            oninput={onThicknessInput}
          />
          <span class="hint">{settings.frameThickness ?? 14}px</span>
        {/if}
      </div>
      {/if}
    </section>

    <!-- ── Keeper's note ──────────────────────────────────────── -->
    <section class="section">
      <h3 class="section-title">Keeper's note</h3>
      <p class="section-hint">Leave blank to use the default text.</p>
      <div class="bilingual">
        <label class="field" for="note-en">
          <span class="label">Note (EN)</span>
          <textarea
            id="note-en"
            class="input textarea"
            rows="3"
            placeholder="The museum is preparing a few showings. None can be hurried — each opens in its own hour."
            bind:value={settings.curatorNoteEn}
          ></textarea>
        </label>
        <label class="field" for="note-ru">
          <span class="label">Note (RU)</span>
          <textarea
            id="note-ru"
            class="input textarea"
            rows="3"
            placeholder="Музей готовит несколько показов. Их нельзя поторопить — каждый откроется в свой час."
            bind:value={settings.curatorNoteRu}
          ></textarea>
        </label>
      </div>
      <div class="bilingual">
        <label class="field" for="sign-en">
          <span class="label">Signature (EN)</span>
          <input id="sign-en" class="input" type="text" placeholder="— THE KEEPER" bind:value={settings.curatorSignEn} />
        </label>
        <label class="field" for="sign-ru">
          <span class="label">Signature (RU)</span>
          <input id="sign-ru" class="input" type="text" placeholder="— СМОТРИТЕЛЬ" bind:value={settings.curatorSignRu} />
        </label>
      </div>
    </section>

    <!-- ── Save bar ───────────────────────────────────────────── -->
    <div class="save-bar">
      {#if saveMsg === 'ok'}
        <span class="msg ok">Saved</span>
      {:else if saveMsg === 'err'}
        <span class="msg err">Error — changes not saved</span>
      {:else if hasUnsaved}
        <span class="msg unsaved">Unsaved changes</span>
      {/if}
      <button class="btn-save" onclick={save} disabled={isSaving || !hasUnsaved}>
        {isSaving ? 'Saving…' : 'Save'}
      </button>
    </div>
  {/if}
</div>

<style>
  .wrap {
    height: 100%;
    overflow-y: auto;
    padding: 28px 32px 40px;
    max-width: 800px;
  }

  .loading {
    padding: 32px;
    font-size: 14px;
    color: var(--color-ink-tertiary);
  }

  /* ── Sections ──────────────────────────────────────────────── */
  .section {
    margin-bottom: 36px;
  }
  .section-title {
    margin: 0 0 16px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--color-border, #d8c6b1);
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary, #9a7e68);
  }
  .section-hint {
    margin: -8px 0 16px;
    font-size: 13px;
    color: var(--color-ink-tertiary, #9a7e68);
  }

  /* ── Fields ─────────────────────────────────────────────────── */
  .field-row { display: flex; flex-direction: column; gap: 12px; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .label {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--color-ink-secondary, #5f4636);
  }
  .input-with-hint { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .hint { font-size: 12px; color: var(--color-ink-tertiary, #9a7e68); }
  .input {
    padding: 7px 10px;
    border: 1px solid var(--color-border, #d8c6b1);
    border-radius: 4px;
    background: var(--color-surface, #fff);
    font-size: 14px;
    color: var(--color-ink-primary, #34251c);
    font-family: inherit;
    width: 100%;
    box-sizing: border-box;
  }
  .input.narrow { width: 80px; }
  .textarea { resize: vertical; }
  .bilingual {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    margin-bottom: 14px;
  }

  /* ── Background preview ─────────────────────────────────────── */
  .bg-preview {
    position: relative;
    height: 110px;
    border-radius: 6px;
    margin-bottom: 18px;
    overflow: hidden;
    border: 1px solid rgba(0,0,0,0.15);
    transition: background 0.4s ease;
  }
  /* the bronze molding ring drawn over the preview (mirrors HouseNoticeBoard) */
  .bg-preview.framed::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: 2;
    border-radius: 6px;
    padding: var(--frame-thickness, 14px);
    background: var(--frame-gradient);
    -webkit-mask:
      linear-gradient(#000 0 0) content-box,
      linear-gradient(#000 0 0);
    -webkit-mask-composite: xor;
    mask-composite: exclude;
    box-shadow:
      inset 0 1px 0 rgba(255, 240, 200, 0.55),
      inset 0 -1px 0 rgba(0, 0, 0, 0.55),
      inset 0 0 0 1px rgba(0, 0, 0, 0.35);
    pointer-events: none;
  }
  .bg-preview-inner {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 0 20px;
    gap: 4px;
  }
  .bg-preview-eyebrow {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.22em;
    color: rgba(200,170,110,0.7);
  }
  .bg-preview-title {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 20px;
    font-weight: 600;
    color: rgba(220,195,155,0.9);
  }
  .bg-preview-screws {
    position: absolute;
    inset: 8px;
    pointer-events: none;
  }
  .bg-preview-screws span {
    position: absolute;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: radial-gradient(circle at 35% 32%, #c0a983, #6d5839 70%);
    box-shadow: inset 0 1px 0 rgba(255,255,255,0.4);
  }
  .bg-preview-screws span:nth-child(1) { top: 0; left: 0; }
  .bg-preview-screws span:nth-child(2) { top: 0; right: 0; }
  .bg-preview-screws span:nth-child(3) { bottom: 0; left: 0; }
  .bg-preview-screws span:nth-child(4) { bottom: 0; right: 0; }

  /* ── Preset swatches ─────────────────────────────────────────── */
  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-bottom: 16px;
  }
  .preset-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 0;
    background: none;
    border: none;
    cursor: pointer;
  }
  .preset-swatch {
    display: block;
    width: 56px;
    height: 40px;
    border-radius: 5px;
    border: 2px solid transparent;
    box-shadow: 0 1px 4px rgba(0,0,0,0.18);
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .preset-btn.active .preset-swatch {
    border-color: var(--color-accent, #c65f3c);
    box-shadow: 0 0 0 3px rgba(198,95,60,0.22);
  }
  .preset-label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--color-ink-secondary, #5f4636);
    white-space: nowrap;
  }
  .custom-swatch {
    position: relative;
  }
  .custom-swatch-icon {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 16px;
    color: rgba(255,255,255,0.7);
  }

  /* ── Custom colour row ───────────────────────────────────────── */
  .custom-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    background: var(--color-surface, #fff);
    border: 1px solid var(--color-border, #d8c6b1);
    border-radius: 6px;
  }
  .custom-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-ink-secondary, #5f4636);
    white-space: nowrap;
  }
  .color-picker {
    width: 40px;
    height: 32px;
    padding: 2px;
    border: 1px solid var(--color-border, #d8c6b1);
    border-radius: 4px;
    background: none;
    cursor: pointer;
    flex-shrink: 0;
  }
  .custom-hint {
    font-size: 12px;
    color: var(--color-ink-tertiary, #9a7e68);
  }

  /* ── Frame mode segmented control ────────────────────────────── */
  .mode-row {
    display: inline-flex;
    gap: 0;
    margin-bottom: 16px;
    border: 1px solid var(--color-border, #d8c6b1);
    border-radius: 6px;
    overflow: hidden;
  }
  .mode-btn {
    padding: 7px 16px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    background: var(--color-surface, #fff);
    color: var(--color-ink-secondary, #5f4636);
    border: none;
    border-left: 1px solid var(--color-border, #d8c6b1);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .mode-btn:first-child { border-left: none; }
  .mode-btn.active {
    background: var(--color-accent, #c65f3c);
    color: #fff;
  }

  /* ── Thickness row ───────────────────────────────────────────── */
  .thickness-row {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
    margin-top: 4px;
  }
  .auto-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--color-ink-secondary, #5f4636);
    cursor: pointer;
  }
  .thickness-range {
    flex: 1;
    min-width: 140px;
    max-width: 260px;
    accent-color: var(--color-accent, #c65f3c);
  }

  /* ── Save bar ────────────────────────────────────────────────── */
  .save-bar {
    display: flex;
    align-items: center;
    gap: 16px;
    padding-top: 12px;
    border-top: 1px solid var(--color-border, #d8c6b1);
    margin-top: 4px;
  }
  .msg {
    font-size: 13px;
  }
  .msg.ok   { color: #2d6a3f; }
  .msg.err  { color: #b91c1c; }
  .msg.unsaved { color: var(--color-ink-tertiary, #9a7e68); }
  .btn-save {
    margin-left: auto;
    padding: 9px 24px;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.08em;
    background: var(--color-accent, #c65f3c);
    color: #fff;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .btn-save:disabled { opacity: 0.45; cursor: default; }
</style>
