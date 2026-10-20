<script lang="ts">
  // Три раздела сайта одним рядом: работы, байки, игра.
  //
  // До этого блока байки и игра были мелкими ссылками рядом с видео
  // мастерской (байки) или не стояли на главной вовсе (игра), и посетитель,
  // пришедший смотреть фигурки, о них не узнавал. Здесь три входа одного
  // размера, и каждый показывает своё содержимое, а не описание: последнюю
  // работу и первую байку полки. Игра — ссылкой словами: карты не работы,
  // и плитка с картами в одном ряду с работами выдавала их за работы.
  //
  // Блок не продаёт: ни счётчиков «осталось», ни «новинка!» (§ 1). Только то,
  // что есть, и куда это ведёт.
  //
  // Байки и список этюдов читаются в браузере (`loadWorld`): главная
  // пререндерится, а эти разделы меняются чаще, чем её собирают.
  import { onMount } from 'svelte';
  import { t, lang } from '$lib/i18n';
  import { interact } from '$lib/analytics';
  import AppImage from '$lib/components/AppImage.svelte';
  import { leafCopy, leafCoverUrl, leafHref } from '$lib/gazette';
  import { leadTale } from '$lib/tales';
  import { loadWorld, type World } from '$lib/world';
  import type { FigurineListItem } from '$lib/types/api';

  let { works, latest }: { works: number; latest: FigurineListItem | null } = $props();

  let world = $state<World | null>(null);
  onMount(() => {
    loadWorld()
      .then((w) => (world = w))
      .catch(() => {});
  });

  let tale = $derived(world ? leadTale(world.tales) : null);
  let taleCopy = $derived(tale ? leafCopy(tale, $lang) : null);
  let taleCover = $derived(tale ? leafCoverUrl(tale) : '');
  let latestFace = $derived(latest ? (latest.faceImageLargeUrl ?? latest.faceImageUrl) : null);
  // Пока этюдов на сайте нет, вход ведёт на полку карт: пустая карта этюдов
  // была бы тупиком.
  let gameHref = $derived(world && world.etudes.length === 0 ? '/battles' : '/battles/etude');
</script>

<section class="sections" aria-labelledby="sections-title">
  <header class="sections-head">
    <h2 id="sections-title" class="sections-title">{$t('worldTitle')}</h2>
    <p class="sections-lead">{$t('worldLead')}</p>
  </header>

  <div class="doors">
    <!-- Работы -->
    <a class="door" href="/figurines" onclick={() => interact('world_link', 'home_to_works')}>
      <span class="door-face door-face--photo">
        {#if latestFace}
          <AppImage src={latestFace} alt="" class="door-img" sizes="(max-width: 860px) 92vw, 360px" />
        {/if}
      </span>
      <span class="door-copy">
        <b class="door-name">{$t('navAllWorks')}</b>
        <span class="door-hint">{$t('worldWorksHint')}</span>
        {#if works > 0}
          <span class="door-meta">{$t('worldWorksCount').replace('{n}', String(works))}</span>
        {/if}
      </span>
    </a>

    <!-- Байки -->
    <a
      class="door"
      href={tale ? leafHref(tale, 'home_sections') : '/tales'}
      onclick={() => interact('world_link', 'home_to_tales')}
    >
      <span class="door-face door-face--photo">
        {#if taleCover}
          <AppImage src={taleCover} alt="" class="door-img" sizes="(max-width: 860px) 92vw, 360px" />
        {/if}
      </span>
      <span class="door-copy">
        <b class="door-name">{$t('navTales')}</b>
        <span class="door-hint">{$t('worldTalesHint')}</span>
        {#if taleCopy}
          <span class="door-quote">«{taleCopy.title}»{#if taleCopy.dek}{' — '}{taleCopy.dek.replace(/^[\s—–-]+/, '')}{/if}</span>
          <span class="door-go">{$t('worldTalesGo')} →</span>
        {/if}
      </span>
    </a>

    <!-- Игра: без картинки, только слова, строкой во всю ширину под двумя
         входами с фотографиями. Пустая плитка с рамкой в ряду с ними
         выглядела бы пропущенной фотографией. -->
    <a class="door door--game" href={gameHref} onclick={() => interact('world_link', 'home_to_game')}>
      <span class="door-copy">
        <b class="door-name">{$t('navGame')}</b>
        <span class="door-hint">{$t('worldGameHint')}</span>
        <span class="door-go">{gameHref === '/battles' ? $t('worldOpenShelf') : $t('worldDemoGo')} →</span>
      </span>
    </a>
  </div>
</section>

<style>
  .sections {
    max-width: 1180px;
    margin: clamp(56px, 8vw, 110px) auto 0;
    padding: 0 16px;
  }
  .sections-head {
    text-align: center;
    margin-bottom: clamp(24px, 4vw, 44px);
  }
  .sections-title {
    margin: 0 0 0.4rem;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(28px, 3.4vw, 42px);
    font-weight: 400;
    font-style: italic;
    color: var(--color-ink-primary, #34251c);
  }
  .sections-lead {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 18px;
    color: var(--color-ink-secondary, #5b4638);
  }

  .doors {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    max-width: 820px;
    margin: 0 auto;
    gap: clamp(18px, 2.4vw, 32px);
    align-items: start;
  }
  .door--game {
    grid-column: 1 / -1;
    padding: 20px 24px;
  }
  .door--game .door-copy {
    grid-template-columns: auto 1fr auto;
    align-items: baseline;
    gap: 6px 20px;
  }
  .door {
    display: grid;
    gap: 16px;
    padding: 16px;
    border: 1px solid var(--color-rule, #d8c6b1);
    outline: 1px solid var(--color-rule, #d8c6b1);
    outline-offset: -6px;
    background: color-mix(in srgb, var(--color-canvas, #f8f1e7) 70%, #fff);
    color: inherit;
    text-decoration: none;
    transition: transform 0.3s ease, box-shadow 0.3s ease;
  }
  a.door:hover {
    transform: translateY(-3px);
    box-shadow: 0 14px 30px -18px rgba(52, 37, 28, 0.45);
  }
  a.door:focus-visible {
    outline: 2px solid rgba(198, 95, 60, 0.56);
    outline-offset: 3px;
  }

  .door-face {
    display: block;
    position: relative;
    aspect-ratio: 4 / 5;
    overflow: hidden;
    background: var(--color-canvas-sunken, #efe4d3);
  }
  .door-face :global(.door-img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 0.6s ease;
  }
  a.door:hover .door-face :global(.door-img) {
    transform: scale(1.04);
  }

  .door-copy {
    display: grid;
    gap: 6px;
  }
  .door-name {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 26px;
    font-weight: 500;
    color: var(--color-ink-primary, #34251c);
  }
  .door-hint {
    font-size: 13px;
    line-height: 1.5;
    color: var(--color-ink-secondary, #5b4638);
  }
  .door-meta {
    font-size: 11px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary, #8a6a55);
  }
  .door-quote {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-style: italic;
    font-size: 17px;
    line-height: 1.4;
    color: var(--color-ink-primary, #34251c);
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .door-go {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 18px;
    color: var(--color-ember-deep, #6f3b24);
    text-decoration: none;
  }
  .door--game .door-go {
    border-bottom: 1px solid color-mix(in srgb, var(--color-ember, #c65f3c) 40%, transparent);
    justify-self: start;
  }

  @media (max-width: 860px) {
    .doors {
      grid-template-columns: 1fr;
      max-width: 460px;
      margin: 0 auto;
    }
    .door--game .door-copy {
      grid-template-columns: 1fr;
    }
    .door-face {
      aspect-ratio: 16 / 11;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .door,
    .door-face :global(.door-img) {
      transition: none;
    }
    a.door:hover,
    a.door:hover .door-face :global(.door-img) {
      transform: none;
    }
  }
</style>
