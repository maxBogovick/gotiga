<script lang="ts">
  // Зал авторов: страница человека и всё, что от него осталось на людях.
  //
  // Это и есть «увековечим» из уговора: не строчка в списке, а своя страница со
  // своим адресом, которую можно дать ссылкой и которую найдут поиском.
  //
  // Видна без имени — иначе ссылкой её не поделишься.
  import { onMount } from 'svelte';
  import { page as routePage } from '$app/stores';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import { cardFromRequest, emptyBattleCard } from '$lib/battles';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import type { BattleCard as BattleCardDto, StudioAuthor } from '$lib/types/api';

  let author = $state<StudioAuthor | null>(null);
  let loading = $state(true);
  let slug = $derived($routePage.params.slug ?? '');

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

  onMount(async () => {
    author = await api.getStudioAuthor(slug).catch(() => null);
    loading = false;
  });
</script>

<svelte:head>
  <title>{author?.name ?? $t('studioAuthors')}</title>
  {#if author}
    <meta
      name="description"
      content={$t('studioAuthorLead').replace('{name}', author.name).replace('{n}', String(author.approved))}
    />
  {/if}
</svelte:head>

<div class="mx-auto max-w-5xl px-5 py-10">
  <div class="flex flex-wrap items-baseline justify-between gap-3">
    <h1 class="font-serif text-2xl text-[#34251c]">{author?.name ?? $t('studioAuthors')}</h1>
    <a
      href="/studio/gallery"
      class="text-xs uppercase tracking-[0.16em] text-[#8a6a55] hover:text-[#c65f3c]"
      >← {$t('studioGallery')}</a
    >
  </div>

  {#if loading}
    <p class="mt-8 text-sm text-[#8a6a55]">{$t('studioLoading')}</p>
  {:else if !author}
    <p class="mt-8 text-sm text-[#8a6a55]">{$t('studioAuthorGone')}</p>
  {:else}
    <p class="mt-2 text-sm text-[#6f3b24]">
      {$t('studioAuthorLead')
        .replace('{name}', author.name)
        .replace('{n}', String(author.approved))}
    </p>

    <div class="mt-8 grid grid-cols-2 gap-6 sm:grid-cols-3 lg:grid-cols-4">
      {#each author.works as work (work.id)}
        <div>
          <BattleCard card={sample} frames={[work.body]} owned={true} />
          <p class="mt-2 truncate text-sm text-[#34251c]">{work.name}</p>
          {#if work.approvedAt}
            <p class="text-[10px] uppercase tracking-[0.14em] text-[#c65f3c]">
              {$t('studioApprovedMark')}
              {#if work.editionSize}· {$t('studioEditionOf').replace('{n}', String(work.editionSize))}{/if}
            </p>
          {:else}
            <p class="text-[10px] uppercase tracking-[0.14em] text-[#b0a08e]">
              {$t('studioShownMark')}
            </p>
          {/if}
        </div>
      {/each}
    </div>

    <!-- Карты автора. Третье обещанное место автографа: лист взятия · ЗАЛ
         АВТОРОВ · лавка. Показаны настоящими картами, теми же, что стоят на
         полке дома. -->
    {#if author.cards.length}
      <h2 class="mt-12 font-serif text-xl text-[#34251c]">{$t('studioAuthorCards')}</h2>
      <div class="mt-4 grid grid-cols-2 gap-6 sm:grid-cols-3 lg:grid-cols-4">
        {#each author.cards as made (made.id)}
          <div>
            <BattleCard card={cardFromRequest(made.body)} frames={null} owned={true} />
            {#if made.cardId}
              <a
                href="/battles?card={made.cardId}"
                class="mt-2 block text-[10px] uppercase tracking-[0.14em] text-[#c65f3c] hover:underline"
                >{$t('studioCardSeeOnShelf')}</a
              >
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>
