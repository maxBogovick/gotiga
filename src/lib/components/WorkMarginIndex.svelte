<script lang="ts">
  import { fade } from 'svelte/transition';
  import AppImage from '$lib/components/AppImage.svelte';
  import { t } from '$lib/i18n';
  import type { FigurineListItem } from '$lib/types/api';

  type Props = {
    /** The reel, in reel order. */
    figurines: FigurineListItem[];
    /** How many panes the reader has already left behind, from the top. */
    count: number;
  };

  let { figurines, count }: Props = $props();

  let seen = $derived(figurines.slice(0, Math.max(0, count)));

  const ROMAN_MAP: [number, string][] = [
    [1000, 'M'], [900, 'CM'], [500, 'D'], [400, 'CD'],
    [100, 'C'], [90, 'XC'], [50, 'L'], [40, 'XL'],
    [10, 'X'], [9, 'IX'], [5, 'V'], [4, 'IV'], [1, 'I'],
  ];

  function toRoman(num: number): string {
    let n = num;
    let out = '';
    for (const [value, sym] of ROMAN_MAP) {
      while (n >= value) { out += sym; n -= value; }
    }
    return out || 'I';
  }
</script>

<!-- The margin of the catalogue: works already passed are entered here, in the
     order they were read. The plates link back UP the reel — never to the detail
     page — so this stays an index of where you have been, not a second shelf.
     Deliberately no view-transition-name: a duplicate `figurine-{id}` on the page
     would abort the card→detail morph. -->
<aside class="margin-index" aria-hidden={seen.length === 0}>
  {#if seen.length > 0}
    <div class="mi-inner" in:fade={{ duration: 500 }}>
      <p class="mi-title">{$t('homeReelIndexTitle')}</p>
      <span class="mi-rule" aria-hidden="true"></span>

      <ol class="mi-list">
        {#each seen as fig, i (fig.id)}
          <li in:fade={{ duration: 450 }}>
            <a class="mi-plate" href="#work-{fig.id}" title={fig.name}>
              <span class="mi-thumb">
                <AppImage src={fig.faceImageUrl} thumbUrl={fig.thumbUrl} alt="" class="mi-img" />
              </span>
              <span class="mi-roman">{toRoman(i + 1)}</span>
            </a>
          </li>
        {/each}
      </ol>
    </div>
  {/if}
</aside>

<style>
  /* The column only exists where there is real margin to spare; below that the
     reel runs full width and this is not rendered at all (see the page). */
  .margin-index {
    position: relative;
    height: 100%;
  }

  .mi-inner {
    position: sticky;
    top: calc(var(--site-header-h, 54px) + 42px);
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }

  .mi-title {
    margin: 0;
    font-family: var(--font-display);
    font-size: 0.6rem;
    letter-spacing: 0.24em;
    text-transform: uppercase;
    color: var(--muted, #a0745a);
    opacity: 0.75;
  }

  .mi-rule {
    display: block;
    width: 2rem;
    height: 1px;
    background: linear-gradient(90deg, rgba(160, 116, 74, 0.5), transparent);
  }

  .mi-list {
    margin: 0.35rem 0 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  /* An engraved plate, not a floating thumbnail: 3px radius, an inner hairline,
     no drop shadow. It sits IN the paper. */
  .mi-plate {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    text-decoration: none;
    opacity: 0.55;
    transition: opacity 0.4s ease;
  }

  .mi-plate:hover,
  .mi-plate:focus-visible {
    opacity: 1;
  }

  .mi-thumb {
    display: block;
    width: 34px;
    height: 34px;
    flex-shrink: 0;
    border-radius: 3px;
    overflow: hidden;
    background: rgba(160, 116, 74, 0.12);
    box-shadow: inset 0 0 0 1px rgba(60, 25, 10, 0.14);
    /* Passed works are memory, not merchandise: they sit back a step in colour. */
    filter: saturate(0.55);
    transition: filter 0.4s ease;
  }

  .mi-plate:hover .mi-thumb,
  .mi-plate:focus-visible .mi-thumb {
    filter: none;
  }

  .mi-thumb :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .mi-roman {
    font-family: var(--font-display);
    font-size: 0.66rem;
    letter-spacing: 0.12em;
    color: var(--muted, #a0745a);
  }

  @media (prefers-reduced-motion: reduce) {
    .mi-plate,
    .mi-thumb {
      transition: none;
    }
  }
</style>
