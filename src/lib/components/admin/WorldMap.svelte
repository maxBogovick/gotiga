<script module lang="ts">
  // Module-level (computed once, shared by every instance): projecting 177
  // country outlines and joining them to ISO alpha-2 is pure/static work,
  // independent of the traffic data passed in as a prop.
  import { feature } from 'topojson-client';
  import { geoNaturalEarth1, geoPath } from 'd3-geo';
  import iso from 'iso-3166-1';
  // world-atlas ships three resolutions (10m/50m/110m); 110m is the
  // low-detail one — plenty for a dashboard widget, ~100KB vs. multiple MB.
  import worldTopology from 'world-atlas/countries-110m.json';

  const WIDTH = 960;
  const HEIGHT = 470;

  // world-atlas feature ids are ISO 3166-1 *numeric* codes; the GeoIP
  // lookups on the server (and everywhere else in this codebase) use
  // *alpha-2* — this is the join between the two.
  const numericToAlpha2 = new Map(iso.all().map((c) => [c.numeric, c.alpha2]));

  const topology = worldTopology as any;
  const featureCollection = feature(topology, topology.objects.countries) as any;

  const projection = geoNaturalEarth1().fitSize([WIDTH, HEIGHT], featureCollection);
  const pathGenerator = geoPath(projection);

  type CountryShape = { alpha2: string | null; name: string; d: string };

  /** Every landmass in the topology, geocoded once. Territories with no
   * ISO alpha-2 (Antarctica, disputed/unrecognized areas — Natural Earth
   * marks these id="-99") still render, just as inert "no data" shapes —
   * a world map with silent gaps where those should be reads as broken. */
  const shapes: CountryShape[] = featureCollection.features
    .map((f: any) => {
      const d = pathGenerator(f);
      if (!d) return null;
      const alpha2 = numericToAlpha2.get(String(f.id)) ?? null;
      const name = f.properties?.name ?? 'Unknown';
      return { alpha2, name, d } satisfies CountryShape;
    })
    .filter((s: CountryShape | null): s is CountryShape => s !== null);
</script>

<script lang="ts">
  export type GeoPoint = { key: string; views: number; uniqueVisitors: number };

  import { onDestroy } from 'svelte';

  let {
    data,
    selected = null,
    onSelect,
  }: {
    data: GeoPoint[];
    /** Currently-selected country (ISO alpha-2), or null. */
    selected?: string | null;
    onSelect?: (code: string | null) => void;
  } = $props();

  // Sequential ramp derived from the site's own accent (#c65f3c) / deep
  // (#6f3b24) tokens — one hue, monotone lightness, validated with
  // dataviz's validate_palette.js --ordinal (light-end contrast 2.12:1,
  // hue spread 4°) rather than eyeballed.
  const RAMP = ['#d79a85', '#cb795c', '#bd5733', '#974828', '#6f3b24'];
  const NO_DATA_FILL = '#ece1d1';

  let known = $derived(data.filter((d) => d.key !== 'unknown' && d.views > 0));
  let byAlpha2 = $derived(new Map(known.map((d) => [d.key.toUpperCase(), d])));
  let maxViews = $derived(Math.max(1, ...known.map((d) => d.views)));
  let totalViews = $derived(data.reduce((sum, d) => sum + d.views, 0));
  let unknownViews = $derived(data.find((d) => d.key === 'unknown')?.views ?? 0);

  const nf = new Intl.NumberFormat('en-US');

  /** Log-scaled bucket, not linear: visit counts across countries on a small
   * site are heavily skewed (one or two dominant countries, a long tail of
   * 1-2 visits) — a linear scale would paint almost everything the lightest
   * shade and defeat the point of a choropleth. */
  function bucketFill(views: number): string {
    if (views <= 0) return NO_DATA_FILL;
    const t = Math.log(views + 1) / Math.log(maxViews + 1);
    const idx = Math.min(RAMP.length - 1, Math.floor(t * RAMP.length));
    return RAMP[idx];
  }

  function select(alpha2: string) {
    onSelect?.(selected === alpha2 ? null : alpha2);
  }

  function onKey(e: KeyboardEvent, alpha2: string) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      select(alpha2);
    }
  }

  function countryName(alpha2: string, fallback: string): string {
    return iso.whereAlpha2(alpha2)?.country ?? fallback;
  }

  function tooltipFor(name: string, views: number): string {
    if (views <= 0) return `${name} — no visits in range`;
    const share = totalViews > 0 ? (views / totalViews) * 100 : 0;
    return `${name} — ${nf.format(views)} view${views === 1 ? '' : 's'} (${share.toFixed(1)}%)`;
  }

  // ── Zoom / pan — plain viewBox manipulation, no extra dependency. Wheel
  // zooms centered on the cursor; dragging pans; buttons cover the case
  // scroll-zoom isn't discoverable (trackpad-less mice, touch).
  const MAX_ZOOM = 8;
  let svgEl = $state<SVGSVGElement | null>(null);
  let vbX = $state(0);
  let vbY = $state(0);
  let vbW = $state(WIDTH);
  let vbH = $state(HEIGHT);
  let dragging = $state(false);
  /** Set once a drag moves more than a couple pixels — suppresses the
   * country click that would otherwise fire on pointerup after a pan. */
  let dragMoved = $state(false);
  let dragStartClientX = 0;
  let dragStartClientY = 0;
  let dragStartVbX = 0;
  let dragStartVbY = 0;

  // Raw pointermove/wheel events can fire far faster than the display can
  // paint (trackpads especially). Writing vbX/vbY/vbW/vbH straight from the
  // event handler forces a viewBox reflow + repaint of ~177 country paths
  // per event, which pegs a CPU core while dragging. Coalesce to one commit
  // per animation frame instead — every event still updates the pending
  // target, but only the latest one per frame actually touches $state.
  let rafId: number | null = null;
  let pendingWrite: (() => void) | null = null;

  function scheduleWrite(fn: () => void) {
    pendingWrite = fn;
    if (rafId == null) {
      rafId = requestAnimationFrame(() => {
        rafId = null;
        pendingWrite?.();
        pendingWrite = null;
      });
    }
  }

  onDestroy(() => {
    if (rafId != null) cancelAnimationFrame(rafId);
  });

  // Any time the data itself changes (a different figurine selected, or the
  // date range moved), start from the full view again rather than leaving
  // an old zoomed-in crop that may no longer be relevant.
  $effect(() => {
    void data;
    resetView();
  });

  function clampView() {
    vbW = Math.min(WIDTH, Math.max(WIDTH / MAX_ZOOM, vbW));
    vbH = vbW * (HEIGHT / WIDTH);
    vbX = Math.min(WIDTH - vbW, Math.max(0, vbX));
    vbY = Math.min(HEIGHT - vbH, Math.max(0, vbY));
  }

  function resetView() {
    vbX = 0;
    vbY = 0;
    vbW = WIDTH;
    vbH = HEIGHT;
  }

  function zoomAtViewboxPoint(px: number, py: number, factor: number) {
    const newW = vbW / factor;
    const newH = vbH / factor;
    vbX = px - ((px - vbX) / vbW) * newW;
    vbY = py - ((py - vbY) / vbH) * newH;
    vbW = newW;
    vbH = newH;
    clampView();
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    if (!svgEl) return;
    const rect = svgEl.getBoundingClientRect();
    const px = vbX + ((e.clientX - rect.left) / rect.width) * vbW;
    const py = vbY + ((e.clientY - rect.top) / rect.height) * vbH;
    const factor = e.deltaY < 0 ? 1.25 : 0.8;
    scheduleWrite(() => zoomAtViewboxPoint(px, py, factor));
  }

  function zoomButton(factor: number) {
    zoomAtViewboxPoint(vbX + vbW / 2, vbY + vbH / 2, factor);
  }

  function onPointerDown(e: PointerEvent) {
    dragging = true;
    dragMoved = false;
    dragStartClientX = e.clientX;
    dragStartClientY = e.clientY;
    dragStartVbX = vbX;
    dragStartVbY = vbY;
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging || !svgEl) return;
    const rect = svgEl.getBoundingClientRect();
    const dx = e.clientX - dragStartClientX;
    const dy = e.clientY - dragStartClientY;
    if (Math.abs(dx) > 3 || Math.abs(dy) > 3) dragMoved = true;
    scheduleWrite(() => {
      vbX = dragStartVbX - (dx / rect.width) * vbW;
      vbY = dragStartVbY - (dy / rect.height) * vbH;
      clampView();
    });
  }

  function onPointerUp() {
    dragging = false;
  }

  function selectUnlessDragged(alpha2: string) {
    if (dragMoved) return;
    select(alpha2);
  }
</script>

<div class="world-map">
  <div class="map-wrap">
    <svg
      bind:this={svgEl}
      viewBox="{vbX} {vbY} {vbW} {vbH}"
      role="img"
      aria-label="Site visits by country"
      class:dragging
      onwheel={onWheel}
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointerleave={onPointerUp}
    >
      {#each shapes as shape (shape.alpha2 ?? shape.name)}
        {#if shape.alpha2}
          {@const point = byAlpha2.get(shape.alpha2)}
          {@const views = point?.views ?? 0}
          {@const name = countryName(shape.alpha2, shape.name)}
          <path
            d={shape.d}
            class="country"
            class:has-data={views > 0}
            class:selected={selected === shape.alpha2}
            style="fill:{bucketFill(views)}"
            role="button"
            tabindex={views > 0 ? 0 : -1}
            aria-pressed={selected === shape.alpha2}
            onclick={() => views > 0 && selectUnlessDragged(shape.alpha2!)}
            onkeydown={(e) => views > 0 && onKey(e, shape.alpha2!)}
          ><title>{tooltipFor(name, views)}</title></path>
        {:else}
          <path d={shape.d} class="country country--inert" style="fill:{NO_DATA_FILL}" />
        {/if}
      {/each}
    </svg>

    <div class="map-zoom-controls">
      <button type="button" onclick={() => zoomButton(1.4)} aria-label="Zoom in" title="Zoom in">+</button>
      <button type="button" onclick={() => zoomButton(1 / 1.4)} aria-label="Zoom out" title="Zoom out">−</button>
      <button type="button" onclick={resetView} aria-label="Reset zoom" title="Reset zoom">⤾</button>
    </div>
  </div>

  <div class="map-foot">
    <div class="ramp-legend">
      <span class="ramp-label">Fewer</span>
      <i class="ramp-swatch" style="background:{NO_DATA_FILL}" title="No visits"></i>
      {#each RAMP as color}
        <i class="ramp-swatch" style="background:{color}"></i>
      {/each}
      <span class="ramp-label">More</span>
    </div>
    {#if unknownViews > 0}
      <span class="unresolved-note">+{nf.format(unknownViews)} visits with no resolved country</span>
    {/if}
  </div>
</div>

<style>
  .world-map {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .map-wrap {
    position: relative;
  }

  svg {
    width: 100%;
    height: auto;
    display: block;
    touch-action: none;
    cursor: grab;
  }

  svg.dragging { cursor: grabbing; }

  .map-zoom-controls {
    position: absolute;
    top: 0.6rem;
    right: 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .map-zoom-controls button {
    width: 1.6rem;
    height: 1.6rem;
    border: 1px solid #d8c6b1;
    background: #fff;
    color: #6f3b24;
    font-size: 0.9rem;
    line-height: 1;
    cursor: pointer;
  }

  .map-zoom-controls button:hover { background: #fbf3e7; }

  .country {
    stroke: #f8f1e7;
    stroke-width: 0.6;
    transition: filter 0.1s ease;
  }

  .country--inert {
    pointer-events: none;
  }

  .country.has-data {
    cursor: pointer;
  }

  .country.has-data:hover,
  .country.has-data:focus-visible {
    filter: brightness(1.12);
    stroke: #34251c;
    stroke-width: 1.1;
    outline: none;
  }

  .country.selected {
    stroke: #34251c;
    stroke-width: 1.6;
  }

  .map-foot {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.9rem;
  }

  .ramp-legend {
    display: flex;
    align-items: center;
    gap: 0.2rem;
  }

  .ramp-label {
    color: #8a6f5c;
    font-size: 0.64rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin: 0 0.25rem;
  }

  .ramp-swatch {
    display: inline-block;
    width: 0.9rem;
    height: 0.6rem;
  }

  .unresolved-note {
    color: #8a6f5c;
    font-size: 0.68rem;
    font-style: italic;
  }
</style>
