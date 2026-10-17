<script lang="ts">
  /**
   * Земля под клеткой — один рисунок на сцену боя и на стол поля.
   *
   * Отдельным компонентом потому, что местность рисуется в двух местах, а два
   * рисунка одной земли однажды разойдутся: хранитель поставит на столе одно,
   * а гость увидит другое.
   *
   * Рисунки — плитки из `tools/ground_tiles.py`: освещённая карта высот, а не
   * CSS-градиенты. Края плитки растворены в прозрачность, поэтому одна и та же
   * картинка ложится и на бумагу стола, и на тёмное сукно комнаты.
   *
   * В углу — знак смысла (`BattleGroundSigil`): рисунок говорит, что лежит на
   * клетке, знак — чем это обернётся. Без него кладка сверху читалась
   * мостовой, то есть подсказывала ровно обратное правилу.
   */
  import BattleGroundSigil from './BattleGroundSigil.svelte';
  import type { BattleGround } from '$lib/types/api';

  let {
    ground,
    occupied = false,
    motion,
  }: {
    ground: BattleGround;
    /** На клетке стоит тело. Земля под ним не видна, и над верхней кромкой
     *  карты встаёт медальон той же земли в золотой оправе — со знаком. */
    occupied?: boolean;
    /** Прежний переключатель тёмной комнаты. Плитки одинаковы на любом
     *  фоне — принимается и молчит, чтобы не править всех, кто его передаёт. */
    dark?: boolean;
    /** Насколько заметно движение подвижных слоёв (болото, родник, овраг,
     *  холм): `soft` — еле заметно, `bold` — заметно, `vivid` — заметно и с
     *  дополнительными деталями (капли, пылинки, порыв ветра). Не назван —
     *  берётся уровень земли по умолчанию. */
    motion?: 'soft' | 'bold' | 'vivid';
  } = $props();

  /** Выбор автора: везде `vivid`, у родника `bold` (без частого толчка в центре). */
  const level = $derived(motion ?? (ground === 'spring' ? 'bold' : 'vivid'));
</script>

{#if occupied}
  <span class="mark">
    <img src="/battles/ground/{ground}-mark.webp" alt="" aria-hidden="true" draggable="false" />
    <span class="mark-sigil"><BattleGroundSigil {ground} size="100%" /></span>
  </span>
{:else}
  <i class="tile m-{level}" style="background-image: url('/battles/ground/{ground}.webp')" aria-hidden="true">
    {#if ground === 'mire'}
      <i class="fx glint glint-a"></i>
      <i class="fx glint glint-b"></i>
      <i class="fx reeds reeds-l"></i>
      <i class="fx reeds reeds-r"></i>
      {#if level === 'vivid'}
        <i class="drop drop-1"></i>
        <i class="drop drop-2"></i>
        <i class="drop drop-3"></i>
      {/if}
    {:else if ground === 'wall'}
      <i class="fx ivy ivy-0"></i>
      <i class="fx ivy ivy-1"></i>
      <i class="fx wall-grass"></i>
      {#if level === 'vivid'}
        <i class="mote chip chip-1"></i>
        <i class="mote chip chip-2"></i>
        <i class="mote chip chip-3"></i>
      {/if}
    {:else if ground === 'cover'}
      <i class="fx cgrass cgrass-0"></i>
      <i class="fx cgrass cgrass-1"></i>
    {:else if ground === 'pit'}
      <i class="fx vapor vapor-a"></i>
      <i class="fx vapor vapor-b"></i>
      {#if level === 'vivid'}
        <i class="mote pmote pmote-1"></i>
        <i class="mote pmote pmote-2"></i>
        <i class="mote pmote pmote-3"></i>
      {/if}
    {:else if ground === 'ravine'}
      <i class="fx mist mist-a"></i>
      <i class="fx mist mist-b"></i>
      {#if level === 'vivid'}
        <i class="mote mote-1"></i>
        <i class="mote mote-2"></i>
        <i class="mote mote-3"></i>
      {/if}
    {:else if ground === 'hill'}
      <i class="fx wind wind-a"></i>
      <i class="fx wind wind-b"></i>
      <i class="fx grass grass-0"></i>
      <i class="fx grass grass-1"></i>
      <i class="fx grass grass-2"></i>
    {:else if ground === 'spring'}
      <i class="fx glint spring-glint"></i>
      <i class="fx wave wave-1"></i>
      <i class="fx wave wave-2"></i>
      <i class="fx wave wave-3"></i>
      {#if level === 'vivid'}
        <i class="fx wave wave-4"></i>
        <i class="blip"></i>
      {/if}
    {:else if ground === 'embers'}
      <i class="fx film film-fire"></i>
    {:else if ground === 'storm'}
      <i class="fx film film-sky"></i>
      <i class="fx film film-bolt"></i>
    {:else if ground === 'grave'}
      <i class="fx film film-spirit"></i>
    {:else if ground === 'crystal'}
      <i class="fx film film-light"></i>
    {:else if ground === 'glass'}
      <i class="fx film film-glass"></i>
    {:else if ground === 'rift'}
      <i class="fx film film-rift"></i>
    {:else if ground === 'maelstrom'}
      <i class="fx film film-water"></i>
    {:else if ground === 'altar'}
      <i class="fx a-pillar"></i>
      <i class="fx a-ring"></i>
      <i class="fx a-runes a-runes-a"></i>
      <i class="fx a-runes a-runes-b"></i>
      {#if level !== 'soft'}
        <i class="mote gleam gleam-1"></i>
        <i class="mote gleam gleam-2"></i>
      {/if}
      {#if level === 'vivid'}
        <i class="mote gleam gleam-3"></i>
        <i class="mote gleam gleam-4"></i>
      {/if}
    {:else if ground === 'ice'}
      <i class="fx i-mist"></i>
      <i class="fx i-sheen"></i>
      <i class="fx i-spark i-spark-a"></i>
      <i class="fx i-spark i-spark-b"></i>
      {#if level !== 'soft'}
        <i class="mote flake flake-1"></i>
        <i class="mote flake flake-2"></i>
      {/if}
      {#if level === 'vivid'}
        <i class="mote flake flake-3"></i>
        <i class="mote flake flake-4"></i>
      {/if}
    {/if}
    <span class="corner"><BattleGroundSigil {ground} size="100%" /></span>
  </i>
{/if}

<style>
  .tile {
    position: absolute;
    inset: 0;
    z-index: 0;
    pointer-events: none;
    background-size: 100% 100%;
    background-repeat: no-repeat;
    overflow: clip;   /* слои двигаются и не должны выезжать за клетку */
  }

  /* Круги названы `wave`, не `ring`: `.ring` — утилита Tailwind, она рисует рамку.
     Подвижные слои рисует `tools/ground_tiles.py` на полную силу, а сколько их
     видно и как быстро они идут, решают три уровня (`motion`): все числа лежат
     переменными на `.tile`, ключевые кадры одни. Без
     `prefers-reduced-motion: no-preference` слои стоят: блик виден, круги
     скрыты — застывшее кольцо читалось бы трещиной в камне. */
  .tile.m-soft {
    --tempo: 1;
    --fade-pk: 0.64; --fade-dy: 3px;
    --sway: 1.3deg; --grass: 1.3deg;
    --drift-pk: 0.5; --drift-dx: 5px;
    --wind-pk: 0.5;
    --breathe-lo: 0.35; --breathe-hi: 0.9;
    --wave-pk: 0.8;
    --vapor-pk: 0.25; --rise: 6px;
    --strip: 0.7; --glow-lo: 0.2; --glow-hi: 0.55; --rune-lo: 0.3;
  }
  .tile.m-bold {
    --tempo: 0.7;
    --fade-pk: 1; --fade-dy: 7px;
    --sway: 3.2deg; --grass: 3.4deg;
    --drift-pk: 0.85; --drift-dx: 14px;
    --wind-pk: 0.9;
    --breathe-lo: 0.45; --breathe-hi: 1;
    --wave-pk: 1;
    --vapor-pk: 0.4; --rise: 14px;
    --strip: 0.88; --glow-lo: 0.3; --glow-hi: 0.8; --rune-lo: 0.2;
  }
  .tile.m-vivid {
    --tempo: 0.5;
    --fade-pk: 1; --fade-dy: 11px;
    --sway: 5.2deg; --grass: 5.6deg;
    --drift-pk: 1; --drift-dx: 26px;
    --wind-pk: 1;
    --breathe-lo: 0.55; --breathe-hi: 1;
    --wave-pk: 1;
    --vapor-pk: 0.55; --rise: 24px;
    --strip: 1; --glow-lo: 0.35; --glow-hi: 1; --rune-lo: 0.12;
  }

  .fx {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background-size: 100% 100%;
    background-repeat: no-repeat;
  }
  .mist-a { background-image: url('/battles/ground/ravine-mist-a.webp'); opacity: 0.5; }
  .mist-b { background-image: url('/battles/ground/ravine-mist-b.webp'); opacity: 0; }
  .wind-a { background-image: url('/battles/ground/hill-wind-a.webp'); opacity: 0.4; }
  .wind-b { background-image: url('/battles/ground/hill-wind-b.webp'); opacity: 0; }
  /* Точка опоры — средняя высота пучков группы (печатает ground_tiles.py):
     иначе наклон двигал бы корни, а не верхушки. */
  .grass-0 { background-image: url('/battles/ground/hill-grass-0.webp'); transform-origin: 50% 31%; }
  .grass-1 { background-image: url('/battles/ground/hill-grass-1.webp'); transform-origin: 50% 66%; }
  .grass-2 { background-image: url('/battles/ground/hill-grass-2.webp'); transform-origin: 50% 85%; }
  /* Плющ висит на крышке стены: опора — высота, с которой начинаются плети. */
  .ivy-0 { background-image: url('/battles/ground/wall-ivy-0.webp'); transform-origin: 50% 41%; }
  .ivy-1 { background-image: url('/battles/ground/wall-ivy-1.webp'); transform-origin: 50% 28%; }
  .wall-grass { background-image: url('/battles/ground/wall-grass.webp'); transform-origin: 50% 88%; }
  .cgrass-0 { background-image: url('/battles/ground/cover-grass-0.webp'); transform-origin: 50% 60%; }
  .cgrass-1 { background-image: url('/battles/ground/cover-grass-1.webp'); transform-origin: 50% 90%; }
  .vapor-a { background-image: url('/battles/ground/pit-vapor-a.webp'); opacity: 0.2; }
  .vapor-b { background-image: url('/battles/ground/pit-vapor-b.webp'); opacity: 0; }
  .glint-a { background-image: url('/battles/ground/mire-glint-a.webp'); opacity: 0.5; }
  .glint-b { background-image: url('/battles/ground/mire-glint-b.webp'); opacity: 0; }
  .reeds-l { background-image: url('/battles/ground/mire-reeds-l.webp'); transform-origin: 50% 92%; }
  .reeds-r { background-image: url('/battles/ground/mire-reeds-r.webp'); transform-origin: 50% 92%; }
  .spring-glint { background-image: url('/battles/ground/spring-glint.webp'); opacity: 0.7; }
  .wave { background-image: url('/battles/ground/spring-ring.webp'); transform-origin: 50% 52%; opacity: 0; }

  /* Покадровые земли (`tools/ground_fx.py`): жар, гроза, могила, хрусталь.
     Не слои, которые CSS качает, а полосы из 16 кадров, которые он листает
     ступенчато: огонь горит, молния бьёт, души поднимаются — каждый кадр
     другой, а полоса замкнута, и шва между последним кадром и первым нет.
     Положение `p%` при ширине полосы в 16 ширин коробки ставит кадр
     `k = p·15/100`, а `steps(16)` от нуля до `E` выдаёт `k·E/16`, значит
     `E = 100·16/15` — то же рассуждение, что у `stripEnd` в `battles.ts`.
     Под `prefers-reduced-motion` полоса стоит на нулевом кадре: он нарочно
     спокойный (огонь тлеет, молнии нет). */
  .film {
    --n: 16;
    --end: 106.6667%;
    background-size: calc(var(--n) * 100%) 100%;
    background-position: 0 0;
    opacity: var(--strip, 1);
  }
  /* Медленному свету нужно больше кадров: при шестнадцати он ходил бы
     ступеньками. Конец — `100·n/(n−1) %`, как у шестнадцати. */
  .film-light,
  .film-glass,
  .film-rift {
    --n: 32;
    --end: 103.2258%;
  }
  .film-fire { background-image: url('/battles/ground/embers-fire.webp'); }
  .film-sky { background-image: url('/battles/ground/storm-sky.webp'); }
  .film-bolt { background-image: url('/battles/ground/storm-bolt.webp'); mix-blend-mode: screen; }
  .film-spirit { background-image: url('/battles/ground/grave-spirit.webp'); }
  .film-light { background-image: url('/battles/ground/crystal-light.webp'); }
  .film-glass { background-image: url('/battles/ground/glass-light.webp'); }
  .film-rift { background-image: url('/battles/ground/rift-portal.webp'); }
  .film-water { background-image: url('/battles/ground/maelstrom-water.webp'); }

  /* Алтарь. В покое руны еле теплятся, круг и столб гаснут. */
  .a-pillar { background-image: url('/battles/ground/altar-pillar.webp'); opacity: 0.18; transform-origin: 50% 55%; }
  .a-ring { background-image: url('/battles/ground/altar-ring.webp'); transform-origin: 50% 55%; opacity: 0; }
  .a-runes-a { background-image: url('/battles/ground/altar-runes-a.webp'); opacity: 0.45; }
  .a-runes-b { background-image: url('/battles/ground/altar-runes-b.webp'); opacity: 0.2; }
  .mote.gleam { background: #f4e6b8; box-shadow: 0 0 5px 1px rgba(233, 214, 255, 0.85); --fall: -78px; }
  .mote.gleam-1 { left: 47%; top: 58%; }
  .mote.gleam-2 { left: 53%; top: 55%; }
  .mote.gleam-3 { left: 43%; top: 60%; }
  .mote.gleam-4 { left: 57%; top: 61%; }

  /* Лёд. Блик скользит маской (как порыв ветра на холме), искры мигают
     по очереди, холодный пар стелется. */
  .i-mist { background-image: url('/battles/ground/ice-mist.webp'); opacity: 0.35; }
  .i-sheen { background-image: url('/battles/ground/ice-sheen.webp'); opacity: 0; }
  .i-spark-a { background-image: url('/battles/ground/ice-spark-a.webp'); opacity: 0.8; }
  .i-spark-b { background-image: url('/battles/ground/ice-spark-b.webp'); opacity: 0; }
  .mote.flake { width: 4px; height: 4px; background: #f3fbff; box-shadow: 0 0 4px 1px rgba(214, 240, 252, 0.9); --fall: 62px; }
  .mote.flake-1 { left: 30%; top: 24%; }
  .mote.flake-2 { left: 62%; top: 20%; }
  .mote.flake-3 { left: 46%; top: 30%; }
  .mote.flake-4 { left: 74%; top: 34%; }

  /* Детали только уровня `vivid`: они не слои, а CSS-фигуры. */
  .drop {
    position: absolute;
    width: 20%;
    aspect-ratio: 4 / 3;
    border: 1.5px solid rgba(216, 221, 196, 0.85);
    border-radius: 50%;
    opacity: 0;
    pointer-events: none;
  }
  .drop-1 { left: 34%; top: 40%; }
  .drop-2 { left: 64%; top: 58%; }
  .drop-3 { left: 44%; top: 74%; }
  .mote {
    position: absolute;
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: #cdbfa3;
    opacity: 0;
    pointer-events: none;
  }
  .mote-1 { left: 27%; top: 44%; }
  .mote-2 { left: 52%; top: 43%; }
  .mote-3 { left: 74%; top: 46%; }
  .chip { background: #b9ab92; --fall: 56px; }
  .chip-1 { left: 24%; top: 36%; }
  .chip-2 { left: 46%; top: 37%; }
  .chip-3 { left: 72%; top: 30%; }
  .pmote { background: #a89a82; --fall: 26px; }
  .pmote-1 { left: 38%; top: 47%; }
  .pmote-2 { left: 56%; top: 45%; }
  .pmote-3 { left: 47%; top: 49%; }
  .blip {
    position: absolute;
    left: 50%;
    top: 52%;
    width: 7%;
    aspect-ratio: 4 / 3;
    border: 1.5px solid rgba(226, 243, 241, 0.9);
    border-radius: 50%;
    opacity: 0;
    pointer-events: none;
  }

  @media (prefers-reduced-motion: no-preference) {
    .ivy-0 { animation: gm-sway calc(6.8s * var(--tempo)) ease-in-out infinite; }
    .ivy-1 { animation: gm-sway calc(8.1s * var(--tempo)) ease-in-out calc(-2.7s * var(--tempo)) infinite reverse; }
    .wall-grass { animation: gm-grass calc(5.2s * var(--tempo)) ease-in-out infinite; }
    .cgrass-0 { animation: gm-grass calc(5.8s * var(--tempo)) ease-in-out infinite; }
    .cgrass-1 { animation: gm-grass calc(4.7s * var(--tempo)) ease-in-out calc(-2.4s * var(--tempo)) infinite; }
    .vapor-a { animation: gm-rise calc(10s * var(--tempo)) ease-out infinite; }
    .vapor-b { animation: gm-rise calc(10s * var(--tempo)) ease-out calc(-5s * var(--tempo)) infinite; }
    .mist-a { animation: gm-drift calc(14s * var(--tempo)) ease-in-out infinite; }
    .mist-b { animation: gm-drift calc(14s * var(--tempo)) ease-in-out calc(-7s * var(--tempo)) infinite reverse; }
    .wind-a { animation: gm-wind calc(8s * var(--tempo)) ease-in-out infinite; }
    .wind-b { animation: gm-wind calc(8s * var(--tempo)) ease-in-out calc(-4s * var(--tempo)) infinite; }
    .grass-0 { animation: gm-grass calc(5.4s * var(--tempo)) ease-in-out infinite; }
    .grass-1 { animation: gm-grass calc(6.3s * var(--tempo)) ease-in-out calc(-2.1s * var(--tempo)) infinite; }
    .grass-2 { animation: gm-grass calc(4.9s * var(--tempo)) ease-in-out calc(-3.6s * var(--tempo)) infinite; }
    .glint-a { animation: gm-fade calc(9s * var(--tempo)) ease-in-out infinite; }
    .glint-b { animation: gm-fade calc(9s * var(--tempo)) ease-in-out calc(-4.5s * var(--tempo)) infinite; }
    .reeds-l { animation: gm-sway calc(6.5s * var(--tempo)) ease-in-out infinite; }
    .reeds-r { animation: gm-sway calc(7.7s * var(--tempo)) ease-in-out calc(-3s * var(--tempo)) infinite reverse; }
    .spring-glint { animation: gm-breathe calc(5.5s * var(--tempo)) ease-in-out infinite; }
    .wave { animation: gm-ring calc(7.2s * var(--tempo)) ease-out infinite; }
    .wave-2 { animation-delay: calc(-2.4s * var(--tempo)); }
    .wave-3 { animation-delay: calc(-4.8s * var(--tempo)); }
    .wave-4 { animation-delay: calc(-6s * var(--tempo)); }
    .blip { animation: gm-blip calc(3.2s * var(--tempo)) ease-out infinite; }
    .film { animation: gm-film var(--film-dur, 1.4s) steps(var(--n)) infinite; }
    .film-fire { --film-dur: 1.35s; }
    .film-sky { --film-dur: 1.8s; }
    .film-bolt { --film-dur: 2.4s; }
    .film-spirit { --film-dur: 3s; }
    .film-light { --film-dur: 4.4s; }
    .film-glass { --film-dur: 4.8s; }
    .film-rift { --film-dur: 3.6s; }
    .film-water { --film-dur: 2.8s; }
    .a-pillar { animation: gm-pillar calc(5.2s * var(--tempo)) ease-in-out infinite; }
    .a-ring { animation: gm-ring calc(6.4s * var(--tempo)) ease-out infinite; }
    .a-runes-a { animation: gm-rune calc(4.4s * var(--tempo)) ease-in-out infinite; }
    .a-runes-b { animation: gm-rune calc(4.4s * var(--tempo)) ease-in-out calc(-2.2s * var(--tempo)) infinite; }
    .mote.gleam { animation: gm-mote calc(5.4s * var(--tempo)) ease-out infinite; }
    .mote.gleam-2 { animation-delay: calc(-1.8s * var(--tempo)); }
    .mote.gleam-3 { animation-delay: calc(-3.1s * var(--tempo)); }
    .mote.gleam-4 { animation-delay: calc(-4.3s * var(--tempo)); }
    .i-mist { animation: gm-drift calc(12s * var(--tempo)) ease-in-out infinite; }
    .i-sheen {
      opacity: 0.95;
      -webkit-mask-image: linear-gradient(105deg, transparent 38%, #000 50%, transparent 62%);
      mask-image: linear-gradient(105deg, transparent 38%, #000 50%, transparent 62%);
      -webkit-mask-size: 300% 100%;
      mask-size: 300% 100%;
      -webkit-mask-repeat: no-repeat;
      mask-repeat: no-repeat;
      animation: gm-glide calc(7s * var(--tempo)) ease-in-out infinite;
    }
    .i-spark-a { animation: gm-twinkle calc(3.6s * var(--tempo)) ease-in-out infinite; }
    .i-spark-b { animation: gm-twinkle calc(3.6s * var(--tempo)) ease-in-out calc(-1.8s * var(--tempo)) infinite; }
    .mote.flake { animation: gm-mote calc(6.5s * var(--tempo)) linear infinite; }
    .mote.flake-2 { animation-delay: calc(-2.2s * var(--tempo)); }
    .mote.flake-3 { animation-delay: calc(-4.1s * var(--tempo)); }
    .mote.flake-4 { animation-delay: calc(-5.3s * var(--tempo)); }
    .drop { animation: gm-drop calc(6s * var(--tempo)) ease-out infinite; }
    .drop-2 { animation-delay: calc(-2s * var(--tempo)); }
    .drop-3 { animation-delay: calc(-4s * var(--tempo)); }
    .mote { animation: gm-mote calc(7s * var(--tempo)) ease-in infinite; }
    .mote-2 { animation-delay: calc(-2.5s * var(--tempo)); }
    .mote-3 { animation-delay: calc(-5s * var(--tempo)); }
    .chip-2 { animation-delay: calc(-3s * var(--tempo)); }
    .chip-3 { animation-delay: calc(-5.5s * var(--tempo)); }
    .pmote-2 { animation-delay: calc(-3.2s * var(--tempo)); }
    .pmote-3 { animation-delay: calc(-5.8s * var(--tempo)); }
    /* Порыв ветра: полоса света, проходящая по склону слева направо. */
    .m-vivid .wind {
      -webkit-mask-image: linear-gradient(100deg, transparent 35%, #000 50%, transparent 65%);
      mask-image: linear-gradient(100deg, transparent 35%, #000 50%, transparent 65%);
      -webkit-mask-size: 300% 100%;
      mask-size: 300% 100%;
      -webkit-mask-repeat: no-repeat;
      mask-repeat: no-repeat;
      animation: gm-wind calc(8s * var(--tempo)) ease-in-out infinite, gm-gust calc(5s * var(--tempo)) ease-in-out infinite;
    }
    .m-vivid .wind-b { animation-delay: calc(-4s * var(--tempo)), calc(-2.5s * var(--tempo)); }
  }

  @keyframes gm-fade {
    0%, 100% { opacity: 0; transform: translateY(0); }
    50% { opacity: var(--fade-pk); transform: translateY(var(--fade-dy)); }
  }
  @keyframes gm-drift {
    0%, 100% { opacity: 0; transform: translateX(calc(var(--drift-dx) * -1)); }
    50% { opacity: var(--drift-pk); transform: translateX(var(--drift-dx)); }
  }
  @keyframes gm-wind {
    0%, 100% { opacity: 0; }
    50% { opacity: var(--wind-pk); }
  }
  @keyframes gm-rise {
    0% { opacity: 0; transform: translateY(0); }
    30% { opacity: var(--vapor-pk); }
    100% { opacity: 0; transform: translateY(calc(var(--rise) * -1)); }
  }
  @keyframes gm-gust {
    0% { -webkit-mask-position: 100% 0; mask-position: 100% 0; }
    100% { -webkit-mask-position: 0% 0; mask-position: 0% 0; }
  }
  @keyframes gm-sway {
    0%, 100% { transform: skewX(calc(var(--sway) * -0.9)); }
    50% { transform: skewX(var(--sway)); }
  }
  @keyframes gm-grass {
    0%, 100% { transform: skewX(calc(var(--grass) * -0.9)); }
    50% { transform: skewX(var(--grass)); }
  }
  @keyframes gm-breathe {
    0%, 100% { opacity: var(--breathe-lo); }
    50% { opacity: var(--breathe-hi); }
  }
  @keyframes gm-ring {
    0% { transform: scale(0.12); opacity: var(--wave-pk); }
    70% { opacity: calc(var(--wave-pk) * 0.3); }
    100% { transform: scale(1); opacity: 0; }
  }
  @keyframes gm-blip {
    0% { transform: translate(-50%, -50%) scale(0.2); opacity: 0.9; }
    100% { transform: translate(-50%, -50%) scale(1.8); opacity: 0; }
  }
  @keyframes gm-drop {
    0% { transform: translate(-50%, -50%) scale(0.1); opacity: 0.9; }
    80% { opacity: 0.2; }
    100% { transform: translate(-50%, -50%) scale(1); opacity: 0; }
  }
  @keyframes gm-film {
    to { background-position-x: var(--end); }
  }
  /* Столб света дышит  /* Руны вспыхивают и успокаиваются; соседние половины перекликаются. */
  @keyframes gm-rune {
    0%, 100% { opacity: var(--rune-lo); }
    45% { opacity: 1; }
    70% { opacity: 0.55; }
  }
  /* Блик ходит по плите и отдыхает: маска проходит и стоит на краю. */
  @keyframes gm-glide {
    0% { -webkit-mask-position: 100% 0; mask-position: 100% 0; }
    45%, 100% { -webkit-mask-position: 0% 0; mask-position: 0% 0; }
  }
  /* Иней вспыхивает один раз за круг, остальное время молчит. */
  @keyframes gm-twinkle {
    0%, 100% { opacity: 0; }
    14% { opacity: 1; }
    34% { opacity: 0; }
  }
  @keyframes gm-mote {
    0% { transform: translateY(0); opacity: 0; }
    15% { opacity: 0.9; }
    100% { transform: translateY(var(--fall, 40px)); opacity: 0; }
  }

  /* Знак в верхнем левом углу: там у пустой клетки ничего нет, а у занятой
     он уезжает к медальону. Величина — доля клетки, но не мельче, чем
     читается глазом, и не крупнее, чем закрывает рисунок. */
  .corner {
    position: absolute;
    top: 6%;
    left: 7%;
    width: clamp(14px, 22%, 26px);
    aspect-ratio: 1;
    display: flex;
  }

  /* Медальон над картой: посередине верхней кромки, где у карты нет чисел. */
  .mark {
    position: absolute;
    z-index: 3;
    top: -9px;
    left: 50%;
    width: clamp(18px, 26%, 30px);
    aspect-ratio: 1;
    transform: translateX(-50%);
    pointer-events: none;
    filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.55));
  }

  .mark img {
    display: block;
    width: 100%;
    height: 100%;
  }

  .mark-sigil {
    position: absolute;
    right: -42%;
    bottom: -14%;
    width: 62%;
    aspect-ratio: 1;
    display: flex;
  }
</style>
