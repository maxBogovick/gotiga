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
  import type { BattleFrame, BattleRace, StudioCard } from '$lib/types/api';

  let cards = $state<StudioCard[]>([]);
  let frames = $state<BattleFrame[] | null>(null);
  let races = $state<BattleRace[]>([]);
  let loading = $state(true);
  let said = $state<string | null>(null);
  let busy = $state<string | null>(null);
  /** Не согласился с нынешней редакцией. Спрашивается в тот миг, когда работа
   *  впервые уходит из рук, — тем же жестом, что и у рам: человек нажал
   *  «показать», а не «согласиться». */
  let needsAgreement = $state(true);
  let asking = $state<StudioCard | null>(null);

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
    needsAgreement = !studio?.agreed;
    frames = dress?.frames ?? null;
    races = race;
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

  async function drop(card: StudioCard) {
    if (!token) return;
    if (!confirm($t('studioCardDropAsk'))) return;
    busy = card.id;
    await api.deleteStudioCard(token, card.id).catch((e) => flash(String(e)));
    await reload();
    busy = null;
  }

  function nameOf(card: StudioCard): string {
    const body = card.body;
    return (($lang === 'en' ? body.titleEn : body.titleRu) || body.titleRu || body.titleEn || '').trim();
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

<svelte:head><title>{$t('studioCards')}</title></svelte:head>

<div class="mx-auto max-w-5xl px-5 py-10">
  <div class="flex flex-wrap items-baseline justify-between gap-3">
    <h1 class="font-serif text-2xl text-[#34251c]">{$t('studioCards')}</h1>
    <a href="/studio" class="text-xs uppercase tracking-[0.16em] text-[#8a6a55] hover:text-[#c65f3c]"
      >← {$t('studioBack')}</a
    >
  </div>
  <p class="mt-2 max-w-2xl text-sm leading-relaxed text-[#6f3b24]">{$t('studioCardsLead')}</p>

  {#if said}<p class="mt-3 text-xs text-[#c65f3c]">{said}</p>{/if}

  {#if !token}
    <p class="mt-8 text-sm text-[#8a6a55]">{$t('studioCardsSignIn')}</p>
  {:else if loading}
    <p class="mt-8 text-sm text-[#8a6a55]">{$t('studioLoading')}</p>
  {:else}
    <button
      onclick={begin}
      class="mt-6 border border-[#c65f3c]/40 px-4 py-2 text-xs uppercase tracking-[0.16em] text-[#c65f3c] hover:bg-[#c65f3c]/8"
      >{$t('studioCardNew')}</button
    >

    {#if cards.length === 0}
      <p class="mt-8 text-sm text-[#8a6a55]">{$t('studioCardsEmpty')}</p>
    {:else}
      <div class="mt-8 grid grid-cols-2 gap-6 sm:grid-cols-3 lg:grid-cols-4">
        {#each cards as card (card.id)}
          <div>
            <BattleCard card={cardFromRequest(card.body, races)} {frames} owned={true} />
            <p class="mt-2 truncate text-sm text-[#34251c]">
              {nameOf(card) || $t('studioCardNoName')}
            </p>
            <p class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">{markOf(card)}</p>
            {#if card.keeperWord}
              <!-- Слово хозяина — целиком: отказ, из которого не видно, что
                   чинить, вернётся той же работой. -->
              <p class="mt-1 text-[11px] italic leading-relaxed text-[#6f3b24]">{card.keeperWord}</p>
            {/if}
            <div class="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-[10px] uppercase tracking-[0.14em]">
              {#if card.status === 'taken' || card.approvedAt}
                <a href="/battles?card={card.cardId}" class="text-[#c65f3c] hover:underline"
                  >{$t('studioCardSeeOnShelf')}</a
                >
              {:else if card.status === 'shown'}
                <button
                  onclick={() => withdraw(card)}
                  disabled={busy === card.id}
                  class="text-[#8a6a55] hover:text-[#c65f3c] disabled:opacity-40"
                  >{$t('studioCardTakeBack')}</button
                >
              {:else}
                <a href="/studio/cards/{card.id}" class="text-[#8a6a55] hover:text-[#c65f3c]"
                  >{$t('studioCardWork')}</a
                >
                <button
                  onclick={() => show(card)}
                  disabled={busy === card.id}
                  class="text-[#c65f3c] hover:underline disabled:opacity-40"
                  >{$t('studioCardShow')}</button
                >
                <button
                  onclick={() => drop(card)}
                  disabled={busy === card.id}
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

  <!-- Соглашение спрашивается ровно в тот миг, когда работа впервые выходит на
       люди, и одной кнопкой. -->
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
