// While an étude is being played the cabinet chrome (header, footer, the
// offset under them) has no business taking height from the board. The étude
// page covers; the layout listens. A class on <html> would have done the same
// job, but then the offset on <main> would have been a step behind.

let covering = $state(false);

export const matchChrome = {
  get covering(): boolean {
    return covering;
  },
  cover() {
    covering = true;
  },
  uncover() {
    covering = false;
  },
};
