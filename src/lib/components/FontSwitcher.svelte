<script lang="ts">
  import { fade } from 'svelte/transition';
  import { t } from '$lib/i18n';
  import {
    READING_FONTS,
    readingFont,
    setReadingFont,
    preloadReadingFonts,
    previewReadingFont,
    endReadingFontPreview,
  } from '$lib/stores/reading-font.svelte';

  // 'header' → compact "Aa ▾" control in the top bar.
  // 'colophon' → "set in · Garamond ▾" line that sits beside a section header.
  let { variant = 'header' }: { variant?: 'header' | 'colophon' } = $props();

  let open = $state(false);
  let anchor = $state<HTMLElement | null>(null);

  function toggle() {
    if (open) {
      close();
    } else {
      open = true;
      preloadReadingFonts();
    }
  }
  function close() {
    open = false;
    endReadingFontPreview(); // drop any lingering hover preview
  }
  function choose(id: string) {
    setReadingFont(id);
    close();
  }

  function onOutside(e: MouseEvent) {
    if (open && anchor && !anchor.contains(e.target as Node)) close();
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }
</script>

<svelte:window onclick={onOutside} onkeydown={onKey} />

<div class="font-anchor" class:is-colophon={variant === 'colophon'} bind:this={anchor}>
  {#if variant === 'colophon'}
    <button
      class="colophon-trigger"
      class:is-open={open}
      onclick={toggle}
      aria-haspopup="listbox"
      aria-expanded={open}
      aria-label={$t('navTypeface')}
    >
      <span class="colophon-lead">{$t('readingColophon')}</span>
      <span class="colophon-name" style="font-family: {$readingFont.stack}">{$readingFont.name}</span>
      <span class="caret" aria-hidden="true">▾</span>
    </button>
  {:else}
    <button
      class="aa-trigger"
      class:is-open={open}
      onclick={toggle}
      aria-haspopup="listbox"
      aria-expanded={open}
      aria-label={$t('navTypeface')}
      title={$t('navTypeface')}
    >
      <span class="aa-glyph" style="font-family: {$readingFont.stack}">Aa</span>
      <span class="caret" aria-hidden="true">▾</span>
    </button>
  {/if}

  {#if open}
    <div class="font-menu" role="listbox" aria-label={$t('readingMenuTitle')} transition:fade={{ duration: 130 }}>
      <div class="menu-head">{$t('readingMenuTitle')}</div>
      <ul class="menu-list" onmouseleave={endReadingFontPreview}>
        {#each READING_FONTS as font (font.id)}
          <li>
            <button
              class="menu-item"
              class:is-active={$readingFont.id === font.id}
              role="option"
              aria-selected={$readingFont.id === font.id}
              onclick={() => choose(font.id)}
              onmouseenter={() => previewReadingFont(font.id)}
              onfocus={() => previewReadingFont(font.id)}
              onblur={endReadingFontPreview}
            >
              <span class="item-mark" aria-hidden="true"></span>
              <span class="item-text">
                <span class="item-name" style="font-family: {font.stack}">{font.name}</span>
                <span class="item-note">{font.note}</span>
              </span>
              <span class="item-sample" style="font-family: {font.stack}" aria-hidden="true">Аа Gg</span>
            </button>
          </li>
        {/each}
      </ul>
      <p class="menu-hint">{$t('readingMenuHint')}</p>
    </div>
  {/if}
</div>

<style>
  .font-anchor {
    position: relative;
    display: inline-flex;
  }

  /* ── Header "Aa ▾" trigger ── */
  .aa-trigger {
    display: inline-flex;
    align-items: baseline;
    gap: 3px;
    padding: 2px 4px;
    background: none;
    border: none;
    cursor: pointer;
    color: color-mix(in srgb, var(--color-ink-secondary) 72%, transparent);
    transition: color 0.2s;
  }
  .aa-trigger:hover,
  .aa-trigger.is-open {
    color: var(--color-ember);
  }
  .aa-glyph {
    font-size: 16px;
    line-height: 1;
    letter-spacing: 0.01em;
  }
  .caret {
    font-size: 8px;
    line-height: 1;
    opacity: 0.7;
  }

  /* ── Colophon "set in · Garamond ▾" line ── */
  .colophon-trigger {
    display: inline-flex;
    align-items: baseline;
    gap: 0.4em;
    padding: 0;
    background: none;
    border: none;
    cursor: pointer;
    color: color-mix(in srgb, var(--color-ink-secondary) 58%, transparent);
    transition: color 0.2s;
  }
  .colophon-lead {
    font-family: var(--font-body);
    font-size: 0.58rem;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }
  .colophon-name {
    font-size: 0.95rem;
    line-height: 1;
    color: var(--color-ember-deep);
    border-bottom: 1px solid color-mix(in srgb, var(--color-ember) 32%, transparent);
    padding-bottom: 1px;
    transition: border-color 0.2s, color 0.2s;
  }
  .colophon-trigger:hover .colophon-name,
  .colophon-trigger.is-open .colophon-name {
    color: var(--color-ember);
    border-bottom-color: var(--color-ember);
  }
  .colophon-trigger .caret {
    font-size: 9px;
    color: color-mix(in srgb, var(--color-ember) 60%, transparent);
  }

  /* ── Dropdown menu (shared) ── */
  .font-menu {
    position: absolute;
    top: calc(100% + 9px);
    z-index: 320;
    width: 244px;
    background: var(--color-canvas-raised, #fdfaf5);
    border: 1px solid var(--color-cabinet-wood, #d8c6b1);
    box-shadow: 0 10px 34px color-mix(in srgb, var(--color-ink-primary) 16%, transparent);
    color: var(--color-ink-primary);
  }
  /* Header variant opens flush-right; colophon opens flush-left under the line. */
  .font-anchor:not(.is-colophon) .font-menu { right: 0; }
  .font-anchor.is-colophon .font-menu { left: 0; }

  .menu-head {
    padding: 11px 14px 9px;
    font-family: var(--font-body);
    font-size: 0.56rem;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--color-ink-secondary) 55%, transparent);
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 8%, transparent);
  }

  .menu-list {
    list-style: none;
    margin: 0;
    padding: 4px 0;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 14px;
    background: none;
    border: none;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s;
  }
  .menu-item:hover {
    background: color-mix(in srgb, var(--color-ember) 7%, transparent);
  }

  .item-mark {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 22%, transparent);
    background: transparent;
    transition: background 0.15s, border-color 0.15s;
  }
  .menu-item.is-active .item-mark {
    background: var(--color-ember);
    border-color: var(--color-ember);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-ember) 22%, transparent);
  }

  .item-text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    flex: 1;
    min-width: 0;
  }
  .item-name {
    font-size: 1.02rem;
    line-height: 1.1;
    color: var(--color-ink-primary);
  }
  .menu-item.is-active .item-name {
    color: var(--color-ember-deep);
  }
  .item-note {
    font-family: var(--font-body);
    font-size: 0.6rem;
    letter-spacing: 0.04em;
    color: color-mix(in srgb, var(--color-ink-secondary) 52%, transparent);
  }
  .item-sample {
    flex: none;
    font-size: 1.05rem;
    color: color-mix(in srgb, var(--color-ink-secondary) 40%, transparent);
  }

  .menu-hint {
    margin: 0;
    padding: 9px 14px 11px;
    border-top: 1px solid color-mix(in srgb, var(--color-ink-primary) 8%, transparent);
    font-family: var(--font-body);
    font-size: 0.62rem;
    line-height: 1.45;
    color: color-mix(in srgb, var(--color-ink-secondary) 48%, transparent);
  }

  @media (max-width: 680px) {
    .font-anchor:not(.is-colophon) .font-menu {
      right: -8px;
    }
  }
</style>
