<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import type {
    CardStyle,
    CardTarget,
    GradientStop,
    ReelTheme,
    ReelThemePreset,
  } from '$lib/types/api';
  import {
    BACKDROP_PRESETS,
    BUTTON_PRESETS,
    GLASS_PRESETS,
    HOUSE_THEMES,
    HERO_DEFAULTS,
    REEL_DEFAULTS,
    TYPE_PRESETS,
    WORK_DEFAULTS,
    applyReelPreview,
    clearReelPreview,
    luminance,
    resolveReelTheme,
  } from '$lib/stores/reel-theme.svelte';

  let cfg = $state<ReelTheme>(structuredClone(REEL_DEFAULTS) as ReelTheme);
  let presets = $state<ReelThemePreset[]>([]);
  let presetName = $state('');
  let saving = $state(false);
  let status = $state<string | null>(null);
  let loaded = $state(false);
  let dirty = $state(false);

  /** Which pane the card controls below are editing. */
  let target = $state<CardTarget>('hero');

  let bp = $state<'desktop' | 'mobile'>('desktop');
  let bpWidth = $derived(bp === 'desktop' ? 1280 : 390);
  let previewScale = $derived(bp === 'desktop' ? 0.42 : 0.85);

  let resolved = $derived(resolveReelTheme(cfg));
  let card = $derived(resolved[target]);

  let gradientEditsBackdrop = $derived(resolved.backdropKind === 'gradient');

  /** Write one field of the pane currently being edited. */
  function setCard<K extends keyof CardStyle>(key: K, value: CardStyle[K]) {
    const current = cfg[target] ?? {};
    cfg[target] = { ...current, [key]: value };
  }

  function applyCardPreset(patch: Partial<CardStyle>) {
    cfg[target] = { ...(cfg[target] ?? {}), ...patch };
  }

  function applyBackdropPreset(patch: Partial<ReelTheme>) {
    cfg = { ...cfg, ...patch };
  }

  /**
   * A house theme is a COMPLETE look, so it replaces everything — otherwise you
   * end up wearing half of one and half of another. The one thing it keeps is the
   * photograph itself: that is the author's, not the theme's.
   */
  function applyHouseTheme(patch: Partial<ReelTheme>) {
    const image = cfg.backgroundImage;
    const imageMobile = cfg.backgroundImageMobile;
    cfg = structuredClone({
      ...REEL_DEFAULTS,
      ...patch,
      backgroundImage: image ?? REEL_DEFAULTS.backgroundImage,
      backgroundImageMobile: imageMobile ?? '',
    }) as ReelTheme;
  }

  /** Copy the pane you just tuned onto the other one. */
  function copyToOther() {
    const other: CardTarget = target === 'hero' ? 'work' : 'hero';
    cfg[other] = { ...resolved[target] };
  }

  function resetCard() {
    cfg[target] = { ...(target === 'hero' ? HERO_DEFAULTS : WORK_DEFAULTS) };
  }

  /**
   * The one thing here that can render the site unreadable: type that matches the
   * surface behind it. Warn rather than forbid — and offer the fix.
   */
  let contrastWarning = $derived.by(() => {
    const backdropLum =
      resolved.backdropKind === 'image'
        // A photo has no single colour; its brightness knob is the closest proxy.
        ? resolved.backgroundBrightness
        : luminance(resolved.backdropColor);
    const veiled = backdropLum * (1 - Math.min(0.85, resolved.overlayOpacity));
    const titleLum = luminance(card.titleColor);
    if (Math.abs(titleLum - veiled) < 0.22) return true;
    return false;
  });

  // Every edit repaints the preview immediately — tuning glass by
  // save-navigate-look-go-back is not tuning, it is guessing.
  $effect(() => {
    if (!loaded) return;
    applyReelPreview($state.snapshot(cfg));
    dirty = true;
  });

  onMount(() => {
    void (async () => {
      const [saved, savedPresets] = await Promise.all([
        api.getReelTheme().catch(() => null),
        api.getReelThemePresets().catch(() => [] as ReelThemePreset[]),
      ]);
      // Resolving folds a legacy (pre-split) theme into both panes, so an
      // already-saved look keeps rendering as it did.
      cfg = structuredClone(resolveReelTheme(saved)) as ReelTheme;
      presets = savedPresets;
      loaded = true;
      queueMicrotask(() => { dirty = false; });
    })();
  });

  onDestroy(() => {
    // Leaving the panel must not leave the live site wearing an unsaved draft.
    clearReelPreview();
  });

  // The home page inside the iframe only starts listening once it mounts, so a
  // draft sent before that is lost — re-send when the frame reports ready.
  function onIframeLoad() {
    if (!loaded) return;
    const snapshot = $state.snapshot(cfg);
    applyReelPreview(snapshot);
    setTimeout(() => applyReelPreview(snapshot), 500);
  }

  async function save() {
    saving = true;
    status = null;
    try {
      await api.saveReelTheme($state.snapshot(cfg));
      status = $t('adminReelSaved');
      dirty = false;
    } catch (e) {
      status = e instanceof Error ? e.message : String(e);
    } finally {
      saving = false;
    }
  }

  function resetToDefaults() {
    cfg = structuredClone(REEL_DEFAULTS) as ReelTheme;
  }

  async function savePreset() {
    const name = presetName.trim();
    if (!name) return;
    presets = await api.saveReelThemePresets([
      ...presets,
      { id: crypto.randomUUID(), name, config: $state.snapshot(cfg), savedAt: new Date().toISOString() },
    ]);
    presetName = '';
  }

  function applyPreset(p: ReelThemePreset) {
    cfg = structuredClone(resolveReelTheme(p.config)) as ReelTheme;
  }

  async function deletePreset(id: string) {
    presets = await api.saveReelThemePresets(presets.filter((p) => p.id !== id));
  }

  // ── Gradient stops ──────────────────────────────────────────────────────────

  let stops = $derived(cfg.gradientStops ?? REEL_DEFAULTS.gradientStops);

  function updateStop(i: number, patch: Partial<GradientStop>) {
    cfg.gradientStops = stops.map((s, idx) => (idx === i ? { ...s, ...patch } : s));
  }

  function addStop() {
    cfg.gradientStops = [
      ...stops,
      { color: cfg.overlayColor ?? REEL_DEFAULTS.overlayColor, position: 50, opacity: 0.3 },
    ];
  }

  function removeStop(i: number) {
    // A gradient needs two ends; below that it is a flat colour, which is its
    // own mode on this panel.
    if (stops.length <= 2) return;
    cfg.gradientStops = stops.filter((_, idx) => idx !== i);
  }

  // ── The pane's own fill ─────────────────────────────────────────────────────
  // Same editor, different surface: the stops above paint the ROOM, these paint
  // the card. They are deliberately separate — a card gradient over a gradient
  // backdrop is a legitimate look, and sharing one stop list would make it
  // impossible.

  let fillStops = $derived(card.fillStops);

  function updateFillStop(i: number, patch: Partial<GradientStop>) {
    setCard('fillStops', fillStops.map((s, idx) => (idx === i ? { ...s, ...patch } : s)));
  }

  function addFillStop() {
    setCard('fillStops', [...fillStops, { color: card.glassTint, position: 50, opacity: 1 }]);
  }

  function removeFillStop(i: number) {
    if (fillStops.length <= 2) return;
    setCard('fillStops', fillStops.filter((_, idx) => idx !== i));
  }

  /** The fill exactly as the card will wear it — swatch for the editor. */
  let fillPreview = $derived.by(() => {
    const o = card.glassOpacity;
    if (card.fillKind !== 'gradient') return card.glassTint;
    const s = [...card.fillStops]
      .sort((a, b) => a.position - b.position)
      .map((st) => `${st.color} ${st.position}%`)
      .join(', ');
    void o;
    if (card.fillType === 'radial') return `radial-gradient(circle at 50% 50%, ${s})`;
    if (card.fillType === 'conic') return `conic-gradient(from ${card.fillAngle}deg at 50% 50%, ${s})`;
    return `linear-gradient(${card.fillAngle}deg, ${s})`;
  });
</script>

<div class="reel-panel">
  <div class="controls">
    <header class="bar">
      <div class="bar-title">
        <h2>{$t('adminReelTitle')}</h2>
        {#if dirty}<span class="dot" title={$t('adminReelUnsaved')}></span>{/if}
      </div>
      <div class="bar-actions">
        <button type="button" class="primary" onclick={save} disabled={saving}>
          {saving ? '…' : $t('adminReelSave')}
        </button>
        <button type="button" onclick={resetToDefaults}>{$t('adminReelReset')}</button>
      </div>
    </header>

    {#if status}<p class="status">{status}</p>{/if}

    <div class="scroll">
      <p class="hint">{$t('adminReelHint')}</p>

      {#if contrastWarning}
        <div class="warn">
          <strong>{$t('adminReelContrastWarn')}</strong>
        </div>
      {/if}

      <!-- ══ Five finished looks ══ -->
      <section class="themes">
        <h3>{$t('adminReelThemes')}</h3>
        <p class="note">{$t('adminReelThemesHint')}</p>
        <div class="theme-grid">
          {#each HOUSE_THEMES as th (th.id)}
            <button type="button" class="theme" onclick={() => applyHouseTheme(th.patch)}>
              {$t(th.label)}
            </button>
          {/each}
        </div>
      </section>

      <!-- ══ THE ROOM (one backdrop behind everything) ══ -->
      <section>
        <h3>{$t('adminReelBackdrop')}</h3>

        <div class="chips">
          {#each BACKDROP_PRESETS as p (p.id)}
            <button type="button" onclick={() => applyBackdropPreset(p.patch)}>{$t(p.label)}</button>
          {/each}
        </div>

        <label class="row">
          <span>{$t('adminReelBackdropKind')}</span>
          <select bind:value={cfg.backdropKind}>
            <option value="image">{$t('adminReelKindImage')}</option>
            <option value="color">{$t('adminReelKindColor')}</option>
            <option value="gradient">{$t('adminReelKindGradient')}</option>
          </select>
        </label>

        {#if resolved.backdropKind === 'image'}
          <label class="row">
            <span>{$t('adminReelImage')}</span>
            <input type="text" bind:value={cfg.backgroundImage} placeholder="/images/cabinet-bg.jpeg" />
          </label>

          <label class="row">
            <span>{$t('adminReelImageMobile')}</span>
            <input type="text" bind:value={cfg.backgroundImageMobile} placeholder={$t('adminReelImageMobileHint')} />
          </label>

          <label class="row">
            <span>{$t('adminReelFit')}</span>
            <select bind:value={cfg.backgroundFit}>
              <option value="cover">{$t('adminReelFitCover')}</option>
              <option value="contain">{$t('adminReelFitContain')}</option>
            </select>
          </label>

          <label class="row">
            <span>{$t('adminReelPosition')}</span>
            <select bind:value={cfg.backgroundPosition}>
              <option value="center top">{$t('adminReelPosTop')}</option>
              <option value="center center">{$t('adminReelPosCenter')}</option>
              <option value="center bottom">{$t('adminReelPosBottom')}</option>
              <option value="left center">{$t('adminReelPosLeft')}</option>
              <option value="right center">{$t('adminReelPosRight')}</option>
            </select>
          </label>

          <label class="row">
            <span>{$t('adminReelBlur')} <b>{resolved.backgroundBlur}px</b></span>
            <input type="range" min="0" max="20" step="0.5" bind:value={cfg.backgroundBlur} />
          </label>

          <label class="row">
            <span>{$t('adminReelBrightness')} <b>{resolved.backgroundBrightness}</b></span>
            <input type="range" min="0.1" max="1.4" step="0.02" bind:value={cfg.backgroundBrightness} />
          </label>

          <label class="row">
            <span>{$t('adminReelSaturation')} <b>{resolved.backgroundSaturation}</b></span>
            <input type="range" min="0" max="1.6" step="0.02" bind:value={cfg.backgroundSaturation} />
          </label>

          {#if resolved.backgroundFit === 'contain'}
            <label class="row">
              <span>{$t('adminReelLetterbox')}</span>
              <input type="color" bind:value={cfg.backdropColor} />
            </label>
          {/if}
        {:else}
          <!-- A colour picker labelled "background" that does nothing because a
               photograph covers it is exactly how the first version lied. It only
               appears where it actually paints something. -->
          <label class="row">
            <span>{$t('adminReelBackdropColor')}</span>
            <input type="color" bind:value={cfg.backdropColor} />
          </label>
        {/if}
      </section>

      <!-- ══ Gradient: edits whichever surface owns it ══ -->
      {#if gradientEditsBackdrop || resolved.overlayKind === 'gradient'}
        <section>
          <h3>{gradientEditsBackdrop ? $t('adminReelGradientOfBackdrop') : $t('adminReelGradientOfVeil')}</h3>

          <label class="row">
            <span>{$t('adminReelGradientType')}</span>
            <select bind:value={cfg.gradientType}>
              <option value="linear">{$t('adminReelGradLinear')}</option>
              <option value="radial">{$t('adminReelGradRadial')}</option>
              <option value="conic">{$t('adminReelGradConic')}</option>
            </select>
          </label>

          {#if resolved.gradientType !== 'radial'}
            <label class="row">
              <span>{$t('adminReelGradientAngle')} <b>{resolved.gradientAngle}°</b></span>
              <input type="range" min="0" max="360" step="1" bind:value={cfg.gradientAngle} />
            </label>
          {/if}

          <div class="stops-head">
            <span>{$t('adminReelStops')}</span>
            <button type="button" onclick={addStop}>+ {$t('adminReelAddStop')}</button>
          </div>

          {#each stops as stop, i (i)}
            <div class="stop">
              <input type="color" value={stop.color} oninput={(e) => updateStop(i, { color: e.currentTarget.value })} />
              <label>
                <span>{stop.position}%</span>
                <input type="range" min="0" max="100" step="1" value={stop.position}
                  oninput={(e) => updateStop(i, { position: Number(e.currentTarget.value) })} />
              </label>
              <label>
                <span>α {stop.opacity}</span>
                <input type="range" min="0" max="1" step="0.01" value={stop.opacity}
                  oninput={(e) => updateStop(i, { opacity: Number(e.currentTarget.value) })} />
              </label>
              <button type="button" class="del" disabled={stops.length <= 2}
                onclick={() => removeStop(i)} aria-label={$t('adminReelRemoveStop')}>✕</button>
            </div>
          {/each}
        </section>
      {/if}

      <!-- ══ Veil ══ -->
      <section>
        <h3>{$t('adminReelOverlay')}</h3>

        {#if gradientEditsBackdrop}
          <p class="note">{$t('adminReelVeilOffNote')}</p>
        {:else}
          <label class="row">
            <span>{$t('adminReelOverlayKind')}</span>
            <select bind:value={cfg.overlayKind}>
              <option value="none">{$t('adminReelOverlayNone')}</option>
              <option value="solid">{$t('adminReelOverlaySolid')}</option>
              <option value="gradient">{$t('adminReelKindGradient')}</option>
            </select>
          </label>

          {#if resolved.overlayKind === 'solid'}
            <label class="row">
              <span>{$t('adminReelOverlayColor')}</span>
              <input type="color" bind:value={cfg.overlayColor} />
            </label>
            <label class="row">
              <span>{$t('adminReelOverlayOpacity')} <b>{resolved.overlayOpacity}</b></span>
              <input type="range" min="0" max="1" step="0.01" bind:value={cfg.overlayOpacity} />
            </label>
          {/if}
        {/if}

        <label class="row">
          <span>{$t('adminReelVignette')} <b>{resolved.vignette}</b></span>
          <input type="range" min="0" max="1" step="0.01" bind:value={cfg.vignette} />
        </label>

        {#if resolved.vignette > 0}
          <label class="row">
            <span>{$t('adminReelVignetteColor')}</span>
            <input type="color" bind:value={cfg.overlayColor} />
          </label>
        {/if}

        <label class="row">
          <span>{$t('adminReelGrain')} <b>{resolved.grain}</b></span>
          <input type="range" min="0" max="0.6" step="0.01" bind:value={cfg.grain} />
        </label>
      </section>

      <!-- ══ THE PANES — each surface set independently ══ -->
      <section class="card-section">
        <div class="target">
          <button type="button" class:on={target === 'hero'} onclick={() => (target = 'hero')}>
            {$t('adminReelTargetHero')}
          </button>
          <button type="button" class:on={target === 'work'} onclick={() => (target = 'work')}>
            {$t('adminReelTargetWork')}
          </button>
        </div>
        <p class="note">
          {target === 'hero' ? $t('adminReelTargetHeroHint') : $t('adminReelTargetWorkHint')}
        </p>

        <div class="chips">
          <button type="button" onclick={copyToOther}>{$t('adminReelCopyToOther')}</button>
          <button type="button" onclick={resetCard}>{$t('adminReelResetCard')}</button>
        </div>
      </section>

      <!-- Glass of the selected pane -->
      <section>
        <h3>{$t('adminReelGlass')}</h3>

        <div class="chips">
          {#each GLASS_PRESETS as p (p.id)}
            <button type="button" onclick={() => applyCardPreset(p.patch)}>{$t(p.label)}</button>
          {/each}
        </div>

        <!-- What the pane is FILLED with: one flat colour, or a gradient of its
             own. The opacity slider below governs both — see cardFill(). -->
        <label class="row">
          <span>{$t('adminReelCardFill')}</span>
          <select value={card.fillKind} onchange={(e) => setCard('fillKind', e.currentTarget.value as 'solid' | 'gradient')}>
            <option value="solid">{$t('adminReelCardFillSolid')}</option>
            <option value="gradient">{$t('adminReelCardFillGradient')}</option>
          </select>
        </label>

        <div class="fill-swatch" style="background: {fillPreview}" aria-hidden="true"></div>

        {#if card.fillKind === 'gradient'}
          <label class="row">
            <span>{$t('adminReelGradientType')}</span>
            <select value={card.fillType} onchange={(e) => setCard('fillType', e.currentTarget.value as 'linear' | 'radial' | 'conic')}>
              <option value="linear">{$t('adminReelGradLinear')}</option>
              <option value="radial">{$t('adminReelGradRadial')}</option>
              <option value="conic">{$t('adminReelGradConic')}</option>
            </select>
          </label>

          {#if card.fillType !== 'radial'}
            <label class="row">
              <span>{$t('adminReelGradientAngle')} <b>{card.fillAngle}°</b></span>
              <input type="range" min="0" max="360" step="1" value={card.fillAngle}
                oninput={(e) => setCard('fillAngle', Number(e.currentTarget.value))} />
            </label>
          {/if}

          <div class="stops-head">
            <span>{$t('adminReelStops')}</span>
            <button type="button" onclick={addFillStop}>+ {$t('adminReelAddStop')}</button>
          </div>

          {#each fillStops as stop, i (i)}
            <div class="stop">
              <input type="color" value={stop.color} oninput={(e) => updateFillStop(i, { color: e.currentTarget.value })} />
              <label>
                <span>{stop.position}%</span>
                <input type="range" min="0" max="100" step="1" value={stop.position}
                  oninput={(e) => updateFillStop(i, { position: Number(e.currentTarget.value) })} />
              </label>
              <label>
                <span>α {stop.opacity}</span>
                <input type="range" min="0" max="1" step="0.01" value={stop.opacity}
                  oninput={(e) => updateFillStop(i, { opacity: Number(e.currentTarget.value) })} />
              </label>
              <button type="button" class="del" disabled={fillStops.length <= 2}
                onclick={() => removeFillStop(i)} aria-label={$t('adminReelRemoveStop')}>✕</button>
            </div>
          {/each}
        {:else}
          <label class="row">
            <span>{$t('adminReelGlassTint')}</span>
            <input type="color" value={card.glassTint} oninput={(e) => setCard('glassTint', e.currentTarget.value)} />
          </label>
        {/if}

        <label class="row">
          <span>{$t('adminReelGlassOpacity')} <b>{card.glassOpacity}</b></span>
          <input type="range" min="0" max="1" step="0.01" value={card.glassOpacity}
            oninput={(e) => setCard('glassOpacity', Number(e.currentTarget.value))} />
        </label>

        <label class="row">
          <span>{$t('adminReelGlassBlur')} <b>{card.glassBlur}px</b></span>
          <input type="range" min="0" max="60" step="1" value={card.glassBlur} disabled={resolved.performanceMode}
            oninput={(e) => setCard('glassBlur', Number(e.currentTarget.value))} />
        </label>

        <label class="row">
          <span>{$t('adminReelGlassSaturation')} <b>{card.glassSaturation}</b></span>
          <input type="range" min="0.5" max="2.5" step="0.05" value={card.glassSaturation}
            oninput={(e) => setCard('glassSaturation', Number(e.currentTarget.value))} />
        </label>

        <label class="row">
          <span>{$t('adminReelGlassRadius')} <b>{card.glassRadius}px</b></span>
          <input type="range" min="0" max="60" step="1" value={card.glassRadius}
            oninput={(e) => setCard('glassRadius', Number(e.currentTarget.value))} />
        </label>

        <label class="row">
          <span>{$t('adminReelGlassSheen')} <b>{card.glassSheen}</b></span>
          <input type="range" min="0" max="2" step="0.05" value={card.glassSheen}
            oninput={(e) => setCard('glassSheen', Number(e.currentTarget.value))} />
        </label>

        <label class="row">
          <span>{$t('adminReelGlassShadow')} <b>{card.glassShadow}</b></span>
          <input type="range" min="0" max="1" step="0.01" value={card.glassShadow}
            oninput={(e) => setCard('glassShadow', Number(e.currentTarget.value))} />
        </label>

        <label class="row">
          <span>{$t('adminReelShadowColor')}</span>
          <input type="color" value={card.shadowColor} oninput={(e) => setCard('shadowColor', e.currentTarget.value)} />
        </label>

        <!-- The hairline around the pane. Until now it was hardcoded in the card,
             so a dark theme kept a warm brown edge it never asked for. -->
        <label class="row">
          <span>{$t('adminReelEdgeColor')}</span>
          <input type="color" value={card.edgeColor} oninput={(e) => setCard('edgeColor', e.currentTarget.value)} />
        </label>

        <label class="row">
          <span>{$t('adminReelEdgeOpacity')} <b>{card.edgeOpacity}</b></span>
          <input type="range" min="0" max="1" step="0.01" value={card.edgeOpacity}
            oninput={(e) => setCard('edgeOpacity', Number(e.currentTarget.value))} />
        </label>

        <label class="row">
          <span>{$t('adminReelEdgeHoverColor')}</span>
          <input type="color" value={card.edgeHoverColor} oninput={(e) => setCard('edgeHoverColor', e.currentTarget.value)} />
        </label>
      </section>

      <!-- Type of the selected pane -->
      <section>
        <h3>{$t('adminReelType')}</h3>

        <div class="chips">
          {#each TYPE_PRESETS as p (p.id)}
            <button type="button" onclick={() => applyCardPreset(p.patch)}>{$t(p.label)}</button>
          {/each}
        </div>

        <label class="row">
          <span>{$t('adminReelTitleColor')}</span>
          <input type="color" value={card.titleColor} oninput={(e) => setCard('titleColor', e.currentTarget.value)} />
        </label>

        <label class="row">
          <span>{$t('adminReelTitleSize')} <b>{card.titleSize}rem</b></span>
          <input type="range" min="1.2" max="6" step="0.1" value={card.titleSize}
            oninput={(e) => setCard('titleSize', Number(e.currentTarget.value))} />
        </label>

        <label class="row">
          <span>{$t('adminReelBodyColor')}</span>
          <input type="color" value={card.bodyColor} oninput={(e) => setCard('bodyColor', e.currentTarget.value)} />
        </label>

        <label class="row">
          <span>{$t('adminReelBodySize')} <b>{card.bodySize}rem</b></span>
          <input type="range" min="0.6" max="1.6" step="0.02" value={card.bodySize}
            oninput={(e) => setCard('bodySize', Number(e.currentTarget.value))} />
        </label>

        <label class="row">
          <span>{$t('adminReelMetaColor')}</span>
          <input type="color" value={card.metaColor} oninput={(e) => setCard('metaColor', e.currentTarget.value)} />
        </label>

        <label class="row">
          <span>{$t('adminReelMetaSize')} <b>{card.metaSize}rem</b></span>
          <input type="range" min="0.5" max="1.6" step="0.02" value={card.metaSize}
            oninput={(e) => setCard('metaSize', Number(e.currentTarget.value))} />
        </label>
      </section>

      <!-- Buttons of the selected pane -->
      <section>
        <h3>{$t('adminReelButtons')}</h3>

        <div class="chips">
          {#each BUTTON_PRESETS as p (p.id)}
            <button type="button" onclick={() => applyCardPreset(p.patch)}>{$t(p.label)}</button>
          {/each}
        </div>

        <label class="row">
          <span>{$t('adminReelButtonFill')}</span>
          <input type="color" value={card.btnFill} oninput={(e) => setCard('btnFill', e.currentTarget.value)} />
        </label>

        <label class="row">
          <span>{$t('adminReelButtonText')}</span>
          <input type="color" value={card.btnText} oninput={(e) => setCard('btnText', e.currentTarget.value)} />
        </label>

        <label class="row">
          <span>{$t('adminReelButtonBorder')}</span>
          <input type="color" value={card.btnBorder} oninput={(e) => setCard('btnBorder', e.currentTarget.value)} />
        </label>

        <label class="row">
          <span>{$t('adminReelButtonRadius')} <b>{card.btnRadius}px</b></span>
          <input type="range" min="0" max="40" step="1" value={card.btnRadius}
            oninput={(e) => setCard('btnRadius', Number(e.currentTarget.value))} />
        </label>

        <label class="row">
          <span>{$t('adminReelButtonSize')} <b>{card.btnSize}rem</b></span>
          <input type="range" min="0.5" max="1.2" step="0.02" value={card.btnSize}
            oninput={(e) => setCard('btnSize', Number(e.currentTarget.value))} />
        </label>
      </section>

      <!-- ══ Whole reel ══ -->
      <section>
        <h3>{$t('adminReelDensity')}</h3>

        <label class="row">
          <span>{$t('adminReelCardGap')} <b>{resolved.cardGap}rem</b></span>
          <input type="range" min="0" max="10" step="0.25" bind:value={cfg.cardGap} />
        </label>

        <label class="row">
          <span>{$t('adminReelCardWidth')} <b>{resolved.cardWidth}rem</b></span>
          <input type="range" min="30" max="90" step="1" bind:value={cfg.cardWidth} />
        </label>

        <label class="row">
          <span>{$t('adminReelTextTone')}</span>
          <select bind:value={cfg.textTone}>
            <option value="light">{$t('adminReelToneLight')}</option>
            <option value="dark">{$t('adminReelToneDark')}</option>
          </select>
        </label>

        <label class="row check">
          <input type="checkbox" bind:checked={cfg.performanceMode} />
          <span>
            {$t('adminReelPerformance')}
            <em>{$t('adminReelPerformanceHint')}</em>
          </span>
        </label>
      </section>

      <!-- ══ Saved looks (whole theme) ══ -->
      <section>
        <h3>{$t('adminReelPresets')}</h3>

        {#each presets as p (p.id)}
          <div class="preset">
            <button type="button" class="preset-name" onclick={() => applyPreset(p)}>{p.name}</button>
            <button type="button" class="del" onclick={() => deletePreset(p.id)} aria-label="✕">✕</button>
          </div>
        {/each}

        <div class="preset-new">
          <input type="text" bind:value={presetName} placeholder={$t('adminReelPresetName')} />
          <button type="button" onclick={savePreset} disabled={!presetName.trim()}>
            {$t('adminReelSavePreset')}
          </button>
        </div>
      </section>
    </div>
  </div>

  <!-- ── Live preview ─────────────────────────────────────────────────────── -->
  <div class="preview">
    <div class="preview-bar">
      <button type="button" class:on={bp === 'desktop'} onclick={() => (bp = 'desktop')}>
        {$t('adminHomeLayoutBpDesktop')}
      </button>
      <button type="button" class:on={bp === 'mobile'} onclick={() => (bp = 'mobile')}>
        {$t('adminHomeLayoutBpMobile')}
      </button>
    </div>
    <div class="preview-frame">
      <iframe
        src="/"
        title={$t('adminReelPreview')}
        onload={onIframeLoad}
        style="width:{bpWidth}px; height:{Math.round(760 / previewScale)}px; transform:scale({previewScale});"
      ></iframe>
    </div>
  </div>
</div>

<style>
  .reel-panel {
    display: grid;
    grid-template-columns: minmax(330px, 430px) 1fr;
    gap: 20px;
    height: 100%;
    /* Grid children default to min-height:auto and refuse to shrink below their
       content — that is what left the controls column unscrollable. */
    min-height: 0;
    padding: 16px 20px;
  }

  .controls {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding-right: 10px;
  }

  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding-bottom: 10px;
    border-bottom: 1px solid var(--color-border-default);
    margin-bottom: 10px;
  }

  .bar-title {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .bar-title h2 {
    font-size: 16px;
    margin: 0;
  }

  /* Unsaved-draft mark: the preview lies about being live, so say so. */
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--color-ember);
  }

  .bar-actions {
    display: flex;
    gap: 6px;
  }

  .hint,
  .note {
    font-size: 12px;
    line-height: 1.5;
    color: var(--color-ink-tertiary);
    margin: 0 0 14px;
  }

  .note {
    margin: 8px 0 10px;
  }

  .status {
    font-size: 12px;
    color: var(--color-ink-tertiary);
    margin: 0 0 8px;
  }

  section {
    border-top: 1px solid var(--color-border-subtle);
    padding: 12px 0;
  }

  section h3 {
    font-size: 11px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
    margin: 0 0 10px;
  }

  .themes {
    border-top: none;
    padding-top: 0;
  }

  .theme-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }

  .theme {
    padding: 11px 12px;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    background: var(--color-canvas-raised);
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
    transition: border-color 0.2s, background 0.2s;
  }

  .theme:hover {
    border-color: var(--color-ember);
    background: var(--color-ember-subtle);
  }

  /* Which pane the card controls below are editing. */
  .card-section {
    border-top: 2px solid var(--color-border-strong);
  }

  .target {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }

  .target button {
    padding: 9px;
    border: 1px solid var(--color-border-default);
    border-radius: 5px;
    background: var(--color-canvas-raised);
    font-size: 12px;
    cursor: pointer;
  }

  .target button.on {
    background: var(--color-ink-primary);
    border-color: var(--color-ink-primary);
    color: var(--color-canvas-base);
  }

  /* Ready-made starting points. Each is a patch: it sets only what it is about. */
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    margin-bottom: 10px;
  }

  .chips button {
    padding: 5px 10px;
    border: 1px solid var(--color-border-default);
    border-radius: 20px;
    background: var(--color-canvas-raised);
    font-size: 11px;
    cursor: pointer;
  }

  .chips button:hover {
    border-color: var(--color-ember);
    color: var(--color-ember-ink);
  }

  .row {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 10px;
    margin-bottom: 8px;
    font-size: 13px;
  }

  .row b {
    color: var(--color-ink-tertiary);
    font-weight: 500;
  }

  .row input[type='range'] {
    width: 165px;
  }

  .row input[type='text'],
  .row select {
    width: 165px;
    padding: 5px 7px;
    border: 1px solid var(--color-border-default);
    border-radius: 4px;
    background: var(--color-canvas-raised);
    font-size: 12px;
  }

  .row.check {
    grid-template-columns: auto 1fr;
    align-items: start;
  }

  .row.check em {
    display: block;
    font-style: normal;
    font-size: 11px;
    color: var(--color-ink-tertiary);
  }

  /* The admin's global input styling flattens colour pickers into blank pills.
     Restore the swatch — a colour control that doesn't show its colour is
     useless. */
  input[type='color'] {
    -webkit-appearance: none;
    appearance: none;
    width: 44px;
    height: 26px;
    padding: 0;
    border: 1px solid var(--color-border-default);
    border-radius: 4px;
    background: none;
    cursor: pointer;
  }

  input[type='color']::-webkit-color-swatch-wrapper {
    padding: 2px;
  }

  input[type='color']::-webkit-color-swatch {
    border: none;
    border-radius: 2px;
  }

  .warn {
    padding: 9px 11px;
    margin-bottom: 14px;
    border: 1px solid var(--color-border-ember);
    background: var(--color-ember-subtle);
    border-radius: 5px;
    font-size: 12px;
  }

  /* The fill exactly as the card will wear it — a gradient is unreadable as a
     list of numbers. Checkerboard shows through wherever the fill is see-through. */
  .fill-swatch {
    height: 40px;
    margin: 8px 0 4px;
    border-radius: 4px;
    border: 1px solid rgba(0, 0, 0, 0.25);
  }

  .stops-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    margin: 10px 0 6px;
  }

  .stop {
    display: grid;
    grid-template-columns: 46px 1fr 1fr 24px;
    align-items: center;
    gap: 6px;
    margin-bottom: 5px;
  }

  .stop label {
    display: grid;
    font-size: 10px;
    color: var(--color-ink-tertiary);
  }

  .stop input[type='range'] {
    width: 100%;
  }

  .del {
    border: none;
    background: transparent;
    color: var(--color-ink-muted);
    cursor: pointer;
  }

  .del:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .preset,
  .preset-new {
    display: flex;
    gap: 6px;
    align-items: center;
    margin-bottom: 6px;
  }

  .preset-name {
    flex: 1;
    text-align: left;
    padding: 6px 9px;
    border: 1px solid var(--color-border-default);
    border-radius: 4px;
    background: var(--color-canvas-raised);
    font-size: 12px;
    cursor: pointer;
  }

  .preset-new input {
    flex: 1;
    padding: 6px 9px;
    border: 1px solid var(--color-border-default);
    border-radius: 4px;
    font-size: 12px;
  }

  .bar-actions button,
  .preset-new button,
  .stops-head button {
    padding: 7px 14px;
    border: 1px solid var(--color-border-default);
    border-radius: 5px;
    background: var(--color-canvas-raised);
    font-size: 12px;
    cursor: pointer;
  }

  .bar-actions .primary {
    background: var(--color-ember);
    border-color: var(--color-ember);
    color: #fff;
  }

  /* ── Preview ── */
  .preview {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .preview-bar {
    display: flex;
    gap: 6px;
    margin-bottom: 10px;
  }

  .preview-bar button {
    padding: 5px 12px;
    border: 1px solid var(--color-border-default);
    border-radius: 4px;
    background: var(--color-canvas-raised);
    font-size: 11px;
    cursor: pointer;
  }

  .preview-bar button.on {
    background: var(--color-ink-primary);
    color: var(--color-canvas-base);
    border-color: var(--color-ink-primary);
  }

  .preview-frame {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    background: var(--color-canvas-sunken);
    display: flex;
    justify-content: center;
  }

  /* A flex item shrinks by default, so the iframe collapsed to the column's width
     and the "desktop" preview came out phone-shaped. It must keep the viewport
     width it is emulating and be scaled, not squeezed. */
  .preview-frame iframe {
    flex: 0 0 auto;
    border: 0;
    transform-origin: top center;
  }
</style>
