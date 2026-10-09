<script lang="ts">
  /**
   * Знак смысла земли: что она делает с тем, кто на неё идёт.
   *
   * Рисунок говорит, ЧТО лежит на клетке, знак — ЧЕМ это обернётся, и формы у
   * знаков три, по смыслу, а не семь, по рисункам: запрет (тёмная печать с
   * крестом), опасность (треугольник), подмога (золотой круг). Различать семь
   * значков никто не станет; три формы читаются с одного взгляда и в клетке
   * шириной 140 px. Знак тот же на поле боя, в легенде рядом с полем и на
   * столе поля — иначе один и тот же смысл выглядел бы в трёх местах трижды.
   */
  import { GROUND_KIND } from '$lib/battles';
  import type { BattleGround } from '$lib/types/api';

  let { ground, size = '1.1em' }: { ground: BattleGround; size?: string } = $props();
  let kind = $derived(GROUND_KIND[ground]);
</script>

<svg
  class="sigil"
  viewBox="0 0 24 24"
  width={size}
  height={size}
  aria-hidden="true"
  focusable="false"
>
  {#if kind === 'block'}
    <rect x="2.5" y="2.5" width="19" height="19" rx="3.5" fill="#2a1f19" stroke="#d4b06a" stroke-width="1.6" />
    <path d="M8 8 L16 16 M16 8 L8 16" stroke="#f3e7cf" stroke-width="2.6" stroke-linecap="round" />
  {:else if kind === 'hazard'}
    <path d="M12 2.6 L22 20.4 H2 Z" fill="#9c3e26" stroke="#f2d9a6" stroke-width="1.5" stroke-linejoin="round" />
    <path d="M12 8.6 V14" stroke="#fbeeda" stroke-width="2.4" stroke-linecap="round" />
    <circle cx="12" cy="17.2" r="1.35" fill="#fbeeda" />
  {:else}
    <circle cx="12" cy="12" r="9.6" fill="#c9a254" stroke="#6f3b24" stroke-width="1.5" />
    <path d="M12 7.4 V16.6 M7.4 12 H16.6" stroke="#3a2416" stroke-width="2.4" stroke-linecap="round" />
  {/if}
</svg>

<style>
  .sigil {
    display: inline-block;
    flex-shrink: 0;
    vertical-align: -0.18em;
    filter: drop-shadow(0 1px 1.5px rgba(0, 0, 0, 0.5));
  }
</style>
