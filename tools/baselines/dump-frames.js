// Эталон стола рамок. Вставить в консоль на /admin#battles → вкладка «Рамки».
// Обходит пять чинов, снимает геометрию каждой детали резьбы и печатает JSON.
// Сравнивать с tools/baselines/frames-before-extract.json обычным diff-ом.
window.__dump = () => {
  const cards = [...document.querySelectorAll('.card')].filter(c => c.getBoundingClientRect().width > 120);
  const card = cards.sort((a, b) => b.getBoundingClientRect().width - a.getBoundingClientRect().width)[0];
  if (!card) return { error: 'карта не найдена' };
  const cr = card.getBoundingClientRect();
  const r2 = n => Math.round(n * 100) / 100;
  const base = u => (u || 'none').replace(/.*\//, '').replace(/["')]/g, '').slice(0, 40);
  const pieces = [...card.querySelectorAll('[data-piece]')].map(el => {
    const b = el.getBoundingClientRect(), s = getComputedStyle(el);
    return { p: el.dataset.piece, sd: el.dataset.side,
      x: r2((b.left - cr.left) / cr.width * 100), y: r2((b.top - cr.top) / cr.height * 100),
      w: r2(b.width / cr.width * 100), h: r2(b.height / cr.height * 100),
      z: s.zIndex, img: base(s.backgroundImage), bs: s.backgroundSize,
      tr: s.transform === 'none' ? '' : s.transform };
  });
  const cs = getComputedStyle(card);
  return { w: Math.round(cr.width), h: Math.round(cr.height), ratio: r2(cr.width / cr.height),
    bg: cs.backgroundColor, n: pieces.length, pieces };
};
(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const tiers = [...document.querySelectorAll('button')].filter(b => /^[1-5]\s+\S/.test(b.textContent.trim()));
  const out = { taken: new Date().toISOString(), tiers: {} };
  for (const b of tiers) { b.click(); await sleep(800); out.tiers[b.textContent.trim()] = window.__dump(); }
  console.log(JSON.stringify(out, null, 1));
})();
