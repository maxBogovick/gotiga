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
  import StudioAsk from '$lib/components/studio/StudioAsk.svelte';
  import '$lib/components/studio/studio-room.css';
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
  /** Какой род выбрасывают. Своим окном: необратимое дом спрашивает сам. */
  let dropping = $state<StudioRace | null>(null);

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

  async function drop() {
    const race = dropping;
    dropping = null;
    if (!token || !race) return;
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

<svelte:head>
  <title>{$t('studioRaces')}</title>
  <meta name="description" content={$t('studioRacesLead')} />
</svelte:head>

<div class="studio-room">
  <div class="page" style="max-width: 60rem">
    <p class="eyebrow">
      <a href="/studio">{$t('studioBack')}</a>
      <span class="eyebrow-rule"></span>
      <span>{$t('studioEyebrow')}</span>
    </p>
    <h1 class="room-title">{$t('studioRaces')}</h1>
    <p class="room-lead">{$t('studioRacesLead')}</p>

    {#if said}<p class="said">{said}</p>{/if}

    {#if !token}
      <p class="empty">{$t('studioCardsSignIn')}</p>
    {:else if loading}
      <p class="empty">{$t('studioLoading')}</p>
    {:else}
      {#if editing}
        <div class="slip">
          <div class="field-grid grid--two">
            <label class="field">
              {$t('studioRaceNameRu')}
              <input bind:value={draft.nameRu} maxlength="60" />
            </label>
            <label class="field">
              {$t('studioRaceNameEn')}
              <input bind:value={draft.nameEn} maxlength="60" />
            </label>
            <label class="field">
              {$t('studioRaceNoteRu')}
              <textarea bind:value={draft.noteRu} rows="2" maxlength="200"></textarea>
            </label>
            <label class="field">
              {$t('studioRaceNoteEn')}
              <textarea bind:value={draft.noteEn} rows="2" maxlength="200"></textarea>
            </label>
          </div>

          <!-- Значок — из своего ящика: второго склада для этого нет. -->
          <p class="field" style="margin-top:1rem">{$t('studioRaceIcon')}</p>
          {#if box.length === 0}
            <p class="hint">
              {$t('studioCardArtEmpty')}
              <a href="/studio/assets">{$t('studioStore')}</a>
            </p>
          {:else}
            <div class="icons">
              {#each box as asset (asset.id)}
                <button
                  class="icon"
                  class:icon--lit={draft.iconUrl === asset.url}
                  onclick={() => (draft.iconUrl = asset.url)}
                >
                  <img src={asset.url} alt={asset.name} />
                </button>
              {/each}
              {#if draft.iconUrl}
                <button class="quiet quiet--danger" onclick={() => (draft.iconUrl = null)}
                  >{$t('studioCardArtClear')}</button
                >
              {/if}
            </div>
          {/if}

          <div class="doors">
            <button class="btn btn--lit" disabled={busy !== null} onclick={keep}
              >{$t('studioCardSave')}</button
            >
            <button class="btn" onclick={() => (editing = null)}>{$t('studioCancel')}</button>
          </div>
        </div>
      {:else}
        <div class="doors">
          <button class="btn btn--lit" onclick={begin}>{$t('studioRaceNew')}</button>
        </div>
      {/if}

      {#if races.length === 0}
        <p class="empty">{$t('studioRacesEmpty')}</p>
      {:else}
        <div class="rows">
          {#each races as race (race.id)}
            <div class="row">
              {#if race.iconUrl}
                <img class="row-icon" src={race.iconUrl} alt="" />
              {/if}
              <div class="row-words">
                <p class="name" style="margin:0">
                  {$lang === 'en' ? race.nameEn : race.nameRu}
                  <span style="opacity:.45">· {$lang === 'en' ? race.nameRu : race.nameEn}</span>
                </p>
                <p class="mark" class:mark--lit={race.status === 'taken' || !!race.approvedAt}>
                  {markOf(race)}
                </p>
                {#if race.noteRu || race.noteEn}
                  <p class="word">{$lang === 'en' ? race.noteEn : race.noteRu}</p>
                {/if}
                {#if race.keeperWord}
                  <p class="word">{race.keeperWord}</p>
                {/if}
              </div>
              <div class="deeds row-deeds">
                {#if race.status === 'taken' || race.approvedAt}
                  <span class="mark mark--lit" style="margin:0">{$t('studioRaceInHouse')}</span>
                {:else if race.status === 'shown'}
                  <button class="quiet" disabled={busy === race.id} onclick={() => withdraw(race)}
                    >{$t('studioCardTakeBack')}</button
                  >
                {:else}
                  <button class="quiet" onclick={() => edit(race)}>{$t('studioCardWork')}</button>
                  <button
                    class="quiet"
                    style="color:#c65f3c"
                    disabled={busy === race.id}
                    onclick={() => show(race)}>{$t('studioCardShow')}</button
                  >
                  <button
                    class="quiet quiet--danger"
                    disabled={busy === race.id}
                    onclick={() => (dropping = race)}>{$t('studioCardDrop')}</button
                  >
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</div>

{#if asking}
  <StudioAsk
    title={$t('studioAgreementTitle')}
    lead={$t('studioAgreementBody')}
    yes={$t('studioAgreementAgree')}
    onyes={agreeAndShow}
    onclose={() => (asking = null)}
  />
{/if}

{#if dropping}
  <StudioAsk
    title={$t('studioCardDrop')}
    lead={$t('studioRaceDropAsk')}
    yes={$t('studioCardDrop')}
    danger
    onyes={drop}
    onclose={() => (dropping = null)}
  />
{/if}

<style>
  .icons {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.6rem;
  }

  .icon {
    padding: 0.25rem;
    border: 1px solid #d8c6b1;
    background: none;
    cursor: pointer;
    line-height: 0;
  }

  .icon--lit {
    border-color: #c65f3c;
  }

  .icon img {
    width: 2.5rem;
    height: 2.5rem;
    object-fit: contain;
  }

  .rows {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-top: 2rem;
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 0.9rem;
    padding: 0.9rem 1rem;
    border: 1px solid #d8c6b1;
    background: #fdf9f3;
  }

  .row-icon {
    width: 2.5rem;
    height: 2.5rem;
    object-fit: contain;
  }

  .row-words {
    flex: 1 1 14rem;
    min-width: 0;
  }

  .row-deeds {
    margin: 0;
  }
</style>
