<script lang="ts" module>
  /**
   * Край капли сургуча. Считается один раз и без случайности: страница
   * пререндерится, и печать, нарисованная на сервере иначе, чем в браузере,
   * дрогнула бы при оживлении.
   */
  function waxEdge(): string {
    const n = 15;
    const at = (i: number) => {
      const a = (i / n) * Math.PI * 2 - Math.PI / 2;
      const r = 17.4 + [1.2, -0.7, 0.4][i % 3] + (i % 5 === 0 ? 0.5 : 0);
      return [20 + r * Math.cos(a), 20 + r * Math.sin(a)];
    };
    const mid = (i: number) => {
      const [x0, y0] = at(i);
      const [x1, y1] = at(i + 1);
      return [(x0 + x1) / 2, (y0 + y1) / 2];
    };
    const f = (v: number) => v.toFixed(2);
    const [sx, sy] = mid(0);
    let d = `M${f(sx)} ${f(sy)}`;
    for (let i = 1; i <= n; i++) {
      const [cx, cy] = at(i);
      const [mx, my] = mid(i);
      d += ` Q${f(cx)} ${f(cy)} ${f(mx)} ${f(my)}`;
    }
    return `${d}Z`;
  }
  const EDGE = waxEdge();
</script>

<script lang="ts">
  /**
   * Сургучная печать на прочитанной небылице. Та же печать, что дом ставит
   * при завершении любого дела, только маленькая: это след, а не событие.
   */
  let {
    size = 30,
    pressed = false,
    label = '',
  }: {
    size?: number;
    /** Печать ставится сейчас — один раз, на глазах. */
    pressed?: boolean;
    /** Подпись для читающих экран. Пусто — печать молчит, как украшение. */
    label?: string;
  } = $props();

  const uid = $props.id();
</script>

<svg
  class="seal"
  class:pressed
  viewBox="0 0 40 40"
  width={size}
  height={size}
  role={label ? 'img' : undefined}
  aria-label={label || undefined}
  aria-hidden={label ? undefined : 'true'}
>
  <defs>
    <radialGradient id="wax-{uid}" cx="36%" cy="30%" r="75%">
      <stop offset="0%" stop-color="#dd8262" />
      <stop offset="48%" stop-color="#b04e30" />
      <stop offset="100%" stop-color="#6f3b24" />
    </radialGradient>
  </defs>
  <path d={EDGE} fill="url(#wax-{uid})" />
  <!-- Оттиск: кольцо и звезда дома, вдавленные, а не нарисованные. -->
  <circle cx="20" cy="20" r="11.2" fill="none" stroke="rgba(60, 22, 10, 0.42)" stroke-width="1.1" />
  <circle cx="20.5" cy="20.6" r="11.2" fill="none" stroke="rgba(255, 214, 190, 0.22)" stroke-width="0.6" />
  <path
    d="M20 12.6 L21.7 18.3 L27.4 20 L21.7 21.7 L20 27.4 L18.3 21.7 L12.6 20 L18.3 18.3 Z"
    fill="rgba(60, 22, 10, 0.4)"
  />
  <path
    d="M20.4 13.2 L21.9 18.5 L26.9 20.3"
    fill="none"
    stroke="rgba(255, 214, 190, 0.28)"
    stroke-width="0.5"
  />
</svg>

<style>
  .seal {
    display: block;
    flex-shrink: 0;
    filter: drop-shadow(0 1px 1.5px rgba(52, 37, 28, 0.35));
  }

  /* Печать опускается на лист и чуть проминается — один раз. */
  .pressed {
    animation: seal-press 0.7s cubic-bezier(0.2, 0.9, 0.3, 1.2) both;
  }
  @keyframes seal-press {
    0% { transform: scale(1.6) rotate(-14deg); opacity: 0; }
    60% { transform: scale(0.94) rotate(3deg); opacity: 1; }
    100% { transform: scale(1) rotate(0); opacity: 1; }
  }

  @media (prefers-reduced-motion: reduce) {
    .pressed { animation: none; }
  }
</style>
