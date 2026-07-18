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
</script>

<div class="world-map">
  <svg viewBox="0 0 {WIDTH} {HEIGHT}" role="img" aria-label="Site visits by country">
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
          onclick={() => views > 0 && select(shape.alpha2!)}
          onkeydown={(e) => views > 0 && onKey(e, shape.alpha2!)}
        ><title>{tooltipFor(name, views)}</title></path>
      {:else}
        <path d={shape.d} class="country country--inert" style="fill:{NO_DATA_FILL}" />
      {/if}
    {/each}
  </svg>

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

  svg {
    width: 100%;
    height: auto;
    display: block;
  }

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
