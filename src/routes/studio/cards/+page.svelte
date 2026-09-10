<script lang="ts">
  // Свои карты: полка работ человека.
  //
  // Карта показана НАСТОЯЩИМ отрисовщиком — тем же, что стоит на полке дома и
  // в бою. Второй облик однажды соврал бы, а здесь врать особенно дорого:
  // человек по нему решает, что отдавать хозяину.
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { t, lang } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { cardFromRequest, emptyCardRequest } from '$lib/battles';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import StudioAsk from '$lib/components/studio/StudioAsk.svelte';
  import '$lib/components/studio/studio-room.css';
  import type { BattleFrame, BattleRace, StudioCard } from '$lib/types/api';

  let cards = $state<StudioCard[]>([]);
  let frames = $state<BattleFrame[] | null>(null);
  let races = $state<BattleRace[]>([]);
  let loading = $state(true);
  let said = $state<string | null>(null);
  let busy = $state<string | null>(null);
  /** Не согласился с нынешней редакцией. Спрашивается в тот миг, когда работа
   *  впервые уходит из рук, — тем же жестом, что и у рам. */
  let needsAgreement = $state(true);
  let asking = $state<StudioCard | null>(null);
  /** Какую работу выбрасывают. Своим окном: необратимое дом спрашивает сам. */
  let dropping = $state<StudioCard | null>(null);

  let token = $derived(authStore.token);

  onMount(reload);

  async function reload() {
    loading = true;
    if (!token) {
      loading = false;
      return;
    }
    const [mine, dress, race, studio] = await Promise.all([
      api.getStudioCards(token).catch(() => []),
      api.getBattleFrames().catch(() => null),
      api.getBattleRaces().catch(() => []),
      api.getStudio(token).catch(() => null),
    ]);
    cards = mine;
    frames = dress?.frames ?? null;
    races = race;
    needsAgreement = !studio?.agreed;
    loading = false;
  }

  function flash(text: string, ms = 7000) {
    said = text;
    setTimeout(() => (said = null), ms);
  }

  async function begin() {
    if (!token) return;
    const made = await api
      .saveStudioCard(token, null, emptyCardRequest(), $lang)
      .catch((e) => {
        flash(String(e));
        return null;
      });
    if (made) await goto(`/studio/cards/${made.id}`);
  }

  async function show(card: StudioCard) {
    if (!token) return;
    // Соглашение — то же, что у рам: им человек разрешает дому пользоваться
    // работой, и без него сервер её не примет.
    if (needsAgreement) {
      asking = card;
      return;
    }
    busy = card.id;
    try {
      await api.showStudioCard(token, card.id);
      await reload();
      flash($t('studioCardShown'));
    } catch (e) {
      // Отказ весов приходит списком причин — он и есть ответ на «почему»,
      // и прятать его нельзя: человек иначе не знает, что чинить.
      flash(String(e));
    }
    busy = null;
  }

  async function agreeAndShow() {
    const card = asking;
    if (!token || !card) return;
    asking = null;
    await api.acceptStudioAgreement(token).catch((e) => flash(String(e)));
    needsAgreement = false;
    await show(card);
  }

  async function withdraw(card: StudioCard) {
    if (!token) return;
    busy = card.id;
    await api.withdrawStudioCard(token, card.id).catch((e) => flash(String(e)));
    await reload();
    busy = null;
  }

  async function drop() {
    const card = dropping;
    dropping = null;
    if (!token || !card) return;
    busy = card.id;
    await api.deleteStudioCard(token, card.id).catch((e) => flash(String(e)));
    await reload();
    busy = null;
  }

  function nameOf(card: StudioCard): string {
    const body = card.body;
    return (
      ($lang === 'en' ? body.titleEn : body.titleRu) ||
      body.titleRu ||
      body.titleEn ||
      ''
    ).trim();
  }

  /** Что с работой сейчас — одним словом. Взятое домом названо своим словом
   *  (`taken`), и это состояние терминальное: ни править, ни показывать. */
  function markOf(card: StudioCard): string {
    if (card.status === 'taken' || card.approvedAt) return $t('studioCardOnShelf');
    if (card.status === 'shown') return $t('studioCardWaiting');
    if (card.status === 'withdrawn') return $t('studioCardTakenBack');
    return $t('studioCardDraft');
  }
</script>

<svelte:head>
  <title>{$t('studioCards')}</title>
  <meta name="description" content={$t('studioCardsLead')} />
</svelte:head>

<div class="studio-room">
  <div class="page">
    <p class="eyebrow">
      <a href="/studio">{$t('studioBack')}</a>
      <span class="eyebrow-rule"></span>
      <span>{$t('studioEyebrow')}</span>
    </p>
    <h1 class="room-title">{$t('studioCards')}</h1>
    <p class="room-lead">{$t('studioCardsLead')}</p>

    {#if said}<p class="said">{said}</p>{/if}

    {#if !token}
      <p class="empty">{$t('studioCardsSignIn')}</p>
    {:else if loading}
      <p class="empty">{$t('studioLoading')}</p>
    {:else}
      <div class="doors">
        <button class="btn btn--lit" onclick={begin}>{$t('studioCardNew')}</button>
      </div>

      {#if cards.length === 0}
        <p class="empty">{$t('studioCardsEmpty')}</p>
      {:else}
        <div class="shelf">
          {#each cards as card (card.id)}
            <div>
              <BattleCard card={cardFromRequest(card.body, races)} {frames} owned={true} />
              <p class="name">{nameOf(card) || $t('studioCardNoName')}</p>
              <p class="mark" class:mark--lit={card.status === 'taken' || !!card.approvedAt}>
                {markOf(card)}
              </p>
              {#if card.keeperWord}
                <!-- Слово хозяина — целиком: отказ, из которого не видно, что
                     чинить, вернётся той же работой. -->
                <p class="word">{card.keeperWord}</p>
              {/if}
              <div class="deeds">
                {#if card.status === 'taken' || card.approvedAt}
                  <a class="quiet" href="/battles?card={card.cardId}"
                    >{$t('studioCardSeeOnShelf')}</a
                  >
                {:else if card.status === 'shown'}
                  <button
                    class="quiet"
                    disabled={busy === card.id}
                    onclick={() => withdraw(card)}>{$t('studioCardTakeBack')}</button
                  >
                {:else}
                  <a class="quiet" href="/studio/cards/{card.id}">{$t('studioCardWork')}</a>
                  <button
                    class="quiet"
                    style="color:#c65f3c"
                    disabled={busy === card.id}
                    onclick={() => show(card)}>{$t('studioCardShow')}</button
                  >
                  <button
                    class="quiet quiet--danger"
                    disabled={busy === card.id}
                    onclick={() => (dropping = card)}>{$t('studioCardDrop')}</button
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
  <!-- Соглашение спрашивается ровно в тот миг, когда работа впервые выходит на
       люди, и одной кнопкой. -->
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
    lead={$t('studioCardDropAsk')}
    yes={$t('studioCardDrop')}
    danger
    onyes={drop}
    onclose={() => (dropping = null)}
  />
{/if}
