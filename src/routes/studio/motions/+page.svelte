<script lang="ts">
  // Свои движения: тот же стол такта, что у хозяина, и своя полка над ним.
  //
  // Стол один на весь дом — второй был бы вторым словарём жестов, а движок не
  // знает слов, которых у него нет. Здесь только транспорт: чем грузить, чем
  // сохранять и откуда брать картинки — свой ящик, а не склад дома.
  //
  // Полка сверху, а не сбоку и не под столом: сцена прибита и берёт всю
  // оставшуюся высоту, и всё, что встанет под ней, человек увидит только
  // прокруткой — то есть не увидит.
  import { onMount } from 'svelte';
  import { t, lang } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import BattleMotionsDesk from '$lib/components/BattleMotionsDesk.svelte';
  import '$lib/components/studio/studio-room.css';
  import StudioAssetPicker from '$lib/components/studio/StudioAssetPicker.svelte';
  import StudioAsk from '$lib/components/studio/StudioAsk.svelte';
  import type { Motion, StudioMotion } from '$lib/types/api';

  let mine = $state<StudioMotion[]>([]);
  let said = $state<string | null>(null);
  let busy = $state<string | null>(null);
  /** Кому отдать выбранную из ящика картинку. Обещание, а не колбэк: стол ждёт
   *  адрес, и ждать его удобнее там, где его попросили. */
  let waiting = $state<((url: string | null) => void) | null>(null);
  let asking = $state(false);

  let token = $derived(authStore.token);

  onMount(reload);

  async function reload() {
    if (!token) return;
    mine = await api.getStudioMotions(token).catch(() => []);
  }

  function flash(text: string, ms = 7000) {
    said = text;
    setTimeout(() => (said = null), ms);
  }

  function pickFromStore(): Promise<string | null> {
    return new Promise((resolve) => {
      waiting = resolve;
    });
  }

  /** Отдать хозяину. Соглашение спрашивается тем же жестом, что у рам и карт:
   *  человек нажал «показать», а не «согласиться». */
  async function show(one: StudioMotion) {
    if (!token) return;
    busy = one.id;
    try {
      await api.showStudioMotion(token, one.id);
      await reload();
      flash($t('studioMotionShown'));
    } catch (e) {
      if (String(e).includes('needsAgreement')) asking = true;
      else flash(String(e));
    }
    busy = null;
  }

  async function agreeAndReload() {
    if (!token) return;
    asking = false;
    await api.acceptStudioAgreement(token).catch((e) => flash(String(e)));
    flash($t('studioMotionAgreed'));
  }

  async function withdraw(one: StudioMotion) {
    if (!token) return;
    busy = one.id;
    await api.withdrawStudioMotion(token, one.id).catch((e) => flash(String(e)));
    await reload();
    busy = null;
  }

  const nameOf = (one: StudioMotion) =>
    ($lang === 'en' ? one.body.nameEn : one.body.nameRu) || one.body.id;

  /** Что с движением сейчас — одним словом. Взятое домом названо своим: оно
   *  стоит в своде и здесь больше не правится. */
  function markOf(one: StudioMotion): string {
    if (one.status === 'taken') return $t('studioMotionInHouse');
    if (one.status === 'shown') return $t('studioCardWaiting');
    if (one.status === 'withdrawn') return $t('studioCardTakenBack');
    return $t('studioCardDraft');
  }
</script>

<svelte:head>
  <title>{$t('studioMotions')}</title>
  <meta name="description" content={$t('studioMotionsAbout')} />
</svelte:head>

{#if !token}
  <div class="studio-room">
    <div class="page">
      <p class="eyebrow">
        <a href="/studio">{$t('studioBack')}</a>
        <span class="eyebrow-rule"></span>
        <span>{$t('studioEyebrow')}</span>
      </p>
      <h1 class="room-title">{$t('studioMotions')}</h1>
      <p class="room-lead">{$t('studioMotionsSignIn')}</p>
      <div class="doors"><a class="btn" href="/login">{$t('studioSignIn')}</a></div>
    </div>
  </div>
{:else}
  <div class="flex h-[100dvh] flex-col bg-[#f8f1e7]">
    <!-- Полка своих движений: что отдано, что взято домом и что ещё в руках. -->
    <div class="strip studio-room">
      <a class="quiet" href="/studio">← {$t('studioBack')}</a>
      <span class="strip-name">{$t('studioMotions')}</span>
      {#each mine as one (one.id)}
        <span class="slip-one">
          <span class="slip-name">{nameOf(one)}</span>
          <span class="mark" class:mark--lit={one.status === 'taken'} style="margin:0"
            >{markOf(one)}</span
          >
          {#if one.status === 'shown'}
            <button class="quiet" disabled={busy === one.id} onclick={() => withdraw(one)}
              >{$t('studioCardTakeBack')}</button
            >
          {:else if one.status !== 'taken'}
            <button
              class="quiet"
              style="color:#c65f3c"
              disabled={busy === one.id}
              onclick={() => show(one)}>{$t('studioCardShow')}</button
            >
          {/if}
          {#if one.keeperWord}<span class="slip-word">{one.keeperWord}</span>{/if}
        </span>
      {/each}
      {#if said}<span class="slip-word" style="color:#c65f3c">{said}</span>{/if}
    </div>

    <div class="min-h-0 flex-1">
      <BattleMotionsDesk
        {flash}
        onSaved={() => void reload()}
        loadMotions={async () => (await api.getStudioMotions(token!)).map((m) => m.body)}
        saveMotions={async (motions: Motion[]) =>
          (await api.saveStudioMotions(token!, motions, $lang)).map((m) => m.body)}
        loadCards={() => api.getBattleCards()}
        loadFrames={() => api.getBattleFrames()}
        loadRaces={() => api.getBattleRaces()}
        uploadArt={async (file) =>
          (await api.uploadStudioAsset(token!, file, file.name.slice(0, 60) || 'кадр', 'art')).url}
        {pickFromStore}
      />
    </div>
  </div>

  {#if waiting}
    <StudioAssetPicker
      role="art"
      onpick={(url) => {
        waiting?.(url);
        waiting = null;
      }}
      onclose={() => {
        waiting?.(null);
        waiting = null;
      }}
    />
  {/if}

  {#if asking}
    <StudioAsk
      title={$t('studioAgreementTitle')}
      lead={$t('studioAgreementBody')}
      yes={$t('studioAgreementAgree')}
      onyes={agreeAndReload}
      onclose={() => (asking = false)}
    />
  {/if}
{/if}

<style>
  /* Полка своих движений — тонкой планкой над столом: стол прибит и берёт всю
     оставшуюся высоту, и всё, что встанет под ним, человек увидит только
     прокруткой, то есть не увидит. */
  .strip {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem 1rem;
    min-height: auto;
    padding: 0.5rem 1rem;
    border-bottom: 1px solid #d8c6b1;
  }

  .strip::before {
    content: none;
  }

  .strip-name {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1rem;
  }

  .slip-one {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    padding: 0.2rem 0.6rem;
    border: 1px solid #d8c6b1;
    background: #fdf9f3;
  }

  .slip-name {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.92rem;
    color: #34251c;
  }

  .slip-word {
    font-family: Georgia, 'Fraunces', serif;
    font-style: italic;
    font-size: 0.85rem;
    color: #6f3b24;
  }
</style>
