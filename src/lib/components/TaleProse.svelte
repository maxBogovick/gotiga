<script lang="ts">
  // Текст байки, напечатанный так, как его читают.
  //
  // Один отрисовщик на страницу байки и на предпросмотр стола рассказов:
  // предпросмотр, печатающий вторым кодом, однажды соврёт — и соврёт ровно
  // о том, ради чего его открывают (заголовок, жирный, карточка ссылки).
  //
  // Обёртки нет: блоки ложатся прямо в сетку того, кто их позвал (`.leaf`
  // страницы раскладывает их по колонкам), поэтому место в сетке назначает
  // вызывающий, а здесь — только набор.
  import type { Snippet } from 'svelte';
  import { ORNAMENT, type TaleBlock, type TaleRun } from '$lib/tales';
  import { siteRefKey, type SiteRefInfo } from '$lib/siteLinks';
  import TaleLink from './TaleLink.svelte';

  let {
    blocks,
    links = {},
    cardMorphs = [],
    after,
  }: {
    blocks: TaleBlock[];
    /** Названия ссылок на дом; ненайденная печатается адресом. */
    links?: Record<string, SiteRefInfo>;
    /** Имя перехода у карточки по номеру блока; пусто — без перехода. */
    cardMorphs?: string[];
    /** Что вставить после блока с этим номером (плита работы на странице). */
    after?: Snippet<[number]>;
  } = $props();

  // The drop cap belongs to the first paragraph, which need not be the first
  // block — a tale may open on an ornament.
  let firstPara = $derived(blocks.findIndex((b) => b.kind === 'p'));
</script>

<!-- Runs butt against each other with no whitespace between the tags: a
     newline here would print as a space before the comma that follows a link. -->
{#snippet bit(run: TaleRun)}{#if run.kind === 'link'}<TaleLink ref={run.ref} info={links[siteRefKey(run.ref)]} />{:else}{run.text}{/if}{/snippet}
{#snippet prose(runs: TaleRun[])}{#each runs as run}{#if run.bold}<strong>{@render bit(run)}</strong>{:else}{@render bit(run)}{/if}{/each}{/snippet}

{#each blocks as block, i}
  {#if block.kind === 'ornament'}
    <p class="ornament" aria-hidden="true">{ORNAMENT}</p>
  {:else if block.kind === 'card'}
    <div class="tipped">
      <TaleLink card ref={block.ref} info={links[siteRefKey(block.ref)]} morph={cardMorphs[i] || undefined} />
    </div>
  {:else if block.kind === 'heading'}
    <h2 class="subhead">{@render prose(block.runs)}</h2>
  {:else}
    <p class="para" class:opening={i === firstPara}>{@render prose(block.runs)}</p>
  {/if}
  {@render after?.(i)}
{/each}

<style>
  .para {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(18px, 1.9vw, 21px);
    font-weight: 400;
    line-height: 1.72;
    color: var(--ink, #34251c);
    max-width: 62ch;
    margin: 0 0 1.15em;
  }

  .para strong,
  .subhead strong { font-weight: 600; }

  .subhead {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(26px, 3vw, 36px);
    font-weight: 400;
    line-height: 1.15;
    text-wrap: balance;
    color: var(--ink, #34251c);
    max-width: 62ch;
    margin: 1.1em 0 0.55em;
  }
  /* A heading that opens the tale sits flush under the masthead. */
  .subhead:first-child { margin-top: 0; }

  .para.opening::first-letter {
    float: left;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 3.5em;
    line-height: 0.82;
    padding: 0.06em 0.09em 0 0;
    color: var(--deep, #6f3b24);
  }

  .ornament {
    max-width: 62ch;
    margin: 0.5em 0 1.4em;
    text-align: center;
    font-size: 13px;
    letter-spacing: 0.5em;
    color: var(--copper, #c65f3c);
    opacity: 0.55;
  }
</style>
