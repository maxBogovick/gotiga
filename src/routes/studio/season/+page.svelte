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
  import type { BattleCard as BattleCardDto, StudioSeasonPage } from '$lib/types/api';

  let page = $state<StudioSeasonPage | null>(null);
  let loading = $state(true);
  let said = $state<string | null>(null);
  let mineFrames = $state<{ id: string; name: string }[]>([]);
  /** Свои работы, которые ещё ждут слова хозяина. Показаны отдельно и без
   *  кнопки: «нажми и получи отказ» — не выбор. */
  let waiting = $state<string[]>([]);

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
      // Заявиться можно только допущенной работой: до слова хозяина её не
      // видит никто, и в сезон ей нельзя.
      const mine = studio?.frames ?? [];
      // Выставить можно только ДОПУЩЕННУЮ: до слова хозяина работу не видит
      // никто, и в сезон ей нельзя.
      mineFrames = mine
        .filter((f) => f.status === 'shown' && f.admittedAt)
        .map((f) => ({ id: f.id, name: f.name }));
      waiting = mine
        .filter((f) => f.status === 'shown' && !f.admittedAt)
        .map((f) => f.name);
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

  async function report(frameId: string) {
    const reason = prompt($t('studioReportAsk'));
    if (!reason) return;
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

<svelte:head><title>{$t('studioSeason')}</title></svelte:head>

<div class="mx-auto max-w-5xl px-5 py-10">
  <div class="flex flex-wrap items-baseline justify-between gap-3">
    <h1 class="font-serif text-2xl text-[#34251c]">
      {$t('studioSeason')}
      {#if page}<span class="text-[#b0a08e]">№{page.season.number}</span>{/if}
    </h1>
    <a href="/studio" class="text-xs uppercase tracking-[0.16em] text-[#8a6a55] hover:text-[#c65f3c]"
      >← {$t('studioBack')}</a
    >
  </div>

  {#if loading}
    <p class="mt-8 text-sm text-[#8a6a55]">{$t('studioLoading')}</p>
  {:else if !page}
    <p class="mt-8 text-sm text-[#8a6a55]">{$t('studioNoSeason')}</p>
  {:else}
    <!-- Тема недели крупно: это приглашение, а не проверка. Работа мимо темы
         участвует наравне. -->
    {#if page.season.theme}
      <p class="mt-3 font-serif text-xl text-[#6f3b24]">{page.season.theme}</p>
      {#if page.season.themeNote}
        <p class="mt-1 max-w-2xl text-sm text-[#8a6a55]">{page.season.themeNote}</p>
      {/if}
    {:else}
      <p class="mt-3 text-sm text-[#8a6a55]">{$t('studioSeasonFree')}</p>
    {/if}

    <div class="mt-3 flex flex-wrap items-center gap-4 text-[11px] uppercase tracking-[0.14em] text-[#8a6a55]">
      {#if page.season.state === 'open'}
        <span>{$t('studioDaysLeft').replace('{n}', String(daysLeft))}</span>
      {:else}
        <span>{$t('studioSeasonJudged')}</span>
      {/if}
      {#if token && page.mayRate}
        <span>{$t('studioRatingsLeft').replace('{n}', String(page.ratingsLeft))}</span>
      {:else if page.entered}
        <span>{$t('studioAuthorMayNotRate')}</span>
      {/if}
    </div>

    {#if said}<p class="mt-3 text-xs text-[#c65f3c]">{said}</p>{/if}

    <!-- Заявиться. Только допущенной работой. -->
    {#if token && !page.entered && page.season.state === 'open'}
      {#if mineFrames.length}
        <div class="mt-6 border border-[#d8c6b1] bg-[#fdf9f3] p-4">
          <p class="text-sm text-[#6f3b24]">{$t('studioEnterAsk')}</p>
          <div class="mt-2 flex flex-wrap gap-2">
            {#each mineFrames as one (one.id)}
              <button
                onclick={() => enter(one.id)}
                class="border border-[#34251c]/25 px-3 py-1.5 text-xs hover:bg-[#34251c]/5"
                >{one.name}</button
              >
            {/each}
          </div>
        </div>
      {:else if waiting.length}
        <p class="mt-6 text-sm text-[#8a6a55]">
          {$t('studioEnterWaiting').replace('{names}', waiting.join(', '))}
        </p>
      {:else}
        <p class="mt-6 text-sm text-[#8a6a55]">{$t('studioEnterNothing')}</p>
      {/if}
    {/if}

    {#if page.entries.length === 0}
      <p class="mt-8 text-sm text-[#8a6a55]">{$t('studioSeasonEmpty')}</p>
    {:else}
      <div class="mt-8 grid grid-cols-1 gap-8 sm:grid-cols-2 lg:grid-cols-3">
        {#each page.entries as entry (entry.id)}
          <div>
            <div style="max-width: 260px">
              <BattleCard card={sample} frames={[entry.body]} owned={true} />
            </div>
            <div class="mt-2 max-w-[260px]">
              <p class="text-sm text-[#34251c]">{entry.name}</p>
              <p class="text-[11px] text-[#8a6a55]">
                {entry.author}
                {#if entry.place}· {$t('studioPlace').replace('{n}', String(entry.place))}{/if}
              </p>

              <!-- Оценка: пять звёзд, своя видна, чужие — только числом. -->
              <div class="mt-1 flex items-center gap-2">
                {#if token && page.mayRate && entry.authorId !== authStore.user?.id && page.season.state === 'open'}
                  <span class="flex gap-0.5">
                    {#each [1, 2, 3, 4, 5] as star (star)}
                      <button
                        onclick={() => rate(entry.id, star)}
                        aria-label={String(star)}
                        class="text-base leading-none {(entry.mine ?? 0) >= star
                          ? 'text-[#c65f3c]'
                          : 'text-[#d8c6b1] hover:text-[#8a6a55]'}">★</button
                      >
                    {/each}
                  </span>
                {/if}
                <span class="text-[11px] text-[#8a6a55]">
                  {#if entry.votes > 0}
                    {$t('studioVotes').replace('{n}', String(entry.votes))}
                    {#if entry.score}· {entry.score.toFixed(2)}{/if}
                  {:else}
                    {$t('studioNoVotesYet')}
                  {/if}
                </span>
                <button
                  onclick={() => report(entry.frameId)}
                  class="ml-auto text-[10px] uppercase tracking-[0.14em] text-[#b0a08e] hover:text-[#8f2f22]"
                  >{$t('studioReport')}</button
                >
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>
