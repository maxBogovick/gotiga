<script lang="ts">
  // «Похожие работы»: до трёх, в порядке автора. Их показывает док на главной
  // тремя миниатюрами рядом с текущей работой.
  //
  // Выбор идёт по списку реестра, а не по поиску: работ десятки, и название у
  // каждой короткое. Скрытая работа в списке есть, но помечена: в публичный ответ
  // сервер её не отдаёт, и автор должен знать, что миниатюры на сайте не будет.
  // Первое место — самая близкая работа: док показывает их в этом порядке.
  import { t } from '$lib/i18n';
  import AppImage from '$lib/components/AppImage.svelte';
  import type { FigurineListItem } from '$lib/types/api';

  let {
    selfId,
    figurines,
    value = $bindable([]),
  }: {
    /** Номер редактируемой работы: она сама в список не попадает. */
    selfId: string;
    /** Весь реестр. */
    figurines: FigurineListItem[];
    /** Номера выбранных работ по порядку; меньше трёх — остальные места пусты. */
    value: string[] | undefined;
  } = $props();

  const SLOTS = [0, 1, 2];

  let chosen = $derived(value ?? []);
  let byId = $derived(new Map(figurines.map((f) => [f.id, f])));

  // Работа, уже выбранная на другом месте, повторно не предлагается.
  function options(slot: number) {
    const taken = new Set(chosen.filter((_, i) => i !== slot));
    return figurines.filter((f) => f.id !== selfId && !taken.has(f.id));
  }

  function pick(slot: number, id: string) {
    const next = [...chosen];
    // Пустое место посередине сдвигало бы порядок: пустое просто убирается.
    if (id) next[slot] = id;
    else next.splice(slot, 1);
    value = next.filter(Boolean).slice(0, 3);
  }
</script>

<div class="similar">
  <span class="label">{$t('adminFieldSimilar')}</span>
  <div class="slots">
    {#each SLOTS as slot (slot)}
      {@const id = chosen[slot] ?? ''}
      {@const work = id ? byId.get(id) : undefined}
      <div class="slot">
        <span class="thumb">
          {#if work}
            <AppImage src={work.faceImageUrl} thumbUrl={work.thumbUrl} alt="" class="thumb-img" />
          {:else}
            <span class="num">{slot + 1}</span>
          {/if}
        </span>
        <select
          class="input-gothic"
          value={id}
          aria-label={`${$t('adminFieldSimilar')} ${slot + 1}`}
          onchange={(e) => pick(slot, e.currentTarget.value)}
          disabled={slot > chosen.length}
        >
          <option value="">{$t('adminSimilarNone')}</option>
          {#each options(slot) as f (f.id)}
            <option value={f.id}>{f.name}{f.isVisible === false ? ` · ${$t('adminSimilarHidden')}` : ''}</option>
          {/each}
        </select>
      </div>
    {/each}
  </div>
  <span class="hint">{$t('adminFieldSimilarHint')}</span>
</div>

<style>
  .label {
    display: block;
    margin-bottom: 0.35rem;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: #5f4636;
  }

  .slots {
    display: grid;
    gap: 8px;
  }

  .slot {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .thumb {
    flex: none;
    display: grid;
    place-items: center;
    width: 42px;
    height: 42px;
    overflow: hidden;
    border: 1px solid rgba(198, 95, 60, 0.2);
    background: #efe4d3;
  }

  .thumb :global(.thumb-img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .num {
    font-size: 12px;
    color: #7c6554;
  }

  .input-gothic {
    flex: 1;
    min-width: 0;
    background-color: #f8f1e7;
    border: 1px solid rgba(198, 95, 60, 0.2);
    padding: 0.55rem 0.75rem;
    font-size: 0.875rem;
    color: #34251c;
    outline: none;
    font-family: inherit;
  }

  .input-gothic:focus {
    border-color: rgba(198, 95, 60, 0.55);
  }

  .input-gothic:disabled {
    opacity: 0.45;
  }

  .hint {
    display: block;
    margin-top: 0.4rem;
    font-size: 10px;
    color: rgba(95, 70, 54, 0.6);
  }
</style>
