<script lang="ts">
  // Галерея: всё, что люди сделали и что дом пустил на люди.
  //
  // Видна БЕЗ ИМЕНИ, и это не мелочь: сюда приходят по ссылке из соцсетей и
  // отсюда индексируют. Страница, которая просит пароль, не приводит никого.
  //
  // Работы стоят настоящими картами в своих рамах — тем же отрисовщиком, каким
  // они встанут в игру. Одеты комнаты дома одной одеждой (`studio-room.css`):
  // студия — комната того же дома, а не соседний сайт.
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import { emptyBattleCard } from '$lib/battles';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import '$lib/components/studio/studio-room.css';
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

<div class="studio-room">
  <div class="page">
    <p class="eyebrow">
      <a href="/studio">{$t('studioBack')}</a>
      <span class="eyebrow-rule"></span>
      <span>{$t('studioEyebrow')}</span>
    </p>
    <h1 class="room-title">{$t('studioGallery')}</h1>
    <p class="room-lead">{$t('studioGalleryLead')}</p>

    {#if loading}
      <p class="empty">{$t('studioLoading')}</p>
    {:else if !gallery || gallery.works.length === 0}
      <p class="empty">{$t('studioGalleryEmpty')}</p>
    {:else}
      <div class="shelf">
        {#each gallery.works as work (work.id)}
          <div>
            <BattleCard card={sample} frames={[work.body]} owned={true} />
            <p class="name">{work.name}</p>
            <p class="by">
              {#if work.authorSlug}
                <a href="/studio/authors/{work.authorSlug}">{work.author}</a>
              {:else}
                {work.author}
              {/if}
            </p>
            <!-- Утверждённое названо словом: между «пустили на люди» и «взято в
                 игру» разница, ради которой всё и делается. -->
            {#if work.approvedAt}
              <p class="mark mark--lit">
                {$t('studioApprovedMark')}
                {#if work.editionSize}· {$t('studioEditionOf').replace('{n}', String(work.editionSize))}{/if}
              </p>
            {/if}
          </div>
        {/each}
      </div>

      {#if pages > 1}
        <div class="doors">
          {#each Array(pages) as _, i (i)}
            <button class="btn" class:btn--lit={i === page} onclick={() => load(i)}>{i + 1}</button>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</div>
