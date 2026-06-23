// Page-turn intent for figurine ↔ figurine navigation.
//
// The detail page already lifts its main plate out of the root snapshot via
// `view-transition-name: figurine-{id}` (the card→detail morph). For prev/next
// paging we want the OPPOSITE: the whole leaf should turn as one piece, like a
// heavy page in an old book, so the plate must stay *inside* the root snapshot.
//
// A prev/next link "arms" a direction just before navigating; +layout.svelte
// reads it in onNavigate to (a) neutralise the figurine name on the outgoing
// plate and (b) drive the book-turn animation on ::view-transition-*(root).
// FigurineDetailView reads it so the *incoming* plate mounts without the name
// too. The layout disarms once the transition has finished.

export type PageTurnDirection = 'forward' | 'backward';

let direction = $state<PageTurnDirection | null>(null);

export const pageTurn = {
  /** Reactive: the armed direction, or null when no page-turn is in flight. */
  get direction(): PageTurnDirection | null {
    return direction;
  },
  /** Called from a prev/next link the instant before navigation begins. */
  arm(dir: PageTurnDirection) {
    direction = dir;
  },
  /** Called once the view transition has settled. */
  disarm() {
    direction = null;
  },
};
