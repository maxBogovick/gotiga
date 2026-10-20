<script lang="ts">
  /**
   * Плитка входа на полку небылиц, в ряду входов первого экрана.
   *
   * Листает небылицы полки: первой та, что комната показывает крупно, дальше
   * остальные. Полку читает сама, а не расширяет загрузку главной, и когда
   * полка пуста, не рисует ничего: пустой вход хуже отсутствующего.
   */
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t, lang } from '$lib/i18n';
  import { leafCopy, leafCoverUrl, leafHref } from '$lib/gazette';
  import { leadTale } from '$lib/tales';
  import type { GazetteLeaf } from '$lib/types/api';
  import HeroSlideTile, { type TileSlide } from '$lib/components/HeroSlideTile.svelte';

  const MAX_SLIDES = 6;

  let tales = $state<GazetteLeaf[]>([]);
  let slides = $derived.by<TileSlide[]>(() => {
    const lead = leadTale(tales);
    if (!lead) return [];
    const ordered = [lead, ...tales.filter((tale) => tale.id !== lead.id)];
    const out: TileSlide[] = [];
    for (const tale of ordered) {
      const title = leafCopy(tale, $lang).title;
      if (!title) continue;
      out.push({
        id: `tale-${tale.id}`,
        href: leafHref(tale, 'home_tales'),
        title,
        imageUrl: leafCoverUrl(tale) || null,
      });
      if (out.length >= MAX_SLIDES) break;
    }
    return out;
  });

  // Один вход среди нескольких: если полку прочитать нельзя, этого входа просто
  // нет. `getTales` бросает, чтобы пререндер не отдал пустую полку молча.
  onMount(async () => {
    try {
      tales = await api.getTales();
    } catch {
      tales = [];
    }
  });
</script>

<HeroSlideTile {slides} kicker={$t('navTales')} startDelayMs={3000} />
