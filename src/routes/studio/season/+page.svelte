<script lang="ts">
  // Сезон: неделя, работы людей и оценки.
  //
  // Смотреть можно всякому — это та самая страница, ради которой приходят из
  // соцсетей, — а оценивать только со входом. Каждая работа показана НАСТОЯЩЕЙ
  // картой в её раме: показывать её иначе значит завести второй облик, который
  // однажды соврёт.
  //
  // Живой счёт виден в ходе недели: ради него и заходят каждый день.
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { emptyBattleCard } from '$lib/battles';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import BattleIcon from '$lib/components/BattleIcon.svelte';
  import StudioAsk from '$lib/components/studio/StudioAsk.svelte';
  import '$lib/components/studio/studio-room.css';
  import type { BattleCard as BattleCardDto, StudioSeasonPage } from '$lib/types/api';

  let page = $state<StudioSeasonPage | null>(null);
  let loading = $state(true);
  let said = $state<string | null>(null);
  let mineFrames = $state<{ id: string; name: string }[]>([]);
  /** Свои работы, которые ещё ждут слова хозяина. Показаны отдельно и без
   *  кнопки: «нажми и получи отказ» — не выбор. */
  let waiting = $state<string[]>([]);
  /** На какую работу пишут жалобу. Своим окном, а не системным: серое окно
   *  посреди пергамента — самое громкое, что может быть в комнате. */
  let complaining = $state<string | null>(null);

  let token = $derived(authStore.token);

  /** Манекен — карта дома. Одна на всю страницу: работы отличаются рамой, и
   *  показывать их на разных картах значило бы сравнивать не то. */
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

  onMount(reload);

  async function reload() {
    loading = true;
    page = await api.getStudioSeason(token).catch(() => null);
    if (token) {
      const studio = await api.getStudio(token).catch(() => null);
      const mine = studio?.frames ?? [];
      // Выставить можно только ДОПУЩЕННУЮ: до слова хозяина работу не видит
      // никто, и в сезон ей нельзя.
      mineFrames = mine
        .filter((f) => f.status === 'shown' && f.admittedAt)
        .map((f) => ({ id: f.id, name: f.name }));
      waiting = mine.filter((f) => f.status === 'shown' && !f.admittedAt).map((f) => f.name);
    }
    loading = false;
  }

  function flash(text: string, ms = 5000) {
    said = text;
    setTimeout(() => (said = null), ms);
  }

  async function rate(entry: string, value: number) {
    if (!token) return;
    try {
      await api.rateStudioEntry(token, entry, value);
      page = await api.getStudioSeason(token);
    } catch (e) {
      flash(String(e));
    }
  }

  async function enter(frameId: string) {
    if (!token) return;
    try {
      await api.enterStudioSeason(token, frameId);
      await reload();
      flash($t('studioEnteredDone'));
    } catch (e) {
      flash(String(e));
    }
  }

  async function report(reason: string) {
    const frameId = complaining;
    complaining = null;
    if (!frameId) return;
    await api.reportStudioFrame(token, frameId, reason).catch((e) => flash(String(e)));
    flash($t('studioReportDone'));
  }

  /** Сколько дней осталось. Днями, а не часами с секундами: неделя — это про
   *  дни, а бегущие цифры превращают её в таймер, которого дом не держит. */
  let daysLeft = $derived.by(() => {
    if (!page) return 0;
    const left = new Date(page.season.closesAt).getTime() - Date.now();
    return Math.max(0, Math.ceil(left / 86400000));
  });
</script>

<svelte:head>
  <title>{$t('studioSeason')}</title>
  <meta name="description" content={$t('studioSeasonAbout')} />
</svelte:head>

<div class="studio-room">
  <div class="page">
    <p class="eyebrow">
      <a href="/studio">{$t('studioBack')}</a>
      <span class="eyebrow-rule"></span>
      <span>{$t('studioEyebrow')}</span>
    </p>

    {#if loading}
      <h1 class="room-title">{$t('studioSeason')}</h1>
      <p class="empty">{$t('studioLoading')}</p>
    {:else if !page}
      <h1 class="room-title">{$t('studioSeason')}</h1>
      <p class="empty">{$t('studioNoSeason')}</p>
    {:else}
      <h1 class="room-title">
        {$t('studioSeason')} <span style="opacity:.45">№{page.season.number}</span>
      </h1>

      <!-- Тема недели крупно: это приглашение, а не проверка. Работа мимо темы
           участвует наравне. -->
      {#if page.season.theme}
        <p class="room-lead">{page.season.theme}</p>
        {#if page.season.themeNote}<p class="hint">{page.season.themeNote}</p>{/if}
      {:else}
        <p class="room-lead">{$t('studioSeasonFree')}</p>
      {/if}

      <p class="mark">
        {#if page.season.state === 'open'}
          {$t('studioDaysLeft').replace('{n}', String(daysLeft))}
        {:else}
          {$t('studioSeasonJudged')}
        {/if}
        {#if token && page.mayRate}
          · {$t('studioRatingsLeft').replace('{n}', String(page.ratingsLeft))}
        {:else if page.entered}
          · {$t('studioAuthorMayNotRate')}
        {/if}
      </p>

      {#if said}<p class="said">{said}</p>{/if}

      <!-- Заявиться. Только допущенной работой. -->
      {#if token && !page.entered && page.season.state === 'open'}
        {#if mineFrames.length}
          <div class="slip">
            <p class="hint" style="font-style:normal">{$t('studioEnterAsk')}</p>
            <div class="doors" style="margin-top:.6rem">
              {#each mineFrames as one (one.id)}
                <button class="btn" onclick={() => enter(one.id)}>{one.name}</button>
              {/each}
            </div>
          </div>
        {:else if waiting.length}
          <p class="hint">{$t('studioEnterWaiting').replace('{names}', waiting.join(', '))}</p>
        {:else}
          <p class="hint">{$t('studioEnterNothing')}</p>
        {/if}
      {/if}

      {#if page.entries.length === 0}
        <p class="empty">{$t('studioSeasonEmpty')}</p>
      {:else}
        <div class="shelf">
          {#each page.entries as entry (entry.id)}
            <div>
              <BattleCard card={sample} frames={[entry.body]} owned={true} />
              <p class="name">{entry.name}</p>
              <p class="by">
                {entry.author}
                {#if entry.place}· {$t('studioPlace').replace('{n}', String(entry.place))}{/if}
              </p>

              <!-- Оценка: пять знаков дома, своя видна, чужие — только числом.
                   Знак свой, а не юникодная звезда: системный глиф рядом с
                   гравюрной рамой выглядит чужим ровно так же, как системное
                   окно. -->
              <div class="deeds">
                {#if token && page.mayRate && entry.authorId !== authStore.user?.id && page.season.state === 'open'}
                  <span class="stars">
                    {#each [1, 2, 3, 4, 5] as star (star)}
                      <button
                        class="star"
                        class:star--lit={(entry.mine ?? 0) >= star}
                        aria-label={String(star)}
                        onclick={() => rate(entry.id, star)}
                      >
                        <BattleIcon name="bless" size={14} />
                      </button>
                    {/each}
                  </span>
                {/if}
                <span class="mark" style="margin:0">
                  {#if entry.votes > 0}
                    {$t('studioVotes').replace('{n}', String(entry.votes))}
                    {#if entry.score}· {entry.score.toFixed(2)}{/if}
                  {:else}
                    {$t('studioNoVotesYet')}
                  {/if}
                </span>
                <button class="quiet quiet--danger" onclick={() => (complaining = entry.frameId)}
                  >{$t('studioReport')}</button
                >
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</div>

{#if complaining}
  <StudioAsk
    title={$t('studioReport')}
    lead={$t('studioReportAsk')}
    field="text"
    yes={$t('studioReport')}
    danger
    onyes={report}
    onclose={() => (complaining = null)}
  />
{/if}

<style>
  .stars {
    display: inline-flex;
    gap: 0.15rem;
  }

  .star {
    padding: 0.1rem;
    border: 0;
    background: none;
    color: #d8c6b1;
    cursor: pointer;
    line-height: 0;
  }

  .star:hover {
    color: #8a6a55;
  }

  .star--lit {
    color: #c65f3c;
  }
</style>
