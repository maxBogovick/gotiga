<script lang="ts">
  /**
   * HouseGazette — the blotter on the hall table.
   *
   * Two piles, never a news feed: handwritten house leaves (a work laid out,
   * a showing, a guest's tale) and yellowed cuttings the keeper pinned from
   * the world of art. Unpinned cuttings stay in the private desk. Hides entirely
   * when both piles are empty. World cuttings leave the house; house leaves stay
   * inside it.
   */
  import { t, lang, type TranslationKey } from '$lib/i18n';
  import type { GazetteCutting, GazetteKind, GazetteLeaf } from '$lib/types/api';
  import { expectedWhisper, leafCopy, leafHref, quietDate, workHref, leafCoverUrl } from '$lib/gazette';
  import AppImage from '$lib/components/AppImage.svelte';

  let {
    leaves = [],
    cuttings = [],
  }: {
    leaves?: GazetteLeaf[];
    cuttings?: GazetteCutting[];
  } = $props();

  const KIND_KEY: Record<GazetteKind, TranslationKey> = {
    arrival: 'gazetteKind_arrival',
    collage: 'gazetteKind_collage',
    showing: 'gazetteKind_showing',
    guest_story: 'gazetteKind_guest_story',
    tale: 'gazetteKind_tale',
    note: 'gazetteKind_note',
    world: 'gazetteKind_world',
    sketch: 'gazetteKind_sketch',
  };

  let show = $derived(leaves.length > 0 || cuttings.length > 0);
</script>

{#if show}
  <section class="gazette" aria-labelledby="gazette-title">
    <div class="gz-head">
      <div>
        <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('gazetteEyebrow')}</p>
        <h2 id="gazette-title" class="gz-title">{$t('gazetteTitle')}</h2>
      </div>
      <p class="gz-lead">{$t('gazetteLead')}</p>
    </div>

    <div class="gz-board">
      {#if leaves.length > 0}
        <div class="gz-col gz-col-house">
          <p class="gz-col-label">{$t('gazetteHouseCol')}</p>
          <ul class="gz-leaves">
            {#each leaves as leaf, i (leaf.id)}
              {@const copy = leafCopy(leaf, $lang)}
              {@const work = workHref(leaf, 'home_gazette')}
              {@const cover = leafCoverUrl(leaf)}
              {@const when = expectedWhisper(
                leaf,
                $lang,
                (d) => $t('gazetteExpectedAround').replace('{date}', d),
                (a, b) => $t('gazetteExpectedRange').replace('{from}', a).replace('{to}', b),
              )}
              <li class="gz-leaf" style="--tilt: {(i % 2 === 0 ? -1.1 : 0.9)}deg">
                <a class="gz-leaf-link" href={leafHref(leaf, 'home_gazette')}>
                  {#if cover}
                    <span class="gz-leaf-face">
                      <AppImage src={cover} alt="" class="gz-leaf-img" loading="lazy" sizes="72px" />
                    </span>
                  {/if}
                  <span class="gz-leaf-meta">
                    <span class="gz-kind">{$t(KIND_KEY[leaf.kind])}</span>
                    {#if when}
                      <span class="gz-date">{when}</span>
                    {:else if quietDate(leaf.publishedAt ?? leaf.createdAt, $lang)}
                      <span class="gz-date">{quietDate(leaf.publishedAt ?? leaf.createdAt, $lang)}</span>
                    {/if}
                  </span>
                  <span class="gz-leaf-title">{copy.title}</span>
                  {#if copy.dek}
                    <span class="gz-leaf-dek">{copy.dek}</span>
                  {/if}
                </a>
                {#if work}
                  <a class="gz-work" href={work}>{$t('gazetteOpenWork')} →</a>
                {/if}
              </li>
            {/each}
          </ul>
        </div>
      {/if}

      {#if cuttings.length > 0}
        <div class="gz-col gz-col-world">
          <p class="gz-col-label">{$t('gazetteWorldCol')}</p>
          <ul class="gz-cuttings">
            {#each cuttings as cut, i (cut.id)}
              <li class="gz-cut" style="--tilt: {(i % 3 === 0 ? 1.4 : i % 3 === 1 ? -0.8 : 0.6)}deg">
                <a class="gz-cut-link" href={cut.url} target="_blank" rel="noopener noreferrer">
                  <span class="gz-cut-source">{$t('gazetteSource')} {cut.sourceName}</span>
                  <span class="gz-cut-title">{cut.title}</span>
                  <span class="gz-cut-out">{$t('gazetteLeavesHouse')} →</span>
                </a>
              </li>
            {/each}
          </ul>
        </div>
      {/if}
    </div>

    <p class="gz-foot">
      <a href="/gazette" class="gz-all">{$t('gazetteAllLeaves')} →</a>
    </p>
  </section>
{/if}

<style>
  .gazette {
    max-width: 1520px;
    margin: 0 auto;
    padding: clamp(28px, 4vw, 56px) clamp(20px, 4.5vw, 64px) clamp(12px, 2vw, 28px);
  }

  .gz-head {
    display: grid;
    grid-template-columns: minmax(220px, 0.42fr) minmax(0, 0.58fr);
    gap: clamp(18px, 2.4vw, 36px);
    align-items: end;
    margin-bottom: clamp(22px, 3vw, 40px);
  }

  .eyebrow {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.10em;
    text-transform: uppercase;
    color: var(--muted2, #5f4636);
    margin: 0 0 10px;
  }

  .eyebrow-rule {
    display: inline-block;
    width: 26px;
    height: 1px;
    background: var(--copper, #c65f3c);
    opacity: 0.65;
  }

  .gz-title {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(32px, 4.2vw, 52px);
    font-weight: 400;
    line-height: 1.05;
    color: var(--ink, #34251c);
    margin: 0;
  }

  .gz-lead {
    font-family: Georgia, 'Times New Roman', serif;
    font-size: clamp(15px, 1.5vw, 18px);
    line-height: 1.55;
    color: var(--muted2, #5f4636);
    margin: 0;
    max-width: 38em;
  }

  .gz-board {
    display: grid;
    grid-template-columns: 1.15fr 0.85fr;
    gap: clamp(22px, 3vw, 48px);
    align-items: start;
  }

  .gz-col-label {
    font-size: 11px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--copper, #c65f3c);
    margin: 0 0 14px;
  }

  .gz-leaves,
  .gz-cuttings {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .gz-leaf {
    background: #fbf6ee;
    border: 1px solid #d8c6b1;
    box-shadow:
      0 1px 0 rgba(52, 37, 28, 0.04),
      0 10px 24px -18px rgba(52, 37, 28, 0.35);
    padding: 16px 18px 14px;
    transform: rotate(var(--tilt));
    transform-origin: 20% 0;
  }

  .gz-leaf-link {
    display: grid;
    gap: 6px;
    color: inherit;
    text-decoration: none;
  }

  .gz-leaf-face {
    display: block;
    width: 72px;
    height: 90px;
    overflow: hidden;
    float: right;
    margin: 0 0 8px 14px;
    border: 1px solid #d8c6b1;
    background: #1a120e;
  }
  .gz-leaf-face :global(.app-image-wrap) {
    width: 100%;
    height: 100%;
  }

  .gz-leaf-face :global(.app-image-wrap),
  .gz-leaf-face :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .gz-leaf-meta {
    display: flex;
    gap: 10px;
    align-items: baseline;
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .gz-leaf-title {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(20px, 2.2vw, 26px);
    line-height: 1.15;
    color: #34251c;
  }

  .gz-leaf-dek {
    font-size: 14px;
    line-height: 1.5;
    color: #5f4636;
  }

  .gz-work {
    display: inline-block;
    margin-top: 10px;
    font-size: 12px;
    letter-spacing: 0.04em;
    color: #6f3b24;
    text-decoration: none;
    border-bottom: 1px solid transparent;
  }
  .gz-work:hover {
    border-bottom-color: #c65f3c;
  }

  .gz-cut {
    background:
      linear-gradient(180deg, #f3e6c9 0%, #ead9b0 100%);
    border: 1px solid #cbb48a;
    padding: 14px 16px 12px;
    transform: rotate(var(--tilt));
    box-shadow: 0 8px 18px -16px rgba(52, 37, 28, 0.5);
  }

  .gz-cut-link {
    display: grid;
    gap: 5px;
    color: inherit;
    text-decoration: none;
  }

  .gz-cut-source {
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .gz-cut-title {
    font-family: Georgia, 'Times New Roman', serif;
    font-size: 16px;
    line-height: 1.3;
    color: #2c2118;
  }

  .gz-cut-dek {
    font-size: 13px;
    line-height: 1.45;
    color: #5a4634;
  }

  .gz-cut-out {
    font-size: 11px;
    letter-spacing: 0.06em;
    color: #8a5a3a;
    margin-top: 4px;
  }

  .gz-foot {
    margin: 28px 0 0;
    text-align: right;
  }

  .gz-all {
    font-size: 13px;
    letter-spacing: 0.06em;
    color: #6f3b24;
    text-decoration: none;
    border-bottom: 1px solid #d8c6b1;
  }
  .gz-all:hover {
    border-bottom-color: #c65f3c;
  }

  @media (max-width: 860px) {
    .gz-head,
    .gz-board {
      grid-template-columns: 1fr;
    }
    .gz-leaf,
    .gz-cut {
      transform: none;
    }
  }
</style>
