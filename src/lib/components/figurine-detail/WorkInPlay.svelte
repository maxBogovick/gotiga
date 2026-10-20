<script lang="ts">
  // «Эта работа есть в игре»: карта, сделанная из работы, и этюды, где она
  // стоит. Стоит на странице работы рядом с отрывком байки.
  //
  // На страницу работы приходят чаще, чем на любую другую, а игра стоит
  // отдельно и без ссылки на неё не находится. Работа и карта — одна
  // героиня, поэтому ссылка идёт от того, что человеку уже понравилось.
  //
  // Полка читается в браузере, а не загрузчиком страницы: страница работы
  // пререндерится, и карта, назначенная после сборки, иначе не появилась бы.
  import { onMount } from 'svelte';
  import { t, lang, localized } from '$lib/i18n';
  import { interact } from '$lib/analytics';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import CardPostcard from '$lib/components/CardPostcard.svelte';
  import { cardCopy } from '$lib/battles';
  import { loadWorld, cardOfWork, etudesWithCard, cardHref, etudeHref, type World } from '$lib/world';

  // `tale` — тот же блок под байкой: героиня байки и карта — одна работа.
  let { figurineId, from = 'work' }: { figurineId: string; from?: 'work' | 'tale' } = $props();
  let toCard = $derived(`${from}_to_card`);
  let toEtude = $derived(`${from}_to_etude`);

  let world = $state<World | null>(null);

  onMount(() => {
    loadWorld()
      .then((w) => (world = w))
      .catch(() => {});
  });

  let card = $derived(world ? cardOfWork(world, figurineId) : null);
  let etudes = $derived(world && card ? etudesWithCard(world, card) : []);
  let title = $derived(card ? cardCopy(card, $lang).title : '');
</script>

{#if world && card}
  <section class="wip" aria-labelledby="wip-title">
    <a class="wip-card" href={cardHref(card)} onclick={() => interact('world_link', toCard)} aria-label={title}>
      <BattleCard {card} frames={world.frames} owned={true} interactive={false} uniform={true} />
    </a>
    <div class="wip-copy">
      <p class="wip-label">{$t(from === 'tale' ? 'worldTaleInPlayLabel' : 'worldInPlayLabel')}</p>
      <h2 id="wip-title" class="wip-title">
        <a href={cardHref(card)} onclick={() => interact('world_link', toCard)}>{title}</a>
      </h2>
      <p class="wip-text">{$t('worldInPlayText')}</p>

      {#if etudes.length > 0}
        <p class="wip-etudes">
          {$t('worldInPlayEtudes')}
          {#each etudes.slice(0, 3) as etude, i (etude.id)}
            {#if i > 0}<span aria-hidden="true">, </span>{/if}
            <a href={etudeHref(etude)} onclick={() => interact('world_link', toEtude)}
              >«{localized(etude, 'title', $lang)}»</a
            >
          {/each}
        </p>
      {/if}

      <p class="wip-actions">
        <a class="wip-go" href={cardHref(card)} onclick={() => interact('world_link', toCard)}
          >{$t('worldOpenCard')} →</a
        >
        <!-- Без единого этюда на сайте играть негде: ссылки нет вовсе. -->
        {#if world.etudes.length}
          <a
            class="wip-go"
            href={etudes[0] ? etudeHref(etudes[0]) : '/battles/etude'}
            onclick={() => interact('world_link', toEtude)}>{$t('worldPlay')} →</a
          >
        {/if}
        <CardPostcard {card} frames={world.frames} />
      </p>
    </div>
  </section>
{/if}

<style>
  .wip {
    margin: 2.2rem 0 0;
    display: grid;
    grid-template-columns: 150px 1fr;
    gap: 1.4rem;
    align-items: start;
    max-width: 38em;
  }
  .wip-card {
    display: block;
    width: 150px;
    container-type: inline-size;
    transition: transform 0.25s ease;
  }
  .wip-card:hover {
    transform: rotate(-1.5deg) translateY(-2px);
  }
  .wip-label {
    margin: 0 0 0.5rem;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary, #8a6a55);
  }
  .wip-title {
    margin: 0 0 0.5rem;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(22px, 2.4vw, 28px);
    font-weight: 400;
    font-style: italic;
    line-height: 1.2;
  }
  .wip-title a {
    color: var(--color-ink-primary, #34251c);
    text-decoration: none;
  }
  .wip-title a:hover {
    color: var(--color-ember-deep, #6f3b24);
  }
  .wip-text,
  .wip-etudes {
    margin: 0 0 0.6rem;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 17px;
    line-height: 1.5;
    color: var(--color-ink-secondary, #5b4638);
  }
  .wip-etudes a {
    color: var(--color-ember-deep, #6f3b24);
    text-decoration: none;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ember, #c65f3c) 35%, transparent);
  }
  .wip-actions {
    margin: 0.4rem 0 0;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.5rem 1.2rem;
  }
  .wip-go {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 18px;
    color: var(--color-ember-deep, #6f3b24);
    text-decoration: none;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ember, #c65f3c) 40%, transparent);
  }
  .wip-go:hover {
    color: var(--color-ink-primary, #34251c);
  }

  @media (max-width: 520px) {
    .wip {
      grid-template-columns: 110px 1fr;
      gap: 1rem;
    }
    .wip-card {
      width: 110px;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .wip-card {
      transition: none;
    }
    .wip-card:hover {
      transform: none;
    }
  }
</style>
