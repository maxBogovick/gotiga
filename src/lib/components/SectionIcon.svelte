<script lang="ts" module>
  /** Знаки разделов и действий шапки. Один набор на шапку, нижнюю панель
   *  телефона и лист поиска: три набора одного и того же однажды разошлись бы. */
  export type SectionIconName =
    | 'works'
    | 'gazette'
    | 'tales'
    | 'game'
    | 'studio'
    | 'workshop'
    | 'author'
    | 'more'
    | 'search';
</script>

<script lang="ts">
  // Сетка 24, два начертания одной анатомии:
  //   · `line` — контур, линия 1.5;
  //   · `solid` — силуэт: заливка цветом текста, внутренние линии цветом того,
  //     на чём знак лежит (`--ib`, по умолчанию бумага).
  // Красный (`--em`) стоит только там, где он есть у самого предмета: пламя
  // свечи, бубны на карте, чернила пера, краска на кисти.
  let {
    name,
    variant = 'solid',
    size = 20,
  }: { name: SectionIconName; variant?: 'line' | 'solid'; size?: number } = $props();

  // Рваный низ листа вестника: зубцы через 1.6 по ширине листа.
  const TORN = (() => {
    const pts: string[] = [];
    const x0 = 5.5, x1 = 18.5, y = 19.2, n = 8, amp = 1.1;
    const step = (x1 - x0) / n;
    for (let i = 0; i <= n; i++) pts.push(`${(x1 - i * step).toFixed(2)} ${(y + (i % 2 ? amp : 0)).toFixed(2)}`);
    return pts.join(' L');
  })();
</script>

<svg
  class="section-icon"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if variant === 'line'}
    <g stroke="currentColor" stroke-width="1.5">
      {#if name === 'works'}
        <path d="M7.3 9.8c1.4.5 3 .7 4.7.7s3.3-.2 4.7-.7" /><path d="M9.8 9.6 12.3 3.2c1.3.1 2.4.5 3.4 1.4l-2 .9.8 4.1" /><path d="M10.7 10.8c-.5 2.4-1.2 4.8-2 7.1M13.3 10.8c.5 2.4 1.2 4.8 2 7.1" /><ellipse cx="12" cy="18.9" rx="5.8" ry="1.35" /><path d="M6.2 18.9v1.3c0 .75 2.6 1.35 5.8 1.35s5.8-.6 5.8-1.35v-1.3" />
      {:else if name === 'gazette'}
        <path d="M5.5 3.5h13v15.7 L{TORN}z" /><path d="M8 6.6h8v1.5H8z" fill="currentColor" stroke-width="1" /><path d="M8 10.8h3.2M12.8 10.8H16M8 13.4h3.2M12.8 13.4H16M8 16h3.2" />
      {:else if name === 'tales'}
        <path d="M15.3 11V5.9h2v5" /><path d="M16.3 4.4c-.8-.6-.8-1.6 0-2.6.8 1 .8 2 0 2.6z" fill="var(--em, #b44e24)" stroke="var(--em, #b44e24)" stroke-width=".8" /><path d="M12 19.2c-2.6-1.6-5.6-2-8.5-1.2V9.4c2.9-.8 5.9-.4 8.5 1.2 2.6-1.6 5.6-2 8.5-1.2V18c-2.9-.8-5.9-.4-8.5 1.2z" fill="var(--ib, #f8f1e7)" /><path d="M12 10.6v8.6" />
      {:else if name === 'game'}
        <g fill="var(--ib, #f8f1e7)"><rect x="8" y="5.4" width="8" height="11.6" rx="1.2" transform="rotate(-20 12 19.5)" /><rect x="8" y="5.4" width="8" height="11.6" rx="1.2" transform="rotate(20 12 19.5)" /><rect x="8" y="4.8" width="8" height="11.6" rx="1.2" /></g><path d="M12 8.3l1.7 2.3-1.7 2.3-1.7-2.3z" fill="var(--em, #b44e24)" stroke="var(--em, #b44e24)" stroke-width="1" />
      {:else if name === 'studio'}
        <path d="M19.6 3.4C13.4 4.6 9 9.2 7.7 15.6c6-.6 10.7-5 11.9-12.2z" /><path d="M17.3 6.1 5.2 19.8" /><path d="M13.6 7.6l-1.2 2.6M16.4 9.4l-2.6 1.1" /><circle cx="4.1" cy="21.1" r="1" fill="var(--em, #b44e24)" stroke="none" />
      {:else if name === 'workshop'}
        <path d="M18.6 18.8 9.6 9.8" /><path d="M9.6 9.8C8 8.2 6.9 6 7.3 5.2c.9-.4 3 .7 4.6 2.3" /><path d="M5.2 18.8 14.6 9.4" stroke="var(--ib, #f8f1e7)" stroke-width="3.6" /><path d="M5.2 18.8 14.6 9.4l1.7-1.7" /><path d="M16.3 7.7c1-1 2.6-1.9 3.8-2-.1 1.2-1 2.8-2 3.8z" fill="var(--em, #b44e24)" stroke="var(--em, #b44e24)" stroke-width="1" />
      {:else if name === 'author'}
        <ellipse cx="12" cy="12.8" rx="6.3" ry="8" /><circle cx="12" cy="10.8" r="2.3" /><path d="M8 17.8c.9-2.1 2.3-3.2 4-3.2s3.1 1.1 4 3.2" /><path d="M10.6 3.2h2.8" />
      {:else if name === 'more'}
        <path d="M6 10.6l1.4 1.4L6 13.4 4.6 12zM12 10.6l1.4 1.4-1.4 1.4-1.4-1.4zM18 10.6l1.4 1.4-1.4 1.4-1.4-1.4z" />
      {:else if name === 'search'}
        <circle cx="10.2" cy="10.2" r="6.2" /><path d="M7.1 8.4c.6-1.4 1.8-2.3 3.3-2.5" opacity=".55" /><path d="M14.8 14.8 20 20" /><path d="M14.4 16.6l2.2-2.2" stroke="var(--ib, #f8f1e7)" stroke-width="2.6" /><path d="M14.4 16.6l2.2-2.2" />
      {/if}
    </g>
  {:else if name === 'works'}
    <path d="M9.6 9.6 12.2 2.8c1.4.1 2.6.6 3.6 1.5l-2.1.9.9 4.4z" fill="currentColor" /><path d="M7.2 9.8c1.5.55 3.1.8 4.8.8s3.3-.25 4.8-.8" stroke="currentColor" stroke-width="1.7" /><path d="M10.5 10.9h3l1.7 7H8.8z" fill="currentColor" /><path d="M6 18.6c0-.9 2.7-1.5 6-1.5s6 .6 6 1.5v1.6c0 .9-2.7 1.5-6 1.5s-6-.6-6-1.5z" fill="currentColor" /><path d="M6.4 18.9c1.2.6 3.2.9 5.6.9s4.4-.3 5.6-.9" stroke="var(--ib, #f8f1e7)" stroke-width="1" />
  {:else if name === 'gazette'}
    <path d="M5.5 3.5h13v15.7 L{TORN}z" fill="currentColor" /><path d="M8 7.3h8" stroke="var(--ib, #f8f1e7)" stroke-width="1.8" /><path d="M8 10.8h3.2M12.8 10.8H16M8 13.4h3.2M12.8 13.4H16M8 16h3.2" stroke="var(--ib, #f8f1e7)" stroke-width="1.2" />
  {:else if name === 'tales'}
    <path d="M15.2 11V5.7h2.2V11z" fill="currentColor" /><path d="M16.3 4.5c-.9-.6-.9-1.7 0-2.8.9 1.1.9 2.2 0 2.8z" fill="var(--em, #b44e24)" /><path d="M12 19.4c-2.6-1.6-5.6-2-8.6-1.2V9.2c3-.8 6-.4 8.6 1.2 2.6-1.6 5.6-2 8.6-1.2v9c-3-.8-6-.4-8.6 1.2z" fill="currentColor" stroke="var(--ib, #f8f1e7)" stroke-width="1.1" /><path d="M12 11v8" stroke="var(--ib, #f8f1e7)" stroke-width="1.2" />
  {:else if name === 'game'}
    <g fill="currentColor" stroke="var(--ib, #f8f1e7)" stroke-width="1.3"><rect x="8" y="5.4" width="8" height="11.6" rx="1.2" transform="rotate(-20 12 19.5)" /><rect x="8" y="5.4" width="8" height="11.6" rx="1.2" transform="rotate(20 12 19.5)" /><rect x="8" y="4.8" width="8" height="11.6" rx="1.2" /></g><path d="M12 8.1l1.8 2.5-1.8 2.5-1.8-2.5z" fill="var(--em, #b44e24)" stroke="var(--ib, #f8f1e7)" stroke-width=".8" />
  {:else if name === 'studio'}
    <path d="M19.8 3.2C13.4 4.4 8.9 9.1 7.5 15.8c6.2-.6 11-5.1 12.3-12.6z" fill="currentColor" /><path d="M17.4 6 9.4 15.1" stroke="var(--ib, #f8f1e7)" stroke-width="1.1" /><path d="M9.6 14.9 5.2 19.8" stroke="currentColor" stroke-width="1.7" /><circle cx="4" cy="21.2" r="1.1" fill="var(--em, #b44e24)" />
  {:else if name === 'workshop'}
    <path d="M18.6 18.8 10 10.2" stroke="currentColor" stroke-width="1.8" /><path d="M10.3 10.5C8.5 8.7 6.7 6 7.2 5.1c.9-.5 3.6 1.3 5.4 3.1z" fill="currentColor" /><path d="M5.2 18.8 15 9" stroke="var(--ib, #f8f1e7)" stroke-width="4" /><path d="M5.2 18.8 15 9" stroke="currentColor" stroke-width="1.8" /><path d="M14.4 9.6l1.9-1.9 1.4 1.4-1.9 1.9z" fill="currentColor" /><path d="M16.6 7.4c1-1 2.6-1.9 3.8-2-.1 1.2-1 2.8-2 3.8z" fill="var(--em, #b44e24)" />
  {:else if name === 'author'}
    <ellipse cx="12" cy="12.8" rx="6.6" ry="8.4" fill="currentColor" /><circle cx="12" cy="10.7" r="2.3" fill="var(--ib, #f8f1e7)" /><path d="M7.6 18.2c.9-2.4 2.4-3.6 4.4-3.6s3.5 1.2 4.4 3.6c-1.2 1.2-2.7 1.9-4.4 1.9s-3.2-.7-4.4-1.9z" fill="var(--ib, #f8f1e7)" /><path d="M10.6 2.6h2.8" stroke="currentColor" stroke-width="1.6" />
  {:else if name === 'more'}
    <path d="M6 10.4l1.6 1.6L6 13.6 4.4 12zM12 10.4l1.6 1.6-1.6 1.6-1.6-1.6zM18 10.4l1.6 1.6-1.6 1.6-1.6-1.6z" fill="currentColor" />
  {:else if name === 'search'}
    <circle cx="10.2" cy="10.2" r="6" stroke="currentColor" stroke-width="2" /><path d="M14.8 14.8 20 20" stroke="currentColor" stroke-width="2.4" />
  {/if}
</svg>

<style>
  .section-icon {
    display: block;
    flex: none;
  }
</style>
