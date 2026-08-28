<script lang="ts">
  // Этюды хранителя.
  //
  // Расстановка составлена рукой — обе стороны сразу, как шахматный этюд. Гость
  // играет за свою половину против хранителя. Переигрывать можно сколько угодно:
  // пыль даётся за этюд и однажды, иначе самый лёгкий стал бы мельницей.
  //
  // Ни одного правила боя на этой странице нет. Сервер присылает состояние и
  // список законных действий, страница показывает первое и отправляет обратно
  // одно из вторых.
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t, lang } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import BattleScene from '$lib/components/BattleScene.svelte';
  import type {
    BattleAction,
    BattleCard,
    BattleChallenge,
    BattleFrame,
    BattleMatch,
  } from '$lib/types/api';

  let challenges = $state<BattleChallenge[]>([]);
  let cards = $state<BattleCard[]>([]);
  let frames = $state<BattleFrame[]>([]);
  let match = $state<BattleMatch | null>(null);
  let busy = $state(false);
  let complaint = $state<string | null>(null);
  /** Какой этюд играется — чтобы «начать этот заново» знал, какой «этот». */
  let taken = $state<BattleChallenge | null>(null);

  let signedIn = $derived(authStore.isLoggedIn);

  const titleOf = (c: BattleChallenge) => ($lang === 'ru' ? c.titleRu : c.titleEn);
  const noteOf = (c: BattleChallenge) => ($lang === 'ru' ? c.noteRu : c.noteEn);

  onMount(async () => {
    const [got, deck, dressing] = await Promise.all([
      api.getBattleChallenges(authStore.token),
      api.getBattleCards(),
      api.getBattleFrames(),
    ]);
    challenges = got;
    cards = deck;
    frames = dressing.frames;
  });

  async function takeUp(challenge: BattleChallenge) {
    const token = authStore.token;
    if (!token) return;
    busy = true;
    complaint = null;
    taken = challenge;
    try {
      match = await api.beginBattleMatch(token, challenge.id);
    } catch {
      complaint = $t('battleActionLost');
    } finally {
      busy = false;
    }
  }

  /** Тот же этюд с начала. Пыль за него уже отдана однажды — играют ради партии. */
  function again() {
    if (taken) void takeUp(taken);
  }

  async function play(action: BattleAction) {
    const token = authStore.token;
    if (!token || !match) return;
    busy = true;
    complaint = null;
    const sent = match.seq;
    try {
      // `seq` — то, что делает двойной щелчок безвредным: сервер отвечает на
      // повтор той же доской, а не играет ход дважды.
      const next = await api.actInBattleMatch(token, match.id, sent, action);
      // Тот же номер назад — значит, ход не был применён: партия ушла вперёд
      // из другой вкладки. Доска перерисовывается тем, что прислал сервер, и
      // кнопки «обновить» здесь нет: она уже обновлена.
      if (next.seq === sent && !next.outcome) complaint = $t('battleMovedOn');
      match = next;
      if (next.outcome) challenges = await api.getBattleChallenges(token);
    } catch {
      // Позиция не меняется, выбор не снимается: повторить можно тем же
      // касанием — `seq` делает повтор безвредным.
      complaint = $t('battleActionLost');
    } finally {
      busy = false;
    }
  }

  function putBack() {
    match = null;
    taken = null;
    complaint = null;
  }
</script>

<svelte:head>
  <title>{$t('battleStudies')}</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<!-- Партия просит трёх колонок (§9); полка этюдов — узкой страницы. -->
<div class="{match ? 'max-w-6xl' : 'max-w-4xl'} mx-auto px-5 py-12">
  <h1 class="mb-2 text-3xl" style="font-family: 'Cormorant Garamond', Georgia, serif;">
    {$t('battleStudies')}
  </h1>
  <p class="mb-8 max-w-[62ch] text-sm leading-relaxed text-[#5f4636]">{$t('battleStudiesRule')}</p>

  {#if complaint}
    <p class="mb-5 text-sm text-[#8f2f22]" transition:fade={{ duration: 150 }}>{complaint}</p>
  {/if}

  {#if match}
    <div class="mb-6">
      <button
        onclick={putBack}
        class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] hover:text-[#c65f3c]"
      >← {$t('battleLeave')}</button>
    </div>

    <BattleScene
      {match}
      {cards}
      {frames}
      {busy}
      onact={play}
      onleave={putBack}
      onreplay={taken ? again : undefined}
    />

  {:else}
    {#if !signedIn}
      <p class="mb-6 text-sm italic text-[#5f4636]">{$t('battleStudySignIn')}</p>
    {/if}

    {#if !challenges.length}
      <p class="text-sm italic text-[#5f4636]">{$t('battleStudiesEmpty')}</p>
    {:else}
      <ul class="border-t border-[#34251c]/10">
        {#each challenges as challenge (challenge.id)}
          <li class="flex items-baseline gap-4 py-3 border-b border-[#34251c]/10">
            <div class="flex-1 min-w-0">
              <span class="text-base" style="font-family: 'Cormorant Garamond', Georgia, serif;">
                {titleOf(challenge)}
              </span>
              {#if noteOf(challenge)}
                <span class="block mt-0.5 text-xs leading-snug text-[#8a6a55]">{noteOf(challenge)}</span>
              {/if}
            </div>
            {#if challenge.rewardDust > 0}
              <span class="text-[11px] tabular-nums text-[#8a6a55] whitespace-nowrap">
                {#if challenge.alreadyPaid}
                  {$t('battleStudyPaid')}
                {:else}
                  {challenge.rewardDust} {$t('battleStudyReward')}
                {/if}
              </span>
            {/if}
            <button
              disabled={!signedIn || busy}
              onclick={() => takeUp(challenge)}
              class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5 disabled:opacity-40 whitespace-nowrap"
            >{$t('battleStudyPlay')}</button>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</div>
