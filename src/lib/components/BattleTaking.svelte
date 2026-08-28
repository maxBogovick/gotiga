<script lang="ts">
  // Момент получения — и есть продукт (`TASKS-BATTLES.md` §0.6).
  //
  // Три такта, в языке дома: печать, поворот карты лицом с одним медленным
  // бликом фольги, и карта встаёт на полку с пометкой «новая». Ни конфетти, ни
  // фанфар, ни счётчиков — этого в доме нет нигде и здесь не появится.
  //
  // При `prefers-reduced-motion` тактов нет: карта просто есть, с оттиснутой
  // печатью. Человек с этой настройкой не теряет ничего, кроме времени.
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t } from '$lib/i18n';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import WaxSeal from '$lib/components/WaxSeal.svelte';
  import type { BattleCard as BattleCardDto, BattleFrame } from '$lib/types/api';

  let {
    card,
    frames = null,
    onclose,
  }: {
    card: BattleCardDto;
    frames?: BattleFrame[] | null;
    onclose: () => void;
  } = $props();

  /** Такт: 0 — печать, 1 — карта повернулась, 2 — всё сказано. */
  let beat = $state(0);
  let calm = $state(false);

  onMount(() => {
    calm = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (calm) {
      beat = 2;
      return;
    }
    const a = setTimeout(() => (beat = 1), 900);
    const b = setTimeout(() => (beat = 2), 2000);
    return () => {
      clearTimeout(a);
      clearTimeout(b);
    };
  });

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window onkeydown={onkey} />

<!-- Нажатие мимо карты закрывает: церемония не удерживает, она случается. -->
<div
  class="veil"
  role="button"
  tabindex="-1"
  aria-label={$t('battlesTakenClose')}
  onclick={onclose}
  onkeydown={(e) => e.key === 'Enter' && onclose()}
  transition:fade={{ duration: 250 }}
>
  <div class="stage">
    {#if beat === 0}
      <div class="seal-spot" out:fade={{ duration: 300 }}>
        <WaxSeal size="9rem" />
      </div>
    {:else}
      <div class="card" class:card--turned={true} class:card--calm={calm}>
        <BattleCard {card} {frames} owned={true} level={1} isNew={true} transition={false} interactive={false} />
        {#if !calm}
          <span class="foil" aria-hidden="true"></span>
        {/if}
      </div>

      {#if beat === 2}
        <p class="word" in:fade={{ duration: 400 }}>{$t('battlesTakenWord')}</p>
      {/if}
    {/if}
  </div>
</div>

<style>
  .veil {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    background: rgba(52, 37, 28, 0.55);
    backdrop-filter: blur(2px);
    cursor: default;
  }

  .stage {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.25rem;
  }

  .seal-spot {
    display: grid;
    place-items: center;
  }

  .card {
    position: relative;
    width: min(17rem, 62vw);
  }

  /* Второй такт: карта поворачивается лицом. Поворот, а не появление, — она
     лежала рубашкой вверх на полке, и это то же самое движение. */
  .card--turned {
    animation: turn 900ms cubic-bezier(0.2, 0.8, 0.25, 1) both;
    transform-style: preserve-3d;
  }

  .card--calm {
    animation: none;
  }

  @keyframes turn {
    from { transform: perspective(1200px) rotateY(-92deg); }
    to { transform: perspective(1200px) rotateY(0deg); }
  }

  /* Блик фольги проходит один раз и не возвращается. */
  .foil {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: linear-gradient(
      105deg,
      transparent 38%,
      rgba(255, 249, 240, 0.5) 47%,
      rgba(255, 249, 240, 0.16) 54%,
      transparent 62%
    );
    background-size: 260% 100%;
    animation: sweep 1400ms ease-out 700ms both;
  }

  @keyframes sweep {
    from { background-position: 160% 0; }
    to { background-position: -60% 0; }
  }

  .word {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 1.05rem;
    color: #f8f1e7;
    text-align: center;
  }

  @media (prefers-reduced-motion: reduce) {
    .card--turned {
      animation: none;
    }
  }
</style>
