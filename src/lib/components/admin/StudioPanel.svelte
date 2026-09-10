<script lang="ts">
  // Стол студии: всё, что работы людей просят у хозяина.
  //
  // Три очереди, и они РАЗНЫЕ, а не один список с фильтром:
  //
  //   ДОПУСК — «этому можно на люди». Решается за секунды, до того как работу
  //   увидит кто-то кроме автора.
  //   ТРОЙКА — «этому быть в игре и в продаже». Решается после вердикта, и это
  //   другое решение, с тиражом и последствиями.
  //   КАРТЫ — работы людей, которые просятся на полку: своё решение, со
  //   своими последствиями (слуг, цена, тираж, автограф, экземпляр автору).
  //   ЖАЛОБЫ — то, что дом узнал не сам.
  //
  // Работа везде показана НАСТОЯЩЕЙ картой в её раме: решать по имени файла
  // нельзя, а второй облик однажды соврёт.
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import { cardFromRequest, emptyBattleCard, dressOf } from '$lib/battles';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import type {
    BattleCard as BattleCardDto,
    BattleFrame,
    BattleRace,
    StudioCardWaiting,
    StudioMotionWaiting,
    StudioRaceWaiting,
    StudioEntry,
    StudioFrame,
    StudioReport,
    StudioSeason,
    StudioSettings,
    StudioShown,
  } from '$lib/types/api';

  let admissions = $state<StudioFrame[]>([]);
  let queue = $state<StudioEntry[]>([]);
  let reports = $state<StudioReport[]>([]);
  /** Карты людей — ОДНА очередь. Допуск у карт снят разбором
   *  (`STUDIO-CARDS-REVIEW.md` П3): он не значил ничего, кроме того, в каком
   *  ящике стола лежит работа, а утвердить можно было и мимо него. Решений у
   *  хозяина два, и оба с последствиями: взять или вернуть со словом. */
  let cards = $state<StudioCardWaiting[]>([]);
  let frames = $state<BattleFrame[] | null>(null);
  /** Роды людей: словарные строки, которые наденут чужие карты. Взвешивать в
   *  них нечего — решение глазами, и потому очередь простая. */
  let waitingRaces = $state<StudioRaceWaiting[]>([]);
  /** Движения людей: те же два решения, что у карт и родов, — взять или
   *  вернуть со словом. Взвешивать в них нечего: сочетание готовых жестов
   *  либо складывается в такт, либо нет, и смотрит на это хозяин. */
  let motions = $state<StudioMotionWaiting[]>([]);
  let racing = $state<string | null>(null);
  let raceSlug = $state('');
  let races = $state<BattleRace[]>([]);
  /** Дописка хозяина к утверждаемой карте. Держится ОДНА: утверждают по одной,
   *  и десять открытых форм — это десять способов ошибиться строкой. */
  let dressing = $state<string | null>(null);
  let slug = $state('');
  let priceDust = $state<number | null>(20);
  let cardEdition = $state<number | null>(null);
  /** Утверждённые рамки людей — чтобы надеть чужую работу на карту сразу
   *  здесь, а не бегать за этим в панель битв. */
  let approvedFrames = $state<StudioShown[]>([]);
  let chosenFrameId = $state('');
  let season = $state<StudioSeason | null>(null);
  let settings = $state<StudioSettings | null>(null);
  let loading = $state(true);
  let busy = $state(false);
  let said = $state<string | null>(null);
  let theme = $state('');
  let themeNote = $state('');
  let edition = $state(200);

  /** Манекен: карта дома, на которой видно раму. Одна на весь стол — работы
   *  отличаются рамой, и разные карты сравнивали бы не то. */
  const sample: BattleCardDto = {
    ...emptyBattleCard(),
    tier: 1,
    titleEn: 'The Keeper of the Key',
    titleRu: 'Хранительница Ключа',
    effectRu: 'Вихрь Души: каждое третье заклинание создаёт копию эффекта.',
    cost: 5,
    power: 10,
  };

  onMount(reload);

  async function reload() {
    loading = true;
    try {
      const [a, q, r, page, s, cn, rq, mo, dress, race, af] = await Promise.all([
        api.adminStudioAdmissions().catch(() => []),
        api.adminStudioQueue().catch(() => []),
        api.adminStudioReports().catch(() => []),
        api.getStudioSeason(null).catch(() => null),
        api.adminGetStudioSettings().catch(() => null),
        api.adminStudioCardQueue().catch(() => []),
        api.adminStudioRaceQueue().catch(() => []),
        api.adminStudioMotionQueue().catch(() => []),
        api.getBattleFrames().catch(() => null),
        api.getBattleRaces().catch(() => []),
        api.adminApprovedStudioFrames().catch(() => []),
      ]);
      admissions = a;
      queue = q;
      reports = r;
      cards = cn;
      waitingRaces = rq;
      motions = mo;
      frames = dress?.frames ?? null;
      races = race;
      approvedFrames = af;
      season = page?.season ?? null;
      settings = s;
      theme = season?.theme ?? '';
      themeNote = season?.themeNote ?? '';
    } finally {
      loading = false;
    }
  }

  function flash(text: string, ms = 5000) {
    said = text;
    setTimeout(() => (said = null), ms);
  }

  async function run(work: () => Promise<unknown>, done?: string) {
    busy = true;
    try {
      await work();
      await reload();
      if (done) flash(done);
    } catch (e) {
      flash(String(e), 8000);
    } finally {
      busy = false;
    }
  }

  /** Не допустить — только со словом: отказ, которого нельзя исправить,
   *  вернётся той же работой на следующей неделе. */
  function deny(frame: StudioFrame) {
    const word = prompt($t('adminStudioDenyAsk').replace('{name}', frame.name));
    if (!word?.trim()) return;
    run(() => api.adminDenyStudioFrame(frame.id, word), $t('adminStudioDenied'));
  }

  /** Имя работы берётся ИЗ ТЕЛА: второго места, где написано одно имя, у
   *  карты нет. */
  const cardName = (work: StudioCardWaiting) =>
    (work.body.titleRu || work.body.titleEn || '').trim();

  function denyCard(work: StudioCardWaiting) {
    const word = prompt($t('adminStudioDenyAsk').replace('{name}', cardName(work)));
    if (!word?.trim()) return;
    run(() => api.adminDenyStudioCard(work.id, word), $t('adminStudioDenied'));
  }

  /** Утвердить. Слуг обязателен — по нему на карту ссылаются испытания, и
   *  придумать его за хозяина нельзя. */
  function approveCard(work: StudioCardWaiting) {
    if (!slug.trim()) {
      flash($t('adminStudioCardNeedsSlug'));
      return;
    }
    const wornFrame = approvedFrames.find((f) => f.id === chosenFrameId);
    run(
      () =>
        api.adminApproveStudioCard(work.id, {
          slug: slug.trim(),
          priceDust,
          editionSize: cardEdition,
          status: 'published',
          frameOverride: wornFrame ? JSON.stringify(dressOf(wornFrame.body)) : null,
        }),
      $t('adminStudioCardTaken'),
    );
    dressing = null;
    slug = '';
    chosenFrameId = '';
  }

  function denyRace(work: StudioRaceWaiting) {
    const word = prompt($t('adminStudioDenyAsk').replace('{name}', work.nameRu));
    if (!word?.trim()) return;
    run(() => api.adminDenyStudioRace(work.id, word), $t('adminStudioDenied'));
  }

  /** Взять род в словарь. Слуг обязателен: он уникален и по нему словарь
   *  находят. */
  function approveRace(work: StudioRaceWaiting) {
    if (!raceSlug.trim()) {
      flash($t('adminStudioCardNeedsSlug'));
      return;
    }
    run(
      () => api.adminApproveStudioRace(work.id, raceSlug.trim()),
      $t('adminStudioRaceTaken'),
    );
    racing = null;
    raceSlug = '';
  }

  function denyMotion(one: StudioMotionWaiting) {
    const word = prompt(
      $t('adminStudioDenyAsk').replace('{name}', one.body.nameRu || one.body.id),
    );
    if (!word?.trim()) return;
    run(() => api.adminDenyStudioMotion(one.id, word), $t('adminStudioDenied'));
  }

  function strike(id: string, name: string) {
    const word = prompt($t('adminStudioStrikeAsk').replace('{name}', name));
    if (!word?.trim()) return;
    run(() => api.adminStrikeStudioFrame(id, word), $t('adminStudioStruck'));
  }
</script>

<div class="h-full overflow-y-auto p-5">
  <div class="flex flex-wrap items-baseline justify-between gap-3">
    <h2 class="font-serif text-lg text-[#34251c]">{$t('adminStudioTitle')}</h2>
    <button
      onclick={reload}
      class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] hover:text-[#c65f3c]"
      >{$t('adminStudioRefresh')}</button
    >
  </div>

  {#if said}<p class="mt-2 text-xs text-[#c65f3c]">{said}</p>{/if}

  {#if loading}
    <p class="mt-6 text-sm text-[#8a6a55]">{$t('studioLoading')}</p>
  {:else}
    <!-- ── Неделя ────────────────────────────────────────────────────────── -->
    <section class="mt-6 border border-[#d8c6b1] bg-[#fdf9f3] p-4">
      <h3 class="text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
        {$t('adminStudioWeek')}
      </h3>
      {#if season}
        <p class="mt-1 text-xs text-[#8a6a55]">
          №{season.number} · {season.state} · {new Date(season.opensAt).toLocaleDateString()} —
          {new Date(season.closesAt).toLocaleDateString()}
        </p>
      {/if}
      <!-- Тема — приглашение, а не проверка: работа мимо темы участвует
           наравне, и отсеять её может только допуск, словами. -->
      <div class="mt-3 flex flex-wrap items-end gap-2">
        <label class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
          {$t('adminStudioTheme')}
          <input
            bind:value={theme}
            maxlength="120"
            class="ml-2 w-56 border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs outline-none focus:border-[#34251c]/35"
          />
        </label>
        <label class="flex-1 text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
          {$t('adminStudioThemeNote')}
          <input
            bind:value={themeNote}
            maxlength="400"
            class="ml-2 w-full max-w-md border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs outline-none focus:border-[#34251c]/35"
          />
        </label>
        <button
          onclick={() => run(() => api.adminSaveStudioSeason(theme, themeNote), $t('adminStudioThemeSaved'))}
          disabled={busy}
          class="border border-[#34251c]/25 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] hover:bg-[#34251c]/5 disabled:opacity-40"
          >{$t('adminStudioSave')}</button
        >
        <!-- Подвести руками — та же функция, что у фоновой задачи, и она
             идемпотентна: подведённый сезон второй раз не считается.
             Именно поэтому здесь спрашивают: подведённая неделя КОНЧЕНА, и
             никто больше не выставится и не оценит до понедельника. Кнопка без
             вопроса однажды закроет неделю случайно. -->
        <button
          onclick={() =>
            confirm($t('adminStudioJudgeSure')) &&
            run(async () => {
              const r = await api.adminJudgeStudioSeason();
              flash($t('adminStudioJudged').replace('{n}', String(r.toKeeper)));
            })}
          disabled={busy}
          class="border border-[#34251c]/20 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] hover:bg-[#34251c]/5 disabled:opacity-40"
          >{$t('adminStudioJudge')}</button
        >
      </div>
    </section>

    <!-- ── Допуск ───────────────────────────────────────────────────────── -->
    <section class="mt-8">
      <h3 class="text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
        {$t('adminStudioAdmissions')}
        {#if admissions.length}<span class="ml-2 text-[#c65f3c]">{admissions.length}</span>{/if}
      </h3>
      <p class="mt-1 max-w-2xl text-[11px] text-[#8a6a55]">{$t('adminStudioAdmissionsLead')}</p>
      {#if admissions.length === 0}
        <p class="mt-3 text-sm text-[#8a6a55]">{$t('adminStudioNothing')}</p>
      {:else}
        <div class="mt-3 grid grid-cols-2 gap-5 lg:grid-cols-4">
          {#each admissions as frame (frame.id)}
            <div>
              <div style="max-width: 200px">
                <BattleCard card={sample} frames={[frame.body]} owned={true} />
              </div>
              <p class="mt-1 max-w-[200px] truncate text-xs text-[#34251c]">{frame.name}</p>
              <div class="mt-1 flex gap-3">
                <button
                  onclick={() => run(() => api.adminAdmitStudioFrame(frame.id), $t('adminStudioAdmitted'))}
                  disabled={busy}
                  class="text-[10px] uppercase tracking-[0.14em] text-[#c65f3c] hover:underline disabled:opacity-40"
                  >{$t('adminStudioAdmit')}</button
                >
                <button
                  onclick={() => deny(frame)}
                  disabled={busy}
                  class="text-[10px] uppercase tracking-[0.14em] text-[#8f2f22]/70 hover:text-[#8f2f22] disabled:opacity-40"
                  >{$t('adminStudioDeny')}</button
                >
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <!-- ── Тройка ───────────────────────────────────────────────────────── -->
    <section class="mt-8">
      <h3 class="text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
        {$t('adminStudioQueue')}
        {#if queue.length}<span class="ml-2 text-[#c65f3c]">{queue.length}</span>{/if}
      </h3>
      <p class="mt-1 max-w-2xl text-[11px] text-[#8a6a55]">{$t('adminStudioQueueLead')}</p>
      {#if queue.length === 0}
        <p class="mt-3 text-sm text-[#8a6a55]">{$t('adminStudioNothing')}</p>
      {:else}
        <div class="mt-3 flex flex-wrap items-end gap-3">
          <label class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
            {$t('adminStudioEdition')}
            <input
              type="number"
              bind:value={edition}
              min="1"
              class="ml-2 w-20 border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs"
            />
          </label>
        </div>
        <div class="mt-3 grid grid-cols-2 gap-5 lg:grid-cols-4">
          {#each queue as entry (entry.id)}
            <div>
              <div style="max-width: 200px">
                <BattleCard card={sample} frames={[entry.body]} owned={true} />
              </div>
              <p class="mt-1 max-w-[200px] truncate text-xs text-[#34251c]">{entry.name}</p>
              <p class="text-[10px] text-[#8a6a55]">
                {entry.author} · {$t('studioPlace').replace('{n}', String(entry.place ?? 0))} ·
                {entry.score?.toFixed(2) ?? '—'} ({entry.votes})
              </p>
              <div class="mt-1 flex gap-3">
                <button
                  onclick={() =>
                    run(
                      () => api.adminApproveStudioFrame(entry.frameId, edition),
                      $t('adminStudioApproved'),
                    )}
                  disabled={busy}
                  class="text-[10px] uppercase tracking-[0.14em] text-[#c65f3c] hover:underline disabled:opacity-40"
                  >{$t('adminStudioApprove')}</button
                >
                <button
                  onclick={() => strike(entry.frameId, entry.name)}
                  disabled={busy}
                  class="text-[10px] uppercase tracking-[0.14em] text-[#8f2f22]/70 hover:text-[#8f2f22] disabled:opacity-40"
                  >{$t('adminStudioStrike')}</button
                >
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <!-- ── Карты людей ──────────────────────────────────────────────────── -->
    <section class="mt-8">
      <h3 class="text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
        {$t('adminStudioCards')}
        {#if cards.length}<span class="ml-2 text-[#c65f3c]">{cards.length}</span>{/if}
      </h3>
      <p class="mt-1 max-w-2xl text-[11px] text-[#8a6a55]">{$t('adminStudioCardsLead')}</p>

      {#if cards.length === 0}
        <p class="mt-3 text-sm text-[#8a6a55]">{$t('adminStudioNothing')}</p>
      {:else}
        <div class="mt-3 grid grid-cols-2 gap-5 lg:grid-cols-4">
          {#each cards as work (work.id)}
            <div>
              <!-- Настоящая карта, а не строка списка: решать по имени файла
                   нельзя, а числа видны только на лице. -->
              <div style="max-width: 220px">
                <BattleCard card={cardFromRequest(work.body, races)} {frames} owned={true} />
              </div>
              <p class="mt-1 max-w-[220px] truncate text-xs text-[#34251c]">{cardName(work)}</p>
              <p class="text-[10px] text-[#8a6a55]">{work.author}</p>
              {#if dressing === work.id}
                <!-- Дописка: ТОЛЬКО домовое. Содержимое пришло от человека и
                     здесь не правится — иначе на полку встанет не то, что он
                     принёс. -->
                <div class="mt-2 max-w-[220px] space-y-2 border border-[#d8c6b1] p-2">
                  <label class="block text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
                    {$t('adminStudioCardSlug')}
                    <input
                      bind:value={slug}
                      class="mt-1 w-full border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs"
                    />
                  </label>
                  <label class="block text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
                    {$t('adminStudioCardPrice')}
                    <input
                      type="number"
                      bind:value={priceDust}
                      class="mt-1 w-full border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs"
                    />
                  </label>
                  <label class="block text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
                    {$t('adminStudioCardEdition')}
                    <input
                      type="number"
                      bind:value={cardEdition}
                      class="mt-1 w-full border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs"
                    />
                  </label>
                  <label class="block text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
                    {$t('adminStudioCardFrame')}
                    <select
                      bind:value={chosenFrameId}
                      class="mt-1 w-full border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs"
                    >
                      <option value="">{$t('adminStudioCardFrameNone')}</option>
                      {#each approvedFrames as one (one.id)}
                        <option value={one.id}>{one.name} — {one.author}</option>
                      {/each}
                    </select>
                  </label>
                  <div class="flex gap-3">
                    <button
                      onclick={() => approveCard(work)}
                      disabled={busy}
                      class="text-[10px] uppercase tracking-[0.14em] text-[#c65f3c] hover:underline disabled:opacity-40"
                      >{$t('adminStudioApprove')}</button
                    >
                    <button
                      onclick={() => (dressing = null)}
                      class="text-[10px] uppercase tracking-[0.14em] text-[#b0a08e] hover:text-[#8a6a55]"
                      >{$t('adminStudioCardLater')}</button
                    >
                  </div>
                </div>
              {:else}
                <div class="mt-1 flex gap-3">
                  <button
                    onclick={() => {
                      dressing = work.id;
                      slug = '';
                      priceDust = 20;
                      cardEdition = null;
                      chosenFrameId = '';
                    }}
                    disabled={busy}
                    class="text-[10px] uppercase tracking-[0.14em] text-[#c65f3c] hover:underline disabled:opacity-40"
                    >{$t('adminStudioCardDress')}</button
                  >
                  <button
                    onclick={() => denyCard(work)}
                    disabled={busy}
                    class="text-[10px] uppercase tracking-[0.14em] text-[#8f2f22]/70 hover:text-[#8f2f22] disabled:opacity-40"
                    >{$t('adminStudioDeny')}</button
                  >
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <!-- ── Роды людей ───────────────────────────────────────────────────── -->
    <section class="mt-8">
      <h3 class="text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
        {$t('adminStudioRaces')}
        {#if waitingRaces.length}<span class="ml-2 text-[#c65f3c]">{waitingRaces.length}</span>{/if}
      </h3>
      <p class="mt-1 max-w-2xl text-[11px] text-[#8a6a55]">{$t('adminStudioRacesLead')}</p>
      {#if waitingRaces.length === 0}
        <p class="mt-3 text-sm text-[#8a6a55]">{$t('adminStudioNothing')}</p>
      {:else}
        <div class="mt-3 space-y-3">
          {#each waitingRaces as work (work.id)}
            <div class="flex flex-wrap items-start gap-3 border border-[#d8c6b1] p-3">
              {#if work.iconUrl}
                <img src={work.iconUrl} alt="" class="h-10 w-10 object-contain" />
              {/if}
              <div class="min-w-0 flex-1">
                <p class="text-sm text-[#34251c]">
                  {work.nameRu} <span class="text-[#b0a08e]">· {work.nameEn}</span>
                </p>
                <p class="text-[10px] text-[#8a6a55]">{work.author}</p>
                {#if work.noteRu}
                  <p class="mt-1 text-[11px] italic text-[#6f3b24]">{work.noteRu}</p>
                {/if}
              </div>
              {#if racing === work.id}
                <div class="flex flex-wrap items-end gap-2">
                  <label class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
                    {$t('adminStudioCardSlug')}
                    <input
                      bind:value={raceSlug}
                      class="ml-2 w-32 border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs"
                    />
                  </label>
                  <button
                    onclick={() => approveRace(work)}
                    disabled={busy}
                    class="text-[10px] uppercase tracking-[0.14em] text-[#c65f3c] hover:underline disabled:opacity-40"
                    >{$t('adminStudioApprove')}</button
                  >
                  <button
                    onclick={() => (racing = null)}
                    class="text-[10px] uppercase tracking-[0.14em] text-[#b0a08e] hover:text-[#8a6a55]"
                    >{$t('adminStudioCardLater')}</button
                  >
                </div>
              {:else}
                <div class="flex gap-3">
                  <button
                    onclick={() => {
                      racing = work.id;
                      raceSlug = '';
                    }}
                    disabled={busy}
                    class="text-[10px] uppercase tracking-[0.14em] text-[#c65f3c] hover:underline disabled:opacity-40"
                    >{$t('adminStudioRaceTake')}</button
                  >
                  <button
                    onclick={() => denyRace(work)}
                    disabled={busy}
                    class="text-[10px] uppercase tracking-[0.14em] text-[#8f2f22]/70 hover:text-[#8f2f22] disabled:opacity-40"
                    >{$t('adminStudioDeny')}</button
                  >
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <!-- ── Движения людей ───────────────────────────────────────────────── -->
    <section class="mt-8">
      <h3 class="text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
        {$t('adminStudioMotions')}
        {#if motions.length}<span class="ml-2 text-[#c65f3c]">{motions.length}</span>{/if}
      </h3>
      <p class="mt-1 max-w-2xl text-[11px] text-[#8a6a55]">{$t('adminStudioMotionsLead')}</p>
      {#if motions.length === 0}
        <p class="mt-3 text-sm text-[#8a6a55]">{$t('adminStudioNothing')}</p>
      {:else}
        <ul class="mt-3 space-y-2">
          {#each motions as one (one.id)}
            <li
              class="flex flex-wrap items-baseline gap-3 border border-[#d8c6b1] bg-[#fdf9f3] p-3"
            >
              <span class="text-sm text-[#34251c]">{one.body.nameRu || one.body.id}</span>
              <span class="text-[11px] text-[#8a6a55]">
                {one.author} · {$t('adminStudioMotionGestures').replace(
                  '{n}',
                  String(one.body.gestures?.length ?? 0),
                )}
              </span>
              <button
                onclick={() =>
                  run(() => api.adminApproveStudioMotion(one.id), $t('adminStudioApproved'))}
                disabled={busy}
                class="ml-auto text-[10px] uppercase tracking-[0.14em] text-[#c65f3c] hover:underline disabled:opacity-40"
                >{$t('adminStudioApprove')}</button
              >
              <button
                onclick={() => denyMotion(one)}
                disabled={busy}
                class="text-[10px] uppercase tracking-[0.14em] text-[#8f2f22]/70 hover:text-[#8f2f22] disabled:opacity-40"
                >{$t('adminStudioDeny')}</button
              >
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <!-- ── Жалобы ───────────────────────────────────────────────────────── -->
    <section class="mt-8">
      <h3 class="text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
        {$t('adminStudioReports')}
        {#if reports.length}<span class="ml-2 text-[#c65f3c]">{reports.length}</span>{/if}
      </h3>
      {#if reports.length === 0}
        <p class="mt-3 text-sm text-[#8a6a55]">{$t('adminStudioNothing')}</p>
      {:else}
        <ul class="mt-3 divide-y divide-[#d8c6b1]/60 border-y border-[#d8c6b1]/60">
          {#each reports as one (one.id)}
            <li class="flex flex-wrap items-center gap-3 py-3">
              <div style="width: 90px">
                <BattleCard card={sample} frames={[one.body]} owned={true} />
              </div>
              <div class="min-w-0 flex-1">
                <p class="text-sm text-[#34251c]">{one.frameName}</p>
                <p class="text-[11px] text-[#8a6a55]">
                  {one.reason}{#if one.note} — {one.note}{/if}
                </p>
                <p class="text-[10px] text-[#b0a08e]">
                  {one.reporter ?? $t('adminStudioAnon')} ·
                  {new Date(one.createdAt).toLocaleDateString()}
                </p>
              </div>
              <div class="flex gap-3">
                <button
                  onclick={() => strike(one.frameId, one.frameName)}
                  disabled={busy}
                  class="text-[10px] uppercase tracking-[0.14em] text-[#8f2f22]/70 hover:text-[#8f2f22] disabled:opacity-40"
                  >{$t('adminStudioStrike')}</button
                >
                <button
                  onclick={() => run(() => api.adminCloseStudioReport(one.id))}
                  disabled={busy}
                  class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55] hover:text-[#c65f3c] disabled:opacity-40"
                  >{$t('adminStudioReportClose')}</button
                >
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <!-- ── Числа комнаты ────────────────────────────────────────────────── -->
    {#if settings}
      <section class="mt-8 border-t border-[#d8c6b1] pt-4">
        <h3 class="text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
          {$t('adminStudioSettings')}
        </h3>
        <div class="mt-3 flex flex-wrap items-end gap-4">
          <label class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
            {$t('adminStudioGate')}
            <select
              bind:value={settings.gate}
              class="ml-2 border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs"
            >
              <option value="all">{$t('adminStudioGateAll')}</option>
              <option value="owners">{$t('adminStudioGateOwners')}</option>
              <option value="closed">{$t('adminStudioGateClosed')}</option>
            </select>
          </label>
          <label class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
            {$t('adminStudioBox')}
            <input
              type="number"
              value={Math.round(settings.boxBytes / 1024 / 1024)}
              onchange={(e) =>
                settings && (settings.boxBytes = Number(e.currentTarget.value) * 1024 * 1024)}
              min="1"
              class="ml-2 w-20 border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs"
            />
          </label>
          <label class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
            {$t('adminStudioSheetsAtOnce')}
            <input
              type="number"
              bind:value={settings.sheetsAtOnce}
              min="1"
              class="ml-2 w-16 border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs"
            />
          </label>
          <label class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
            {$t('adminStudioSheetsPerDay')}
            <input
              type="number"
              bind:value={settings.sheetsPerDay}
              min="0"
              class="ml-2 w-16 border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs"
            />
          </label>
          <!-- Пороги сезона. Смягчать их на первых неделях — работа хозяина, а
               не программиста: при десятке участников порог живого дома не
               пропустит никого, и недели пройдут впустую. -->
          <label class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
            {$t('adminStudioRatingsPerSeason')}
            <input
              type="number"
              bind:value={settings.ratingsPerSeason}
              min="1"
              class="ml-2 w-16 border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs"
            />
          </label>
          <label class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
            {$t('adminStudioRatingsTrusted')}
            <input
              type="number"
              bind:value={settings.ratingsTrusted}
              min="1"
              class="ml-2 w-16 border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs"
            />
          </label>
          <label class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
            {$t('adminStudioToKeeper')}
            <input
              type="number"
              bind:value={settings.toKeeper}
              min="1"
              class="ml-2 w-16 border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs"
            />
          </label>
          <button
            onclick={() =>
              settings &&
              run(() => api.adminSaveStudioSettings(settings!), $t('adminStudioSettingsSaved'))}
            disabled={busy}
            class="border border-[#34251c]/25 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] hover:bg-[#34251c]/5 disabled:opacity-40"
            >{$t('adminStudioSave')}</button
          >
        </div>
      </section>
    {/if}
  {/if}
</div>
