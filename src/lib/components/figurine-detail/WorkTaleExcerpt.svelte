<script lang="ts">
  // Отрывок байки о работе: стоит под «Историей» и ведёт на сам лист.
  //
  // Читать байку целиком здесь не дают намеренно: у неё есть послесловие,
  // голосование и сцена на полях, а тот же текст на двух адресах поисковик
  // счёл бы повтором. Поэтому показан только зачин, затухающий в бумагу.
  //
  // Затухание — маска на самом тексте, а не градиент цвета фона поверх него:
  // страница бывает на разной бумаге (тема, раскладки), и градиент другого
  // оттенка оставил бы на ней пятно.
  import { t, lang } from '$lib/i18n';
  import type { GazetteLeaf } from '$lib/types/api';
  import { leafCopy, leafHref } from '$lib/gazette';
  import { minutesKey, readingMinutes, renderTale } from '$lib/tales';

  let { tales }: { tales: GazetteLeaf[] } = $props();

  // Сколько знаков зачина показываем. Больше — это уже чтение на странице
  // работы, меньше — затухание съест всю первую строку.
  const SHOWN = 420;
  const HARD_CAP = 560;

  let lead = $derived(tales[0] ?? null);
  let others = $derived(tales.slice(1));
  let copy = $derived(lead ? leafCopy(lead, $lang) : null);

  /**
   * Зачин: абзацы до порога, текстом. Ссылка на дом внутри фразы печатается
   * своим адресом (`bare`) — названия ссылок спрашивает загрузчик страницы
   * байки, а не этой, и из-за зачина вторую загрузку заводить не стоит.
   */
  let opening = $derived.by(() => {
    if (!copy?.body) return { paragraphs: [] as string[], cut: false };
    const paras = renderTale(copy.body)
      .filter((b) => b.kind === 'p')
      .map((b) =>
        b.kind === 'p'
          ? b.runs.map((r) => (r.kind === 'link' ? r.ref.bare : r.text)).join('')
          : '',
      );
    const shown: string[] = [];
    let size = 0;
    for (const p of paras) {
      if (size >= SHOWN) break;
      shown.push(p);
      size += p.length;
    }
    let cut = shown.length < paras.length;
    // Один длинный абзац режем по концу фразы, а не посреди слова.
    const last = shown[shown.length - 1];
    if (last && size > HARD_CAP) {
      const room = last.length - (size - HARD_CAP);
      const head = last.slice(0, Math.max(room, 0));
      const stop = Math.max(head.lastIndexOf('. '), head.lastIndexOf('! '), head.lastIndexOf('? '));
      shown[shown.length - 1] = (stop > 80 ? head.slice(0, stop + 1) : head.slice(0, head.lastIndexOf(' '))).trim();
      cut = true;
    }
    return { paragraphs: shown, cut };
  });

  let minutes = $derived(copy?.body ? readingMinutes(copy.body) : 0);
  let author = $derived(lead?.author?.trim() ?? '');
</script>

{#if lead && copy && (opening.paragraphs.length > 0 || copy.dek)}
  <section class="wte" aria-labelledby="wte-title">
    <p class="wte-label">{$t('talesAboutThisThing')}</p>
    <h2 id="wte-title" class="wte-title">
      <a href={leafHref(lead, 'work')}>{copy.title}</a>
    </h2>
    {#if copy.dek && opening.paragraphs.length === 0}
      <p class="wte-dek">{copy.dek}</p>
    {/if}

    {#if opening.paragraphs.length > 0}
      <div class="wte-text" class:wte-text--cut={opening.cut}>
        {#each opening.paragraphs as para, i}
          <p class="wte-para" class:wte-para--first={i === 0}>{para}</p>
        {/each}
      </div>
    {/if}

    <p class="wte-foot">
      <a class="wte-more" href={leafHref(lead, 'work')}>
        {opening.cut ? $t('talesReadOn') : $t('talesRead')} →
      </a>
      {#if minutes > 0}
        <span class="wte-meta">{minutes} {$t(minutesKey(minutes))}</span>
      {/if}
      {#if author}
        <span class="wte-meta wte-author">{author}</span>
      {/if}
    </p>

    {#if others.length > 0}
      <ul class="wte-others">
        {#each others as tale (tale.id)}
          <li><a href={leafHref(tale, 'work')}>{leafCopy(tale, $lang).title}</a></li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}

<style>
  .wte {
    margin: 2.2rem 0 0;
    padding-left: 1.1rem;
    border-left: 1px solid color-mix(in srgb, var(--color-ember, #c65f3c) 45%, transparent);
    max-width: 38em;
  }

  .wte-label {
    margin: 0 0 0.5rem;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary, #8a6a55);
  }

  .wte-title {
    margin: 0 0 0.6rem;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(22px, 2.4vw, 28px);
    font-weight: 400;
    font-style: italic;
    line-height: 1.2;
    text-wrap: balance;
  }
  .wte-title a {
    color: var(--color-ink-primary, #34251c);
    text-decoration: none;
  }
  .wte-title a:hover {
    color: var(--color-ember-deep, #6f3b24);
  }

  .wte-dek {
    margin: 0 0 0.6rem;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 17px;
    font-style: italic;
    line-height: 1.5;
    color: var(--color-ink-secondary, #5b4638);
  }

  .wte-para {
    margin: 0 0 0.9em;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(17px, 1.7vw, 19px);
    line-height: 1.68;
    color: color-mix(in srgb, var(--color-ink-primary, #34251c) 90%, transparent);
  }
  .wte-para--first::first-letter {
    float: left;
    font-size: 3.2em;
    line-height: 0.82;
    padding: 0.06em 0.09em 0 0;
    color: var(--color-ember-deep, #6f3b24);
  }

  /* Затухание последних строк: маска на тексте, фон страницы не нужен. */
  .wte-text--cut {
    -webkit-mask-image: linear-gradient(to bottom, #000 55%, transparent 100%);
    mask-image: linear-gradient(to bottom, #000 55%, transparent 100%);
    padding-bottom: 0.4rem;
  }
  .wte-text--cut .wte-para:last-child {
    margin-bottom: 0;
  }

  .wte-foot {
    margin: 0.35rem 0 0;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.4rem 1rem;
  }
  .wte-more {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 18px;
    color: var(--color-ember-deep, #6f3b24);
    text-decoration: none;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ember, #c65f3c) 40%, transparent);
  }
  .wte-more:hover {
    color: var(--color-ink-primary, #34251c);
    border-bottom-color: var(--color-ember, #c65f3c);
  }
  .wte-meta {
    font-size: 11px;
    letter-spacing: 0.08em;
    color: var(--color-ink-tertiary, #8a6a55);
  }
  .wte-author {
    font-style: italic;
    letter-spacing: 0.02em;
  }

  .wte-others {
    list-style: none;
    margin: 0.9rem 0 0;
    padding: 0;
    display: grid;
    gap: 4px;
  }
  .wte-others a {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 17px;
    color: var(--color-ember-deep, #6f3b24);
    text-decoration: none;
    border-bottom: 1px solid transparent;
  }
  .wte-others a:hover {
    border-bottom-color: var(--color-ember, #c65f3c);
  }

  @media (max-width: 640px) {
    .wte {
      margin-top: 1.8rem;
    }
  }
</style>
