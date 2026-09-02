<script lang="ts">
  // Три печати у пальца: на поле, в руку, лист.
  //
  // Не панель действий и не витринные кнопки. Стоят в странице там, где
  // нажали — как полоска рамки на столе хранителя, — и разлетаются дугой
  // прочь от карты, чтобы не закрыть её лицо. `position: absolute`, не
  // `fixed`: иначе прокрутка уносит стол, а печати остаются на стекле.
  // `prefers-reduced-motion` — просто появляются.
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';

  let {
    origin,
    from,
    canField = true,
    canHand = true,
    onfield,
    onhand,
    onread,
    onclose,
  }: {
    origin: { x: number; y: number };
    /** Центр карты, от которой веер уходит. Без него — влево, к полю. */
    from?: { x: number; y: number } | null;
    canField?: boolean;
    canHand?: boolean;
    onfield: () => void;
    onhand: () => void;
    onread: () => void;
    onclose: () => void;
  } = $props();

  const R = 86;
  const PAD = 64;

  let calm = $state(false);
  onMount(() => {
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    calm = mq.matches;
    const listen = () => (calm = mq.matches);
    mq.addEventListener('change', listen);
    const onkey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onclose();
        e.stopPropagation();
      }
    };
    window.addEventListener('keydown', onkey);
    return () => {
      mq.removeEventListener('change', listen);
      window.removeEventListener('keydown', onkey);
    };
  });

  type Mark = {
    id: 'field' | 'hand' | 'read';
    x: number;
    y: number;
    dx: number;
    dy: number;
    tilt: number;
    delay: number;
    ok: boolean;
    label: string;
  };

  let marks = $derived.by<Mark[]>(() => {
    const ox = origin.x;
    const oy = origin.y;
    let base = Math.PI;
    if (from) {
      const vx = ox - from.x;
      const vy = oy - from.y;
      if (Math.hypot(vx, vy) >= 10) base = Math.atan2(vy, vx);
    }
    const spread = (48 * Math.PI) / 180;
    const tilts = [-9, 6, -4];
    const ids: Mark['id'][] = ['field', 'hand', 'read'];
    const ok = [canField, canHand, true];
    const labels = [$t('battlesTableToField'), $t('battlesTableToHand'), $t('battlesTableReadCard')];
    return ids.map((id, i) => {
      const a = base + (i - 1) * spread;
      const x = ox + Math.cos(a) * R;
      const y = Math.max(PAD, oy + Math.sin(a) * R);
      return {
        id,
        x,
        y,
        dx: x - ox,
        dy: y - oy,
        tilt: tilts[i] ?? 0,
        delay: i * 55,
        ok: ok[i] ?? true,
        label: labels[i] ?? '',
      };
    });
  });

  function act(id: Mark['id'], ok: boolean) {
    if (!ok) return;
    if (id === 'field') onfield();
    else if (id === 'hand') onhand();
    else onread();
  }
</script>

<div class="cluster" data-hot-marks class:cluster--calm={calm}>
  {#each marks as m (m.id)}
    <button
      type="button"
      data-hot-marks
      class="mark"
      class:mark--off={!m.ok}
      style="left:{m.x}px; top:{m.y}px; --dx:{m.dx}px; --dy:{m.dy}px; --delay:{m.delay}ms"
      disabled={!m.ok}
      aria-label={m.label}
      onclick={() => act(m.id, m.ok)}
    >
      <span class="stamp">
        <span class="disc" style="--tilt:{m.tilt}deg" aria-hidden="true">
        {#if m.id === 'field'}
          <svg viewBox="0 0 24 24" fill="none">
            <rect x="3.5" y="5.5" width="17" height="13" rx="1.2" stroke="currentColor" stroke-width="1.4" />
            <path d="M3.5 12h17M9.2 5.5v13M14.8 5.5v13" stroke="currentColor" stroke-width="1.2" />
            <rect x="9.6" y="12.4" width="4.8" height="5.6" fill="currentColor" opacity="0.35" />
          </svg>
        {:else if m.id === 'hand'}
          <svg viewBox="0 0 24 24" fill="none">
            <rect x="4" y="7" width="9.5" height="13" rx="1" stroke="currentColor" stroke-width="1.4" transform="rotate(-14 8.75 13.5)" />
            <rect x="8" y="5.5" width="9.5" height="13" rx="1" stroke="currentColor" stroke-width="1.4" />
            <rect x="11.5" y="7" width="9.5" height="13" rx="1" stroke="currentColor" stroke-width="1.4" transform="rotate(14 16.25 13.5)" />
          </svg>
        {:else}
          <svg viewBox="0 0 24 24" fill="none">
            <path
              d="M6.5 4.5h8.2l4.8 4.8V19.5H6.5V4.5Z"
              stroke="currentColor"
              stroke-width="1.4"
              stroke-linejoin="round"
            />
            <path d="M14.6 4.5v4.9H19.5" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" />
            <path d="M9 12.2h6.4M9 15.4h4.4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
          </svg>
        {/if}
      </span>
      <span class="word">{m.label}</span>
      </span>
    </button>
  {/each}
</div>

<style>
  .cluster {
    position: contents;
  }

  .mark {
    position: absolute;
    z-index: 40;
    display: flex;
    width: auto;
    min-width: 5.2rem;
    margin: 0;
    padding: 0;
    background: none;
    border: 0;
    cursor: pointer;
    transform: translate(-50%, -50%);
    animation: bloom 560ms cubic-bezier(0.16, 1.12, 0.3, 1) both;
    animation-delay: var(--delay);
  }

  .stamp {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.4rem;
    transition: transform 220ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .cluster--calm .mark {
    animation: none;
  }

  @keyframes bloom {
    from {
      opacity: 0;
      transform: translate(-50%, -50%) translate(calc(-1 * var(--dx)), calc(-1 * var(--dy)))
        scale(0.28);
    }
    to {
      opacity: 1;
      transform: translate(-50%, -50%);
    }
  }

  .mark:hover:not(:disabled) {
    z-index: 41;
  }

  .mark:hover:not(:disabled) .stamp {
    transform: scale(1.06);
  }

  .mark:focus-visible .disc {
    outline: 2px solid #c65f3c;
    outline-offset: 3px;
  }

  .mark--off {
    cursor: default;
    opacity: 0.45;
  }

  .mark--off .word {
    opacity: 0.7;
  }

  .disc {
    position: relative;
    display: grid;
    place-items: center;
    width: 2.85rem;
    height: 2.85rem;
    color: #34251c;
    background:
      radial-gradient(circle at 32% 28%, rgba(255, 252, 246, 0.9), transparent 46%),
      linear-gradient(145deg, #fbf6ee, #eddcc6 58%, #e4d0b6);
    border: 1px solid #d8c6b1;
    border-radius: 46% 54% 49% 51% / 52% 47% 53% 48%;
    transform: rotate(var(--tilt));
    box-shadow:
      inset 1px 1px 6px rgba(255, 255, 255, 0.45),
      inset -2px -2px 6px rgba(111, 59, 36, 0.08),
      0 6px 14px rgba(52, 37, 28, 0.28);
  }

  .disc::after {
    content: '';
    position: absolute;
    inset: 5px;
    border: 1px solid rgba(111, 59, 36, 0.2);
    border-radius: 50%;
    pointer-events: none;
  }

  .mark:hover:not(:disabled) .disc {
    color: #c65f3c;
    border-color: #c65f3c;
  }

  .disc svg {
    width: 1.15rem;
    height: 1.15rem;
  }

  .word {
    padding: 0.18rem 0.45rem 0.16rem;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.72rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    line-height: 1;
    text-transform: uppercase;
    white-space: nowrap;
    color: #34251c;
    background: #f8f1e7;
    border: 1px solid #d8c6b1;
    outline: 1px solid #d8c6b1;
    outline-offset: 2px;
    box-shadow: 0 3px 10px rgba(52, 37, 28, 0.22);
  }

  .mark:hover:not(:disabled) .word {
    color: #6f3b24;
    border-color: #c65f3c;
    outline-color: #c65f3c;
  }

  @media (prefers-reduced-motion: reduce) {
    .mark,
    .mark:hover:not(:disabled) {
      animation: none;
      transform: translate(-50%, -50%);
    }
  }
</style>
