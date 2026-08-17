<script lang="ts">
  /**
   * A parchment-styled facet selector — replacement for native <select>.
   * One control vocabulary with the status chips: same letterpress frame,
   * same typography, zero OS chrome. Opens a small "pasted-in index" panel.
   */
  interface Option { value: string; label: string; count?: number; }

  let {
    label,
    value = $bindable(),
    options,
    allValue = 'all',
    allLabel,
  }: {
    label: string;
    value: string;
    options: Option[];
    allValue?: string;
    allLabel: string;
  } = $props();

  let open = $state(false);
  let root = $state<HTMLElement>();

  let active = $derived(value !== allValue);
  // The "all / default" row is always rendered from allLabel — drop the same
  // value if the caller also put it in `options` (the sort chip used to).
  let menuOptions = $derived(options.filter((o) => o.value !== allValue));
  let currentLabel = $derived(
    value === allValue ? label : (options.find(o => o.value === value)?.label ?? label)
  );

  function select(v: string) {
    value = v;
    open = false;
  }

  function onWindowClick(e: MouseEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) { open = false; }
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onKey} />

<div class="fpop" bind:this={root}>
  <button
    type="button"
    class="fpop__trigger {active ? 'fpop__trigger--active' : ''}"
    onclick={() => open = !open}
    aria-haspopup="listbox"
    aria-expanded={open}
  >
    <span class="fpop__label">{currentLabel}</span>
    <svg class="fpop__chev {open ? 'fpop__chev--open' : ''}" width="8" height="5" viewBox="0 0 8 5" fill="none" aria-hidden="true">
      <path d="M1 1l3 3 3-3" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
    </svg>
  </button>

  {#if open}
    <div class="fpop__panel" role="listbox" tabindex="-1">
      <button
        type="button"
        role="option"
        aria-selected={value === allValue}
        class="fpop__opt {value === allValue ? 'fpop__opt--sel' : ''}"
        onclick={() => select(allValue)}
      >
        <span class="fpop__opt-label">{allLabel}</span>
      </button>
      {#each menuOptions as opt (opt.value)}
        <button
          type="button"
          role="option"
          aria-selected={value === opt.value}
          class="fpop__opt {value === opt.value ? 'fpop__opt--sel' : ''}"
          onclick={() => select(opt.value)}
        >
          <span class="fpop__opt-label">{opt.label}</span>
          {#if opt.count != null}<span class="fpop__opt-count">{opt.count}</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .fpop {
    position: relative;
    flex-shrink: 0;
  }

  /* ── Trigger: identical letterpress language to the status chips ── */
  .fpop__trigger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px 5px 11px;
    font-family: 'Inter', sans-serif;
    font-size: 9px;
    letter-spacing: 0.10em;
    text-transform: uppercase;
    line-height: 1;
    color: rgba(95,70,54,0.68);
    background: transparent;
    border: 1px solid rgba(52,37,28,0.13);
    cursor: pointer;
    white-space: nowrap;
    transition: border-color 0.18s, color 0.18s, background 0.18s;
  }
  .fpop__trigger:hover {
    border-color: rgba(52,37,28,0.30);
    color: rgba(52,37,28,0.85);
  }
  .fpop__trigger--active {
    border-color: rgba(52,37,28,0.45);
    color: #34251c;
    background: rgba(52,37,28,0.04);
  }

  .fpop__chev {
    color: rgba(95,70,54,0.45);
    transition: transform 0.22s cubic-bezier(0.16,1,0.3,1);
  }
  .fpop__chev--open { transform: rotate(180deg); }

  /* ── Panel: a small parchment "vklejka" pasted onto the page ── */
  .fpop__panel {
    position: absolute;
    top: calc(100% + 5px);
    left: 0;
    z-index: 40;
    min-width: max(100%, 12.5rem);
    max-width: 240px;
    max-height: 300px;
    overflow-y: auto;
    padding: 4px;
    background: #fbf5ec;
    border: 1px solid rgba(52,37,28,0.22);
    border-radius: 3px;
    box-shadow: 0 10px 28px -14px rgba(52,37,28,0.45);
    scrollbar-width: thin;
    transform-origin: top left;
    animation: fpop-in 0.16s cubic-bezier(0.16,1,0.3,1);
  }
  @keyframes fpop-in {
    from { opacity: 0; transform: translateY(-3px) scale(0.985); }
    to   { opacity: 1; transform: translateY(0)    scale(1); }
  }

  .fpop__opt {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    width: 100%;
    padding: 7px 9px;
    font-family: 'Inter', sans-serif;
    font-size: 9.5px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: rgba(95,70,54,0.78);
    background: transparent;
    border: none;
    border-radius: 2px;
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
    transition: background 0.14s, color 0.14s;
  }
  .fpop__opt:hover { background: rgba(52,37,28,0.05); color: #34251c; }
  .fpop__opt--sel {
    color: #6f3b24;
    background: rgba(198,95,60,0.07);
  }
  .fpop__opt--sel::before {
    content: '·';
    margin-right: -4px;
    color: #c65f3c;
  }

  .fpop__opt-label { flex: 1; white-space: nowrap; }
  .fpop__opt-count { opacity: 0.5; font-variant-numeric: tabular-nums; }
</style>
