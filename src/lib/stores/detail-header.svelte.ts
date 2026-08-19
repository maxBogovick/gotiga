// Leaf-header chrome for the figurine detail page.
//
// SiteHeader reads neighbours from page.data on its own (so the cornice can
// transform the instant the route resolves). Story / share live in
// FigurineDetailView — this store is the narrow pipe those actions travel
// through, plus the neighbour-hover peek that drives paper bleed-through
// on the current plate.

export type NeighborPeek = 'prev' | 'next' | null;

export type DetailHeaderTools = {
  storySaving: boolean;
  copied: boolean;
  openStoryModal: () => void;
  share: () => void;
};

let tools = $state<DetailHeaderTools | null>(null);
let peek = $state<NeighborPeek>(null);

export const detailHeader = {
  get active(): boolean {
    return tools !== null;
  },
  get storySaving(): boolean {
    return tools?.storySaving ?? false;
  },
  get copied(): boolean {
    return tools?.copied ?? false;
  },
  get peek(): NeighborPeek {
    return peek;
  },
  bind(next: DetailHeaderTools) {
    tools = next;
  },
  clear() {
    tools = null;
    peek = null;
  },
  setPeek(dir: NeighborPeek) {
    peek = dir;
  },
  openStoryModal() {
    tools?.openStoryModal();
  },
  share() {
    tools?.share();
  },
};
