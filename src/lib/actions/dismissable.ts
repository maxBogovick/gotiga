// Всплывающая панель, которую закрывают одинаково везде: Escape (фокус
// возвращается на кнопку, открывшую её), нажатие вне панели, уход фокуса
// клавишей Tab за её пределы. Прежде это было пять отдельных слушателей
// `click` на документе, а Escape знал только две панели из шести.
//
// Узел — обёртка, в которой лежат и кнопка, и панель. Если панель стоит
// отдельно от кнопки, вторая названа в `inside`: нажатие по ней панель не
// закрывает (это переключатель, он решает сам).

export type DismissableOptions = {
  open: boolean;
  /** `refocus` истинен, когда закрыли с клавиатуры и фокус надо вернуть на кнопку. */
  close: (refocus: boolean) => void;
  /** Кнопка, на которую возвращается фокус; по умолчанию первая кнопка узла. */
  trigger?: () => HTMLElement | null | undefined;
  /** Что ещё считается «внутри» (кнопка при панели, стоящей отдельно). */
  inside?: () => (Node | null | undefined)[];
  /** Перенести фокус на панель (`[data-panel]`) при открытии. */
  focusPanel?: boolean;
};

export function dismissable(node: HTMLElement, initial: DismissableOptions) {
  let opts = initial;
  let on = false;

  function isInside(target: Node | null): boolean {
    if (!target) return false;
    if (node.contains(target)) return true;
    return (opts.inside?.() ?? []).some((el) => el?.contains(target));
  }

  function onClick(e: MouseEvent) {
    if (!isInside(e.target as Node)) opts.close(false);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key !== 'Escape' || e.defaultPrevented) return;
    e.stopPropagation();
    opts.close(true);
    const back = opts.trigger?.() ?? node.querySelector<HTMLElement>('button');
    back?.focus();
  }

  function onFocusOut(e: FocusEvent) {
    // Нет следующего элемента — это уход окна или щелчок по пустому месту: его
    // обработает нажатие. Здесь только переход фокуса на что-то вне панели.
    const to = e.relatedTarget as Node | null;
    if (to && !isInside(to)) opts.close(false);
  }

  function sync() {
    if (opts.open && !on) {
      on = true;
      document.addEventListener('click', onClick, { capture: true });
      document.addEventListener('keydown', onKey);
      node.addEventListener('focusout', onFocusOut);
      if (opts.focusPanel) {
        setTimeout(() => node.querySelector<HTMLElement>('[data-panel]')?.focus(), 0);
      }
    } else if (!opts.open && on) {
      detach();
    }
  }

  function detach() {
    on = false;
    document.removeEventListener('click', onClick, { capture: true });
    document.removeEventListener('keydown', onKey);
    node.removeEventListener('focusout', onFocusOut);
  }

  sync();

  return {
    update(next: DismissableOptions) {
      opts = next;
      sync();
    },
    destroy() {
      if (on) detach();
    },
  };
}
