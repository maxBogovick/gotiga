// Ряд пунктов в колонке, ширина которой задана не им: если названия не
// помещаются, ряд переходит на знаки без подписей (`data-fit="icons"`).
//
// Ширина названий зависит от языка и от чисел рядом с ними, поэтому порог в
// CSS по ширине окна верен для одного языка и ошибается в остальных. Здесь
// ряд измеряется сам: сначала в полном виде, затем решение. Оба шага идут в
// одной задаче браузера, поэтому промежуточный вид не рисуется.
//
// Пункты, у которых `flex-grow` (распорки), в ширину не входят: они занимают
// то, что осталось, и мерка с ними никогда бы не сошлась.

const SPACER = '.seal-gap';

/** Запись о картинке печати (слой смены картинки, капля): на ширину названий не влияет. */
function onlyArt(r: MutationRecord): boolean {
  if (r.type !== 'childList') return false;
  // Svelte кладёт метки-комментарии рядом с вставляемыми блоками: они не в счёт.
  const nodes = [...r.addedNodes, ...r.removedNodes].filter((n) => n.nodeType !== Node.COMMENT_NODE);
  if (nodes.length === 0) return r.addedNodes.length + r.removedNodes.length > 0;
  return nodes.every((n) => n instanceof Element && (n.matches('img.art') || n.matches('.drop')));
}

export function fitRow(node: HTMLElement) {
  let running = false;

  function needed(): number {
    const kids = [...node.children].filter((el) => !el.matches(SPACER)) as HTMLElement[];
    let total = 0;
    let shown = 0;
    for (const el of kids) {
      const w = el.getBoundingClientRect().width;
      if (w > 0) {
        total += w;
        shown += 1;
      }
    }
    const gap = parseFloat(getComputedStyle(node).columnGap) || 0;
    return total + gap * Math.max(0, shown - 1);
  }

  // Без requestAnimationFrame: замер идёт в обратном вызове наблюдателя, то
  // есть до отрисовки, и вид без подгонки не успевает показаться ни на кадр.
  function fit() {
    if (running) return;
    running = true;
    try {
      measure();
    } finally {
      running = false;
    }
  }

  function measure() {
    // Ряд скрыт стилем (телефон): мерить нечего.
    if (!node.isConnected || node.clientWidth === 0) return;
    node.dataset.fit = 'full';
    const wide = needed();
    const icons = wide > node.clientWidth + 0.5;
    node.dataset.fit = icons ? 'icons' : 'full';
    // Без подписи пункту нужна подсказка по наведению.
    for (const el of node.querySelectorAll<HTMLElement>('[data-label]')) {
      if (icons) el.title = el.dataset.label ?? '';
      else if (!el.classList.contains('seal-link--minor')) el.removeAttribute('title');
    }
  }

  // Мерка не чаще раза в 100 мс: первое изменение мерится сразу (вид без подгонки
  // не успевает показаться ни на кадр), следующие, пока идёт переход, сливаются в
  // одну мерку после того, как всё затихло. Печати меняют размер переходом (шапка
  // сворачивается), и ряд при этом меняет ширину на каждом кадре; прежде мерка шла
  // на каждый кадр и на каждое окончание перехода, а это около сорока принудительных
  // пересчётов раскладки за одну свёртку. Мерка из середины перехода всё равно
  // неверна, поэтому последняя нужна именно после него.
  const GAP_MS = 100;
  const SETTLE_MS = 120;
  let last = -Infinity;
  let settle: ReturnType<typeof setTimeout> | undefined;
  function fitSoon() {
    clearTimeout(settle);
    const now = performance.now();
    if (now - last >= GAP_MS) {
      last = now;
      fit();
      return;
    }
    settle = setTimeout(() => {
      last = performance.now();
      fit();
    }, SETTLE_MS);
  }

  node.dataset.fit = 'full';
  const size = new ResizeObserver(fitSoon);
  size.observe(node);
  // Названия и числа меняются при смене языка и когда приходят числа. Смена
  // картинки на печати — тоже изменение дерева, но ширины названий не меняет:
  // такие записи не стоят замера.
  const text = new MutationObserver((records) => {
    if (records.some((r) => !onlyArt(r))) fitSoon();
  });
  text.observe(node, { childList: true, characterData: true, subtree: true });
  node.addEventListener('transitionend', fitSoon);
  fit();
  last = performance.now();

  return {
    destroy() {
      clearTimeout(settle);
      size.disconnect();
      text.disconnect();
      node.removeEventListener('transitionend', fitSoon);
    },
  };
}
