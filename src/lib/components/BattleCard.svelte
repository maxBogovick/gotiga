<script lang="ts">
  // One card, rendered one way. The shelf, the keeper's preview and (later) the
  // moment of taking all draw this same component, because a preview that has
  // its own renderer is a preview that eventually lies.
  //
  // Size is never passed in. The card fills its container and reads its own
  // width with container queries, so the same component is a spine on a shelf
  // and a full card in a frame without a second set of styles.
  import type { BattleCard, BattleFrame } from '$lib/types/api';
  import { t, lang } from '$lib/i18n';
  import { cardCopy, frameFor, frameName, parseFocal, pricesOf, cardTransitionName } from '$lib/battles';
  import AppImage from '$lib/components/AppImage.svelte';

  let {
    card,
    frames = null,
    owned = false,
    level = null,
    isNew = false,
    interactive = true,
    transition = true,
  }: {
    card: BattleCard;
    frames?: BattleFrame[] | null;
    /** Face up or face down. A card you do not have lies in dust, price up. */
    owned?: boolean;
    /** The level of *your* copy, 1..5. Null while nobody owns anything. */
    level?: number | null;
    isNew?: boolean;
    /** Off in dense admin lists, where forty tilting cards help nobody. */
    interactive?: boolean;
    /**
     * Whether this card claims its shared-element name. Exactly one element per
     * page may carry it — a second one aborts the whole view transition — so a
     * preview rendered beside the shelf passes `false`.
     */
    transition?: boolean;
  } = $props();

  let frame = $derived(frameFor(card.tier, frames));
  let copy = $derived(cardCopy(card, $lang));
  let focal = $derived(parseFocal(card.artFocal));
  let prices = $derived(pricesOf(card));
  let rank = $derived(frameName(frame, $lang));

  // Pointer tilt and the foil sweep. Written as two custom properties rather
  // than an inline transform so the CSS below owns the whole effect: it can be
  // switched off wholesale by a media query, which an inline style cannot.
  let root = $state<HTMLElement | null>(null);
  let frameId = 0;

  function track(event: PointerEvent) {
    if (!interactive || !root) return;
    const el = root;
    const rect = el.getBoundingClientRect();
    const x = (event.clientX - rect.left) / rect.width;
    const y = (event.clientY - rect.top) / rect.height;
    // One write per frame. Pointermove fires far faster than the screen paints,
    // and every write here invalidates layout on a card that may be one of forty.
    cancelAnimationFrame(frameId);
    frameId = requestAnimationFrame(() => {
      el.style.setProperty('--mx', x.toFixed(3));
      el.style.setProperty('--my', y.toFixed(3));
    });
  }

  function rest() {
    cancelAnimationFrame(frameId);
    root?.style.setProperty('--mx', '0.5');
    root?.style.setProperty('--my', '0.5');
  }
</script>

<article
  bind:this={root}
  class="slot"
  data-tier={card.tier}
  style:--paper={frame.paper}
  style:--ink={frame.ink}
  style:--edge={frame.border}
  style:--foil={frame.foil || 'transparent'}
  style:--art-x="{(focal.x * 100).toFixed(1)}%"
  style:--art-y="{(focal.y * 100).toFixed(1)}%"
  style:--art-zoom={focal.zoom}
  style:view-transition-name={transition ? cardTransitionName(card) : undefined}
  onpointermove={track}
  onpointerleave={rest}
  aria-label="{copy.title} — {rank}"
>
 <div class="card" class:card--down={!owned} class:card--still={!interactive}>
  {#if owned}
    <!-- Cost, top left. In a fanned hand the left edge is the sliver you can
         actually see, which is where every game that is held in a hand puts it. -->
    <span class="corner corner--cost" title={$t('battlesCostLabel')}>{card.cost}</span>

    <div class="art">
      {#if card.artUrl}
        <AppImage src={card.artUrl} alt={copy.title} class="art-image" sizes="(max-width: 640px) 45vw, 260px" />
      {:else}
        <div class="art art--absent" aria-hidden="true"></div>
      {/if}
      <span class="foil" aria-hidden="true"></span>
    </div>

    <div class="plate">
      <h3 class="title">{copy.title}</h3>
      <p class="rank">{rank}</p>
    </div>

    {#if copy.effect}
      <p class="effect">{copy.effect}</p>
    {/if}

    {#if copy.lore}
      <p class="lore">{copy.lore}</p>
    {/if}

    <footer class="foot">
      <!-- Notches, not a number: at shelf size a digit disappears and a row of
           marks does not. This is the level of your copy — never the card's
           rank, which is worn as the frame itself. -->
      {#if level != null}
        <span class="pips" aria-label="{$t('battlesLevelLabel')}: {level}">
          {#each [1, 2, 3, 4, 5] as step (step)}
            <span class="pip" class:pip--lit={step <= level}></span>
          {/each}
        </span>
      {/if}
      <!-- Power, bottom right: the corner Magic has used for thirty years. -->
      <span class="corner corner--power" title={$t('battlesPowerLabel')}>{card.power}</span>
    </footer>

    {#if isNew}
      <span class="new-mark">{$t('battlesNew')}</span>
    {/if}
  {:else}
    <!-- Face down. Not greyed out: a card you do not have is a card lying in
         dust with its price still legible, which is also the room's price list. -->
    <div class="back" aria-hidden="true"></div>
    <div class="back-copy">
      <p class="rank rank--down">{rank}</p>
      <h3 class="title title--down">{copy.title}</h3>
      <ul class="prices">
        {#each prices as price (price.coin)}
          <li class="price">
            <span class="price-amount">{price.amount}</span>
            <span class="price-coin">
              {price.coin === 'dust' ? $t('battlesCoinDust') : $t('battlesCoinFeed')}
            </span>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
 </div>
</article>

<style>
  /* The card reads its own width, so one component serves every size it is ever
     drawn at, and every measurement below is in cqi for the same reason.
     The container must be a SEPARATE element from the one that uses the units:
     an element cannot size itself with its own container units, so a padding in
     cqi on the container itself silently resolves against the page instead. */
  .slot {
    container-type: inline-size;
    position: relative;
    aspect-ratio: 5 / 7;
    --mx: 0.5;
    --my: 0.5;
  }

  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 5cqi;
    background: var(--paper);
    color: var(--ink);
    border: 1px solid var(--edge);
    box-shadow:
      inset 0 0 0 2cqi var(--paper),
      inset 0 0 0 calc(2cqi + 1px) var(--edge),
      0 2px 14px rgba(52, 37, 28, 0.14);
    font-family: Georgia, 'Fraunces', serif;
    transition: transform 420ms cubic-bezier(0.22, 1, 0.36, 1);
    transform-style: preserve-3d;
    will-change: transform;
  }

  /* Rank shows as heavier paper and a heavier edge, never as a brighter colour:
     nothing in this house glows. */
  .slot[data-tier='5'] .card {
    box-shadow:
      inset 0 0 0 2cqi var(--paper),
      inset 0 0 0 calc(2cqi + 1.5px) var(--edge),
      0 3px 22px rgba(52, 37, 28, 0.3);
  }

  .slot:hover .card:not(.card--still) {
    /* Small on purpose. A card that leaps is a card in a shop window. */
    transform: perspective(900px)
      rotateY(calc((var(--mx) - 0.5) * 7deg))
      rotateX(calc((0.5 - var(--my)) * 7deg));
  }

  .corner {
    z-index: 2;
    display: grid;
    place-items: center;
    width: 13cqi;
    height: 13cqi;
    font-size: 7cqi;
    line-height: 1;
    color: var(--paper);
    background: var(--ink);
    border-radius: 50%;
  }

  .corner--cost {
    position: absolute;
    top: 3cqi;
    left: 3cqi;
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: auto;
    padding-top: 3cqi;
  }

  .corner--power {
    margin-left: auto;
    background: var(--edge);
    color: var(--ink);
  }

  .art {
    position: relative;
    flex: 0 0 44%;
    overflow: hidden;
    background: color-mix(in oklab, var(--ink) 8%, var(--paper));
  }

  .art :global(.art-image) {
    width: 100%;
    height: 100%;
  }

  .art :global(.app-image-main) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: var(--art-x) var(--art-y);
    transform: scale(var(--art-zoom));
  }

  .art--absent {
    background: repeating-linear-gradient(
      45deg,
      color-mix(in oklab, var(--ink) 6%, var(--paper)) 0 6px,
      var(--paper) 6px 12px
    );
  }

  /* One slow sweep, following the pointer. Blank at rank 1 and 2, where --foil
     is transparent — a humble card has no foil at all. */
  .foil {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: radial-gradient(
      circle at calc(var(--mx) * 100%) calc(var(--my) * 100%),
      var(--foil) 0%,
      transparent 55%
    );
    mix-blend-mode: soft-light;
    opacity: 0;
    transition: opacity 500ms ease;
  }

  .slot:hover .card:not(.card--still) .foil {
    opacity: 1;
  }

  .plate {
    flex: 0 0 auto;
    margin-top: 3cqi;
  }

  .title {
    margin: 0;
    color: var(--ink);
    font-size: 7cqi;
    line-height: 1.15;
    font-weight: 400;
    letter-spacing: 0.01em;
  }

  .rank {
    margin: 1cqi 0 0;
    color: var(--ink);
    font-size: 4cqi;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    opacity: 0.62;
  }

  .effect {
    /* Ends on a whole line instead of being sliced mid-letter by the card edge. */
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    flex: 0 0 auto;
    overflow: hidden;
    margin: 3cqi 0 0;
    color: var(--ink);
    padding-top: 2.5cqi;
    border-top: 1px solid color-mix(in oklab, var(--edge) 70%, transparent);
    font-size: 5cqi;
    line-height: 1.35;
  }

  @container (max-width: 250px) {
    .lore {
      display: none;
    }
  }

  .lore {
    margin: 2cqi 0 0;
    color: var(--ink);
    font-size: 4.4cqi;
    line-height: 1.4;
    font-style: italic;
    opacity: 0.66;
  }

  .pips {
    display: flex;
    gap: 1.4cqi;
  }

  .pip {
    width: 5cqi;
    height: 1.4cqi;
    background: color-mix(in oklab, var(--ink) 18%, transparent);
  }

  .pip--lit {
    background: var(--ink);
  }

  .new-mark {
    position: absolute;
    top: 3cqi;
    right: 3cqi;
    padding: 1cqi 2.4cqi;
    font-size: 3.6cqi;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: var(--paper);
    background: #c65f3c;
  }

  /* The dusty back. */
  .card--down {
    background: color-mix(in oklab, var(--ink) 10%, var(--paper));
  }

  .back {
    position: absolute;
    inset: 0;
    background:
      repeating-linear-gradient(
        135deg,
        color-mix(in oklab, var(--ink) 7%, transparent) 0 2px,
        transparent 2px 9px
      );
    opacity: 0.7;
  }

  .back-copy {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    height: 100%;
    text-align: center;
  }

  .title--down {
    font-size: 6cqi;
    color: var(--ink);
    opacity: 0.78;
  }

  .rank--down {
    margin: 0 0 auto;
    padding-top: 6cqi;
  }

  .prices {
    display: flex;
    flex-direction: column;
    gap: 1.2cqi;
    margin: 4cqi 0 0;
    padding: 0;
    list-style: none;
  }

  .price {
    display: flex;
    align-items: baseline;
    justify-content: center;
    gap: 1.6cqi;
  }

  .price-amount {
    font-size: 6.4cqi;
    color: var(--ink);
  }

  .price-coin {
    font-size: 3.6cqi;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    opacity: 0.66;
  }

  /* A tilting, sweeping card is decoration; the card without it is the whole
     card. So the effect is removed rather than slowed. */
  @media (prefers-reduced-motion: reduce) {
    .card,
    .slot:hover .card:not(.card--still) {
      transform: none;
      transition: none;
    }

    .foil {
      display: none;
    }
  }
</style>
