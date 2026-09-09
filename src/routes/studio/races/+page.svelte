<script lang="ts">
  // Свои роды: словарные строки, которые однажды наденут чужие карты.
  //
  // Взвешивать в роде нечего — есть имя, слово о нём и значок, и решает хозяин
  // глазами. Поэтому здесь нет весов и нет стола: список и короткая форма.
  //
  // Слова состояния те же, что у карт: `taken` терминально — род стал строкой
  // словаря дома, и править его больше нельзя, потому что его уже носят чужие
  // карты.
  import { onMount } from 'svelte';
  import { t, lang } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import type { SaveStudioRaceRequest, StudioAsset, StudioRace } from '$lib/types/api';

  let races = $state<StudioRace[]>([]);
  let box = $state<StudioAsset[]>([]);
  let loading = $state(true);
  let said = $state<string | null>(null);
  let busy = $state<string | null>(null);
  /** Какой род правим. Один за раз: род — три строки, и десять открытых форм
   *  это десять способов ошибиться строкой. */
  let editing = $state<string | null>(null);
  let draft = $state<SaveStudioRaceRequest>(empty());
  let needsAgreement = $state(true);
  let asking = $state<StudioRace | null>(null);

  let token = $derived(authStore.token);

  function empty(): SaveStudioRaceRequest {
    return { nameEn: '', nameRu: '', noteEn: '', noteRu: '', iconUrl: null };
  }

  onMount(reload);

  async function reload() {
    loading = true;
    if (!token) {
      loading = false;
      return;
    }
    const [mine, studio] = await Promise.all([
      api.getStudioRaces(token).catch(() => []),
      api.getStudio(token).catch(() => null),
    ]);
    races = mine;
    box = studio?.box.assets ?? [];
    needsAgreement = !studio?.agreed;
    loading = false;
  }

  function flash(text: string, ms = 7000) {
    said = text;
    setTimeout(() => (said = null), ms);
  }

  function begin() {
    editing = 'new';
    draft = empty();
  }

  function edit(race: StudioRace) {
    editing = race.id;
    draft = {
      nameEn: race.nameEn,
      nameRu: race.nameRu,
      noteEn: race.noteEn ?? '',
      noteRu: race.noteRu ?? '',
      iconUrl: race.iconUrl ?? null,
    };
  }

  async function keep() {
    if (!token || !editing) return;
    busy = editing;
    try {
      await api.saveStudioRace(token, editing === 'new' ? null : editing, {
        ...draft,
        lang: $lang,
      });
      editing = null;
      await reload();
      flash($t('studioRaceKept'));
    } catch (e) {
      flash(String(e));
    }
    busy = null;
  }

  async function show(race: StudioRace) {
    if (!token) return;
    // Соглашение — то же, что у рам и карт: им человек разрешает дому
    // пользоваться своей работой.
    if (needsAgreement) {
      asking = race;
      return;
    }
    busy = race.id;
    try {
      await api.showStudioRace(token, race.id);
      await reload();
      flash($t('studioRaceShown'));
    } catch (e) {
      flash(String(e));
    }
    busy = null;
  }

  async function agreeAndShow() {
    const race = asking;
    if (!token || !race) return;
    asking = null;
    await api.acceptStudioAgreement(token).catch((e) => flash(String(e)));
    needsAgreement = false;
    await show(race);
  }

  async function withdraw(race: StudioRace) {
    if (!token) return;
    busy = race.id;
    await api.withdrawStudioRace(token, race.id).catch((e) => flash(String(e)));
    await reload();
    busy = null;
  }

  async function drop(race: StudioRace) {
    if (!token) return;
    if (!confirm($t('studioRaceDropAsk'))) return;
    busy = race.id;
    await api.deleteStudioRace(token, race.id).catch((e) => flash(String(e)));
    await reload();
    busy = null;
  }

  function markOf(race: StudioRace): string {
    if (race.status === 'taken' || race.approvedAt) return $t('studioRaceInHouse');
    if (race.status === 'shown') return $t('studioCardWaiting');
    if (race.status === 'withdrawn') return $t('studioCardTakenBack');
    return $t('studioCardDraft');
  }
</script>

<svelte:head><title>{$t('studioRaces')}</title></svelte:head>

<div class="mx-auto max-w-4xl px-5 py-10">
  <div class="flex flex-wrap items-baseline justify-between gap-3">
    <h1 class="font-serif text-2xl text-[#34251c]">{$t('studioRaces')}</h1>
    <a href="/studio" class="text-xs uppercase tracking-[0.16em] text-[#8a6a55] hover:text-[#c65f3c]"
      >← {$t('studioBack')}</a
    >
  </div>
  <p class="mt-2 max-w-2xl text-sm leading-relaxed text-[#6f3b24]">{$t('studioRacesLead')}</p>

  {#if said}<p class="mt-3 text-xs text-[#c65f3c]">{said}</p>{/if}

  {#if !token}
    <p class="mt-8 text-sm text-[#8a6a55]">{$t('studioCardsSignIn')}</p>
  {:else if loading}
    <p class="mt-8 text-sm text-[#8a6a55]">{$t('studioLoading')}</p>
  {:else}
    {#if editing}
      <div class="mt-6 space-y-3 border border-[#d8c6b1] bg-[#fdf9f3] p-4">
        <div class="grid gap-3 sm:grid-cols-2">
          <label class="block text-[11px] text-[#6f3b24]">
            {$t('studioRaceNameRu')}
            <input
              bind:value={draft.nameRu}
              maxlength="60"
              class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
            />
          </label>
          <label class="block text-[11px] text-[#6f3b24]">
            {$t('studioRaceNameEn')}
            <input
              bind:value={draft.nameEn}
              maxlength="60"
              class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
            />
          </label>
          <label class="block text-[11px] text-[#6f3b24]">
            {$t('studioRaceNoteRu')}
            <textarea
              bind:value={draft.noteRu}
              rows="2"
              maxlength="200"
              class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
            ></textarea>
          </label>
          <label class="block text-[11px] text-[#6f3b24]">
            {$t('studioRaceNoteEn')}
            <textarea
              bind:value={draft.noteEn}
              rows="2"
              maxlength="200"
              class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
            ></textarea>
          </label>
        </div>

        <!-- Значок — из своего ящика: второго склада для этого нет. -->
        <div>
          <p class="text-[11px] text-[#6f3b24]">{$t('studioRaceIcon')}</p>
          {#if box.length === 0}
            <p class="mt-1 text-[11px] text-[#8a6a55]">
              {$t('studioCardArtEmpty')}
              <a href="/studio/assets" class="underline">{$t('studioStore')}</a>
            </p>
          {:else}
            <div class="mt-2 flex flex-wrap gap-2">
              {#each box as asset (asset.id)}
                <button
                  onclick={() => (draft.iconUrl = asset.url)}
                  class="border p-1 {draft.iconUrl === asset.url
                    ? 'border-[#c65f3c]'
                    : 'border-[#d8c6b1] hover:border-[#8a6a55]'}"
                >
                  <img src={asset.url} alt={asset.name} class="h-10 w-10 object-contain" />
                </button>
              {/each}
              {#if draft.iconUrl}
                <button
                  onclick={() => (draft.iconUrl = null)}
                  class="self-center text-[10px] uppercase tracking-[0.14em] text-[#b0a08e] hover:text-[#8f2f22]"
                  >{$t('studioCardArtClear')}</button
                >
              {/if}
            </div>
          {/if}
        </div>

        <div class="flex flex-wrap gap-3">
          <button
            onclick={keep}
            disabled={busy !== null}
            class="border border-[#c65f3c]/40 px-4 py-2 text-xs uppercase tracking-[0.16em] text-[#c65f3c] hover:bg-[#c65f3c]/8 disabled:opacity-40"
            >{$t('studioCardSave')}</button
          >
          <button
            onclick={() => (editing = null)}
            class="border border-[#34251c]/20 px-4 py-2 text-xs uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
            >{$t('studioCancel')}</button
          >
        </div>
      </div>
    {:else}
      <button
        onclick={begin}
        class="mt-6 border border-[#c65f3c]/40 px-4 py-2 text-xs uppercase tracking-[0.16em] text-[#c65f3c] hover:bg-[#c65f3c]/8"
        >{$t('studioRaceNew')}</button
      >
    {/if}

    {#if races.length === 0}
      <p class="mt-8 text-sm text-[#8a6a55]">{$t('studioRacesEmpty')}</p>
    {:else}
      <div class="mt-8 space-y-3">
        {#each races as race (race.id)}
          <div class="flex flex-wrap items-start gap-3 border border-[#d8c6b1] bg-[#fdf9f3] p-3">
            {#if race.iconUrl}
              <img src={race.iconUrl} alt="" class="h-10 w-10 object-contain" />
            {/if}
            <div class="min-w-0 flex-1">
              <p class="text-sm text-[#34251c]">
                {$lang === 'en' ? race.nameEn : race.nameRu}
                <span class="text-[#b0a08e]">· {$lang === 'en' ? race.nameRu : race.nameEn}</span>
              </p>
              <p class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">{markOf(race)}</p>
              {#if race.noteRu || race.noteEn}
                <p class="mt-1 text-[11px] italic text-[#6f3b24]">
                  {$lang === 'en' ? race.noteEn : race.noteRu}
                </p>
              {/if}
              {#if race.keeperWord}
                <p class="mt-1 text-[11px] italic leading-relaxed text-[#8f2f22]">
                  {race.keeperWord}
                </p>
              {/if}
            </div>
            <div class="flex flex-wrap gap-3 text-[10px] uppercase tracking-[0.14em]">
              {#if race.status === 'taken' || race.approvedAt}
                <span class="text-[#c65f3c]">{$t('studioRaceInHouse')}</span>
              {:else if race.status === 'shown'}
                <button
                  onclick={() => withdraw(race)}
                  disabled={busy === race.id}
                  class="text-[#8a6a55] hover:text-[#c65f3c] disabled:opacity-40"
                  >{$t('studioCardTakeBack')}</button
                >
              {:else}
                <button
                  onclick={() => edit(race)}
                  class="text-[#8a6a55] hover:text-[#c65f3c]">{$t('studioCardWork')}</button
                >
                <button
                  onclick={() => show(race)}
                  disabled={busy === race.id}
                  class="text-[#c65f3c] hover:underline disabled:opacity-40"
                  >{$t('studioCardShow')}</button
                >
                <button
                  onclick={() => drop(race)}
                  disabled={busy === race.id}
                  class="text-[#b0a08e] hover:text-[#8f2f22] disabled:opacity-40"
                  >{$t('studioCardDrop')}</button
                >
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {/if}

  {#if asking}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-[#34251c]/40 p-4">
      <div class="max-w-lg border border-[#d8c6b1] bg-[#f8f1e7] p-6">
        <h2 class="font-serif text-xl text-[#34251c]">{$t('studioAgreementTitle')}</h2>
        <p class="mt-3 text-sm leading-relaxed text-[#6f3b24]">{$t('studioAgreementBody')}</p>
        <div class="mt-5 flex flex-wrap gap-3">
          <button
            onclick={agreeAndShow}
            class="border border-[#c65f3c]/60 px-4 py-2 text-xs uppercase tracking-[0.16em] text-[#c65f3c] hover:bg-[#c65f3c]/8"
            >{$t('studioAgreementAgree')}</button
          >
          <button
            onclick={() => (asking = null)}
            class="border border-[#34251c]/20 px-4 py-2 text-xs uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
            >{$t('studioAgreementNo')}</button
          >
        </div>
      </div>
    </div>
  {/if}
</div>
