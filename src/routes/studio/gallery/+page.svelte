<script lang="ts">
  // Галерея: всё, что люди сделали и что дом пустил на люди.
  //
  // Видна БЕЗ ИМЕНИ, и это не мелочь: сюда приходят по ссылке из соцсетей и
  // отсюда индексируют. Страница, которая просит пароль, не приводит никого.
  //
  // Работы стоят настоящими картами в своих рамах — тем же отрисовщиком, каким
  // они встанут в игру.
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import { emptyBattleCard } from '$lib/battles';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import type { BattleCard as BattleCardDto, StudioGallery } from '$lib/types/api';

  let gallery = $state<StudioGallery | null>(null);
  let page = $state(0);
  let loading = $state(true);

  const PER_PAGE = 24;

  /** Манекен один на всю полку: работы отличаются рамой, и разные карты
   *  сравнивали бы не то. */
  const sample: BattleCardDto = {
    ...emptyBattleCard(),
    tier: 1,
    titleEn: 'The Keeper of the Key',
    titleRu: 'Хранительница Ключа',
    effectRu: 'Вихрь Души: каждое третье заклинание создаёт копию эффекта.',
    effectEn: 'Wind of Soul: every third spell makes a copy of its effect.',
    cost: 5,
    power: 10,
  };

  onMount(() => load(0));

  async function load(next: number) {
    loading = true;
    page = next;
    gallery = await api.getStudioGallery(next).catch(() => null);
    loading = false;
  }

  let pages = $derived(gallery ? Math.ceil(gallery.total / PER_PAGE) : 0);
</script>

<svelte:head>
  <title>{$t('studioGallery')}</title>
  <meta name="description" content={$t('studioGalleryLead')} />
</svelte:head>

<div class="mx-auto max-w-5xl px-5 py-10">
  <div class="flex flex-wrap items-baseline justify-between gap-3">
    <h1 class="font-serif text-2xl text-[#34251c]">{$t('studioGallery')}</h1>
    <a href="/studio" class="text-xs uppercase tracking-[0.16em] text-[#8a6a55] hover:text-[#c65f3c]"
      >← {$t('studioBack')}</a
    >
  </div>
  <p class="mt-2 max-w-2xl text-sm leading-relaxed text-[#6f3b24]">{$t('studioGalleryLead')}</p>

  {#if loading}
    <p class="mt-8 text-sm text-[#8a6a55]">{$t('studioLoading')}</p>
  {:else if !gallery || gallery.works.length === 0}
    <p class="mt-8 text-sm text-[#8a6a55]">{$t('studioGalleryEmpty')}</p>
  {:else}
    <div class="mt-8 grid grid-cols-2 gap-6 sm:grid-cols-3 lg:grid-cols-4">
      {#each gallery.works as work (work.id)}
        <div>
          <BattleCard card={sample} frames={[work.body]} owned={true} />
          <p class="mt-2 truncate text-sm text-[#34251c]">{work.name}</p>
          <p class="text-[11px] text-[#8a6a55]">
            {#if work.authorSlug}
              <a href="/studio/authors/{work.authorSlug}" class="hover:underline">{work.author}</a>
            {:else}
              {work.author}
            {/if}
          </p>
          <!-- Утверждённое названо словом: между «пустили на люди» и «взято в
               игру» разница, ради которой всё и делается. -->
          {#if work.approvedAt}
            <p class="text-[10px] uppercase tracking-[0.14em] text-[#c65f3c]">
              {$t('studioApprovedMark')}
              {#if work.editionSize}· {$t('studioEditionOf').replace('{n}', String(work.editionSize))}{/if}
            </p>
          {/if}
        </div>
      {/each}
    </div>

    {#if pages > 1}
      <div class="mt-8 flex flex-wrap gap-2">
        {#each Array(pages) as _, i (i)}
          <button
            onclick={() => load(i)}
            class="border px-3 py-1.5 text-xs {i === page
              ? 'border-[#c65f3c] text-[#c65f3c]'
              : 'border-[#34251c]/20 hover:bg-[#34251c]/5'}">{i + 1}</button
          >
        {/each}
      </div>
    {/if}
  {/if}
</div>
