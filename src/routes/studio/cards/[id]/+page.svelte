<script lang="ts">
  // Стол своей карты.
  //
  // Правят не форму рядом с карточкой, а САМУ карту: она стоит крупно и
  // перерисовывается на каждое число. Отрисовщик тот же, что на полке и в бою.
  //
  // Весы стоят рядом и видны всегда. Без них человек назначает 99 силы, а
  // объясняться словами приходится хозяину; с весами объяснять нечего — чашка
  // перевешивает, и это видно. Считает их СЕРВЕР, той же функцией, какой
  // взвешивает карту хозяин: второй счёт однажды разошёлся бы с первым.
  import { page as routePage } from '$app/stores';
  import { goto } from '$app/navigation';
  import { t, lang } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { cardFromRequest } from '$lib/battles';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import BattleAbilities from '$lib/components/BattleAbilities.svelte';
  import type {
    BattleFrame,
    BattleRace,
    BattleWeigh,
    SaveBattleCardRequest,
    StudioAsset,
    StudioCard,
  } from '$lib/types/api';

  let work = $state<StudioCard | null>(null);
  let body = $state<SaveBattleCardRequest | null>(null);
  let frames = $state<BattleFrame[] | null>(null);
  let races = $state<BattleRace[]>([]);
  let box = $state<StudioAsset[]>([]);
  let weigh = $state<BattleWeigh | null>(null);
  let loading = $state(true);
  let saving = $state(false);
  let said = $state<string | null>(null);
  let picking = $state(false);
  /** Что было сохранено последним. По нему, а не по флагу, видно «не
   *  сохранено»: флаг и правда однажды разойдутся. */
  let stored = $state('');

  let token = $derived(authStore.token);
  let id = $derived($routePage.params.id ?? '');
  let dirty = $derived(body ? JSON.stringify(body) !== stored : false);

  // Работа перечитывается на смену адреса, а не один раз при рождении:
  // переход с одной карты на другую меняет только `id`.
  $effect(() => {
    const which = id;
    if (!which || !token) return;
    void load(which);
  });

  async function load(which: string) {
    loading = true;
    const [mine, dress, race, studio] = await Promise.all([
      api.getStudioCards(token!).catch(() => []),
      api.getBattleFrames().catch(() => null),
      api.getBattleRaces().catch(() => []),
      api.getStudio(token!).catch(() => null),
    ]);
    work = mine.find((c) => c.id === which) ?? null;
    body = work ? { ...work.body } : null;
    stored = body ? JSON.stringify(body) : '';
    frames = dress?.frames ?? null;
    races = race;
    box = studio?.box.assets ?? [];
    loading = false;
    void reweigh();
  }

  function flash(text: string, ms = 8000) {
    said = text;
    setTimeout(() => (said = null), ms);
  }

  /** Весы спрашиваются с задержкой: человек тянет число, а не жмёт кнопку, и
   *  запрос на каждый удар по клавише был бы разговором ни о чём. */
  let scaleTimer: ReturnType<typeof setTimeout> | null = null;
  function reweighSoon() {
    if (scaleTimer) clearTimeout(scaleTimer);
    scaleTimer = setTimeout(reweigh, 400);
  }

  // Перевешивается ВСЯ работа целиком, а не поля, к которым привязали ручку.
  // Ручки на полях были ровно этой ошибкой: добавленная способность весит
  // больше всего, что есть на карте, а весы про неё молчали — про числа
  // помнили, про черты и способности забыли. Слепок тела — единственный
  // сторож, который ничего не забудет.
  $effect(() => {
    if (!body) return;
    JSON.stringify(body);
    reweighSoon();
  });

  async function reweigh() {
    if (!token || !body) return;
    // Спрашиваем про ОПУБЛИКОВАННУЮ карту: черновику дом прощает почти всё, а
    // отдавать работу придётся именно опубликованной.
    weigh = await api
      .weighStudioCard(token, { ...body, status: 'published' })
      .catch(() => null);
  }

  async function save() {
    if (!token || !body || !work) return;
    saving = true;
    try {
      const kept = await api.saveStudioCard(token, work.id, body, $lang);
      work = kept;
      body = { ...kept.body };
      stored = JSON.stringify(body);
      flash($t('studioCardSaved'));
    } catch (e) {
      flash(String(e));
    }
    saving = false;
  }

  async function show() {
    if (!token || !work) return;
    if (dirty) await save();
    try {
      await api.showStudioCard(token, work.id);
      await goto('/studio/cards');
    } catch (e) {
      flash(String(e));
    }
  }

  function addTrait() {
    if (!body) return;
    body.traits = [...body.traits, { nameEn: '', nameRu: '', textEn: '', textRu: '' }];
  }

  function dropTrait(at: number) {
    if (!body) return;
    body.traits = body.traits.filter((_, i) => i !== at);
  }

  /** Слово отказа — то же, которым откажет сервер. */
  const faultWord = (fault: string) =>
    $t(`studioFault${fault[0].toUpperCase()}${fault.slice(1)}` as never);

  const NUMBERS: { key: keyof SaveBattleCardRequest; word: string }[] = [
    { key: 'cost', word: 'studioNumCost' },
    { key: 'power', word: 'studioNumPower' },
    { key: 'health', word: 'studioNumHealth' },
    { key: 'mana', word: 'studioNumMana' },
    { key: 'armor', word: 'studioNumArmor' },
    { key: 'ward', word: 'studioNumWard' },
    { key: 'reach', word: 'studioNumReach' },
    { key: 'step', word: 'studioNumStep' },
    { key: 'speed', word: 'studioNumSpeed' },
    { key: 'mend', word: 'studioNumMend' },
  ];

  let overweight = $derived(
    weigh ? weigh.totalPoints > weigh.tierBudget : false,
  );
</script>

<svelte:head><title>{$t('studioCardDesk')}</title></svelte:head>

<div class="mx-auto max-w-6xl px-5 py-8">
  <div class="flex flex-wrap items-baseline justify-between gap-3">
    <h1 class="font-serif text-2xl text-[#34251c]">{$t('studioCardDesk')}</h1>
    <a
      href="/studio/cards"
      class="text-xs uppercase tracking-[0.16em] text-[#8a6a55] hover:text-[#c65f3c]"
      >← {$t('studioCards')}</a
    >
  </div>

  {#if said}<p class="mt-3 text-xs text-[#c65f3c]">{said}</p>{/if}

  {#if loading}
    <p class="mt-8 text-sm text-[#8a6a55]">{$t('studioLoading')}</p>
  {:else if !work || !body}
    <p class="mt-8 text-sm text-[#8a6a55]">{$t('studioCardGone')}</p>
  {:else if work.status === 'shown'}
    <p class="mt-8 text-sm text-[#6f3b24]">{$t('studioCardShownLocked')}</p>
  {:else if work.status === 'taken'}
    <!-- Взятая домом не правится, и это не строгость: карта уже отпечатана и
         стоит на полке, а запись под ней — память о том, ЧТО принесли. -->
    <p class="mt-8 text-sm text-[#6f3b24]">{$t('studioCardTakenLocked')}</p>
    {#if work.cardId}
      <a
        href="/battles?card={work.cardId}"
        class="mt-3 inline-block text-xs uppercase tracking-[0.16em] text-[#c65f3c] hover:underline"
        >{$t('studioCardSeeOnShelf')}</a
      >
    {/if}
  {:else}
    <div class="mt-6 grid gap-8 lg:grid-cols-[320px_1fr]">
      <!-- Карта и весы. Держатся при прокрутке: правят числа, глядя на них. -->
      <div class="lg:sticky lg:top-6 lg:self-start">
        <BattleCard card={cardFromRequest(body, races)} {frames} owned={true} />

        <div class="mt-4 border border-[#d8c6b1] bg-[#fdf9f3] p-3">
          <p class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
            {$t('studioScales')}
          </p>
          {#if weigh}
            <p class="mt-2 font-serif text-lg text-[#34251c]" class:text-[#8f2f22]={overweight}>
              {weigh.totalPoints.toFixed(2)}
              <span class="text-sm text-[#8a6a55]">/ {weigh.tierBudget.toFixed(0)}</span>
            </p>
            <p class="text-[11px] text-[#8a6a55]">
              {$t('studioScalesBudget').replace('{n}', String(body.tier))}
            </p>
            {#if weigh.readiness.blocking.length}
              <ul class="mt-2 space-y-1">
                {#each weigh.readiness.blocking as fault (fault)}
                  <li class="text-[11px] leading-snug text-[#8f2f22]">{faultWord(fault)}</li>
                {/each}
              </ul>
            {:else}
              <p class="mt-2 text-[11px] text-[#4a6b45]">{$t('studioScalesFit')}</p>
            {/if}
          {:else}
            <p class="mt-2 text-[11px] text-[#8a6a55]">{$t('studioLoading')}</p>
          {/if}
        </div>

        <div class="mt-4 flex flex-wrap gap-2">
          <button
            onclick={save}
            disabled={saving || !dirty}
            class="border border-[#34251c]/25 px-4 py-2 text-xs uppercase tracking-[0.16em] hover:bg-[#34251c]/5 disabled:opacity-40"
            >{dirty ? $t('studioCardSave') : $t('studioCardSaved')}</button
          >
          <button
            onclick={show}
            disabled={saving || (weigh?.readiness.blocking.length ?? 1) > 0}
            class="border border-[#c65f3c]/40 px-4 py-2 text-xs uppercase tracking-[0.16em] text-[#c65f3c] hover:bg-[#c65f3c]/8 disabled:opacity-40"
            >{$t('studioCardShow')}</button
          >
        </div>
      </div>

      <!-- Содержимое. Домовых полей здесь нет вовсе: место для цены было бы
           обещанием, которого сервер не выполнит. -->
      <div class="space-y-6">
        <section class="space-y-3">
          <h2 class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
            {$t('studioCardWords')}
          </h2>
          <div class="grid gap-3 sm:grid-cols-2">
            <label class="block text-[11px] text-[#6f3b24]">
              {$t('studioCardTitleRu')}
              <input
                bind:value={body.titleRu}
                class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              />
            </label>
            <label class="block text-[11px] text-[#6f3b24]">
              {$t('studioCardTitleEn')}
              <input
                bind:value={body.titleEn}
                class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              />
            </label>
            <label class="block text-[11px] text-[#6f3b24]">
              {$t('studioCardEffectRu')}
              <textarea
                bind:value={body.effectRu}
                rows="2"
                class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              ></textarea>
            </label>
            <label class="block text-[11px] text-[#6f3b24]">
              {$t('studioCardEffectEn')}
              <textarea
                bind:value={body.effectEn}
                rows="2"
                class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              ></textarea>
            </label>
            <label class="block text-[11px] text-[#6f3b24]">
              {$t('studioCardLoreRu')}
              <textarea
                bind:value={body.loreRu}
                rows="2"
                class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              ></textarea>
            </label>
            <label class="block text-[11px] text-[#6f3b24]">
              {$t('studioCardLoreEn')}
              <textarea
                bind:value={body.loreEn}
                rows="2"
                class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              ></textarea>
            </label>
            <label class="block text-[11px] text-[#6f3b24]">
              {$t('studioCardTypeRu')}
              <input
                bind:value={body.typeRu}
                class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              />
            </label>
            <label class="block text-[11px] text-[#6f3b24]">
              {$t('studioCardTypeEn')}
              <input
                bind:value={body.typeEn}
                class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              />
            </label>
          </div>
        </section>

        <section class="space-y-3">
          <h2 class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
            {$t('studioCardKindAndRank')}
          </h2>
          <div class="grid gap-3 sm:grid-cols-3">
            <label class="block text-[11px] text-[#6f3b24]">
              {$t('studioCardRace')}
              <select
                bind:value={body.raceId}
                class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              >
                <option value={null}>—</option>
                {#each races as race (race.id)}
                  <option value={race.id}>{$lang === 'en' ? race.nameEn : race.nameRu}</option>
                {/each}
              </select>
            </label>
            <label class="block text-[11px] text-[#6f3b24]">
              {$t('studioCardKind')}
              <select
                bind:value={body.kind}
                class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              >
                <option value="unit">{$t('battlesKindUnit')}</option>
                <option value="spell">{$t('battlesKindSpell')}</option>
                <option value="relic">{$t('battlesKindRelic')}</option>
              </select>
            </label>
            <label class="block text-[11px] text-[#6f3b24]">
              <!-- Чин — БЮДЖЕТ, а не награда: взявший пятый получил не сильную
                   карту, а разрешение весить больше. -->
              {$t('studioCardTier')}
              <select
                bind:value={body.tier}
                class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              >
                {#each [1, 2, 3, 4, 5] as n (n)}
                  <option value={n}>{n}</option>
                {/each}
              </select>
            </label>
            <label class="block text-[11px] text-[#6f3b24]">
              {$t('studioCardChannel')}
              <select
                bind:value={body.attackChannel}
                class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              >
                <option value="physical">{$t('battlesChannelPhysical')}</option>
                <option value="magic">{$t('battlesChannelMagic')}</option>
                <option value="pure">{$t('battlesChannelPure')}</option>
                <option value="none">{$t('battlesChannelNone')}</option>
              </select>
            </label>
          </div>
        </section>

        <section class="space-y-3">
          <h2 class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
            {$t('studioCardNumbers')}
          </h2>
          <div class="grid grid-cols-3 gap-3 sm:grid-cols-5">
            {#each NUMBERS as num (num.key)}
              <label class="block text-[11px] text-[#6f3b24]">
                {$t(num.word as never)}
                <input
                  type="number"
                  value={body[num.key]}
                  oninput={(e) => {
                    (body as never as Record<string, number>)[num.key as string] = Number(
                      e.currentTarget.value,
                    );
                  }}
                  class="mt-1 w-full border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
                />
              </label>
            {/each}
          </div>
        </section>

        <section class="space-y-3">
          <h2 class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
            {$t('studioCardTraits')}
          </h2>
          <p class="text-[11px] italic text-[#8a6a55]">{$t('studioCardTraitsHint')}</p>
          {#each body.traits as trait, i (i)}
            <div class="grid gap-2 border border-[#d8c6b1] bg-[#fdf9f3] p-3 sm:grid-cols-2">
              <input
                bind:value={trait.nameRu}
                placeholder={$t('studioCardTraitNameRu')}
                class="border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              />
              <input
                bind:value={trait.nameEn}
                placeholder={$t('studioCardTraitNameEn')}
                class="border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              />
              <input
                bind:value={trait.textRu}
                placeholder={$t('studioCardTraitTextRu')}
                class="border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              />
              <input
                bind:value={trait.textEn}
                placeholder={$t('studioCardTraitTextEn')}
                class="border border-[#d8c6b1] bg-white px-2 py-1 text-sm text-[#34251c]"
              />
              <button
                onclick={() => dropTrait(i)}
                class="justify-self-start text-[10px] uppercase tracking-[0.14em] text-[#b0a08e] hover:text-[#8f2f22]"
                >{$t('studioCardTraitDrop')}</button
              >
            </div>
          {/each}
          <button
            onclick={addTrait}
            class="border border-[#34251c]/25 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
            >{$t('studioCardTraitAdd')}</button
          >
        </section>

        <section class="space-y-3">
          <h2 class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
            {$t('studioCardAbilities')}
          </h2>
          <!-- Тот же редактор, что у хозяина, — не похожий, а ТОТ ЖЕ. Человек
               изобретает СОЧЕТАНИЕ из семнадцати глаголов, которые движок уже
               умеет, а не новое правило: тысячи способностей и ни одной правки
               движка. Второй список глаголов на этой странице однажды разошёлся
               бы с домашним, и сервер отбросил бы лишнее молча. -->
          <p class="text-[11px] italic text-[#8a6a55]">{$t('studioCardAbilitiesHint')}</p>
          <BattleAbilities
            bind:abilities={body.abilities}
            pointsOf={(id) => weigh?.abilities.find((a) => a.id === id)?.points ?? null}
          />
        </section>

        <section class="space-y-3">
          <h2 class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
            {$t('studioCardArt')}
          </h2>
          <!-- Картинка берётся ИЗ ЯЩИКА, а не с верстака: на верстаке она живёт
               в браузере, и карта с такой картинкой у хозяина была бы пустой. -->
          {#if picking}
            {#if box.length === 0}
              <p class="text-[11px] text-[#8a6a55]">
                {$t('studioCardArtEmpty')}
                <a href="/studio/assets" class="underline">{$t('studioStore')}</a>
              </p>
            {:else}
              <div class="grid grid-cols-4 gap-2 sm:grid-cols-6">
                {#each box as asset (asset.id)}
                  <button
                    onclick={() => {
                      body!.artUrl = asset.url;
                      picking = false;
                    }}
                    class="border border-[#d8c6b1] bg-white p-1 hover:border-[#c65f3c]"
                  >
                    <img src={asset.url} alt={asset.name} class="h-16 w-full object-contain" />
                  </button>
                {/each}
              </div>
            {/if}
            <button
              onclick={() => (picking = false)}
              class="text-[10px] uppercase tracking-[0.14em] text-[#b0a08e] hover:text-[#8a6a55]"
              >{$t('studioCancel')}</button
            >
          {:else}
            <div class="flex flex-wrap items-center gap-3">
              <button
                onclick={() => (picking = true)}
                class="border border-[#34251c]/25 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
                >{$t('studioCardArtPick')}</button
              >
              {#if body.artUrl}
                <button
                  onclick={() => (body!.artUrl = null)}
                  class="text-[10px] uppercase tracking-[0.14em] text-[#b0a08e] hover:text-[#8f2f22]"
                  >{$t('studioCardArtClear')}</button
                >
              {/if}
            </div>
          {/if}
        </section>
      </div>
    </div>
  {/if}
</div>
