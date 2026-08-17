import { api } from '$lib/api';
import { isInHouseQuery } from '$lib/keeper-search';
import type { FigurineListItem, SemanticHit } from '$lib/types/api';

/**
 * Shared voice of the keeper: the home blotter, the header loupe, and the
 * archive field all speak the same query. The panel is how a visitor asks
 * from anywhere — including the tenth work on the wall.
 */
class KeeperStore {
  query = $state('');
  panelOpen = $state(false);
  hits = $state<SemanticHit[] | null>(null);
  loading = $state(false);
  error = $state(false);
  figurines = $state.raw<FigurineListItem[]>([]);
  reelIds = $state.raw<string[]>([]);
  /** Bumped when the panel wants the blotter input focused. */
  focusSeq = $state(0);
  /** True once the in-flow cabinet blotter has scrolled under the header. */
  blotterOffscreen = $state(false);

  #seq = 0;
  #debounce: ReturnType<typeof setTimeout> | undefined;
  #fetching = false;

  seed(figurines: FigurineListItem[], reelIds: string[] = []) {
    this.figurines = figurines;
    this.reelIds = reelIds;
  }

  setQuery(q: string) {
    this.query = q;
    this.#schedule();
  }

  openPanel() {
    this.panelOpen = true;
    this.focusSeq += 1;
    void this.#ensureFigurines();
    this.#schedule();
  }

  closePanel() {
    this.panelOpen = false;
  }

  setBlotterOffscreen(off: boolean) {
    if (this.blotterOffscreen === off) return;
    this.blotterOffscreen = off;
    if (!off) this.panelOpen = false;
  }

  togglePanel() {
    if (this.panelOpen) this.closePanel();
    else this.openPanel();
  }

  runNow() {
    const q = this.query.trim();
    if (q.length < 2 || isInHouseQuery(q)) return;
    clearTimeout(this.#debounce);
    void this.#run(q);
  }

  async #ensureFigurines() {
    if (this.figurines.length > 0 || this.#fetching) return;
    this.#fetching = true;
    try {
      this.figurines = await api.getAllFigurines();
    } catch {
      /* blotter degrades to empty */
    } finally {
      this.#fetching = false;
    }
  }

  #schedule() {
    clearTimeout(this.#debounce);
    const q = this.query.trim();
    if (q.length < 2 || isInHouseQuery(q)) {
      this.hits = null;
      this.error = false;
      this.loading = false;
      this.#seq += 1;
      return;
    }
    this.loading = true;
    this.#debounce = setTimeout(() => void this.#run(q), 350);
  }

  async #run(q: string) {
    const seq = ++this.#seq;
    this.loading = true;
    this.error = false;
    try {
      const hits = await api.semanticSearch(q);
      if (seq !== this.#seq) return;
      this.hits = hits;
    } catch {
      if (seq !== this.#seq) return;
      this.error = true;
      this.hits = null;
    } finally {
      if (seq === this.#seq) this.loading = false;
    }
  }
}

export const keeper = new KeeperStore();
