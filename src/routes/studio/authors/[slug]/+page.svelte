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
  import '$lib/components/studio/studio-room.css';
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

  let lead = $derived(
    author
      ? $t('studioAuthorLead')
          .replace('{name}', author.name)
          .replace('{n}', String(author.approved))
      : '',
  );
</script>

<svelte:head>
  <title>{author?.name ?? $t('studioAuthors')}</title>
  {#if author}<meta name="description" content={lead} />{/if}
</svelte:head>

<div class="studio-room">
  <div class="page">
    <p class="eyebrow">
      <a href="/studio/gallery">{$t('studioGallery')}</a>
      <span class="eyebrow-rule"></span>
      <span>{$t('studioAuthors')}</span>
    </p>

    {#if loading}
      <p class="empty">{$t('studioLoading')}</p>
    {:else if !author}
      <h1 class="room-title">{$t('studioAuthors')}</h1>
      <p class="empty">{$t('studioAuthorGone')}</p>
    {:else}
      <h1 class="room-title">{author.name}</h1>
      <p class="room-lead">{lead}</p>

      <div class="shelf">
        {#each author.works as work (work.id)}
          <div>
            <BattleCard card={sample} frames={[work.body]} owned={true} />
            <p class="name">{work.name}</p>
            {#if work.approvedAt}
              <p class="mark mark--lit">
                {$t('studioApprovedMark')}
                {#if work.editionSize}· {$t('studioEditionOf').replace('{n}', String(work.editionSize))}{/if}
              </p>
            {:else}
              <p class="mark">{$t('studioShownMark')}</p>
            {/if}
          </div>
        {/each}
      </div>

      <!-- Карты автора. Третье обещанное место автографа: лист взятия · ЗАЛ
           АВТОРОВ · лавка. -->
      {#if author.cards.length}
        <h2 class="shelf-title">{$t('studioAuthorCards')}</h2>
        <div class="shelf">
          {#each author.cards as made (made.id)}
            <div>
              <BattleCard card={cardFromRequest(made.body)} frames={null} owned={true} />
              {#if made.cardId}
                <p class="deeds">
                  <a class="quiet" href="/battles?card={made.cardId}">{$t('studioCardSeeOnShelf')}</a>
                </p>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</div>
