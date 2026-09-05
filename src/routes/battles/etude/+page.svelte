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
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { fade } from 'svelte/transition';
  import { t, lang, brandName } from '$lib/i18n';
  import { api } from '$lib/api';
  import { HOUSE_RULES, rulesApart } from '$lib/battles';
  import { authStore } from '$lib/stores/auth.svelte';
  import BattleScene from '$lib/components/BattleScene.svelte';
  import BattleDoor from '$lib/components/BattleDoor.svelte';
  import { matchChrome } from '$lib/stores/match-chrome.svelte';
  import type {
    BattleAction,
    BattleBodyCard,
    BattleCard,
    BattleChallenge,
    BattleFrame,
    BattleMatch,
    BattleUnit,
    Foresight,
    Motion,
  } from '$lib/types/api';

  let challenges = $state<BattleChallenge[]>([]);
  let cards = $state<BattleCard[]>([]);
  let frames = $state<BattleFrame[]>([]);
  /** Свод движений. Пустой — комната играет умолчания дома. */
  let motions = $state<Motion[]>([]);
  let match = $state<BattleMatch | null>(null);
  let busy = $state(false);
  let complaint = $state<string | null>(null);
  /** Какой этюд играется — чтобы «начать этот заново» знал, какой «этот». */
  let taken = $state<BattleChallenge | null>(null);
  /**
   * Разложен ли стол гостя. Нужно только ради одного: встреча, на которую
   * человек идёт с неразложенным столом, ведёт НА СТОЛ, а не в партию и не в
   * ошибку — он должен увидеть, что он приводит. Дальше «Взяться» ведёт прямо
   * в партию.
   */
  let laid = $state(false);
  /** onMount finished — `?play=` must not fire while `laid` is still the default. */
  let ready = $state(false);
  let openedPlay = false;
  /** `?chamber=1` already dressed a preview — leaving must not dress it again. */
  let openedChamber = false;
  /** Спросить, оставить открытую партию или отдать поле. */
  let leaving = $state(false);

  let signedIn = $derived(authStore.isLoggedIn);

  const titleOf = (c: BattleChallenge) => ($lang === 'ru' ? c.titleRu : c.titleEn);
  const noteOf = (c: BattleChallenge) => ($lang === 'ru' ? c.noteRu : c.noteEn);
  /** Чем правила этого боя отличаются от домашних. Пусто — обычный бой, и
   *  говорить нечего. */
  const apart = (c: BattleChallenge) => rulesApart(c.setup.rules);

  function loginFrom(challenge?: BattleChallenge) {
    const dest = challenge ? `/battles/etude?play=${challenge.id}` : '/battles/etude';
    return `/login?from=${encodeURIComponent(dest)}`;
  }

  onMount(async () => {
    const [got, deck, dressing, moving] = await Promise.all([
      api.getBattleChallenges(authStore.token),
      api.getBattleCards(),
      api.getBattleFrames(),
      api.getBattleMotions(),
    ]);
    challenges = got;
    cards = deck;
    frames = dressing.frames;
    motions = moving.motions;
    const token = authStore.token;
    if (token) {
      // Стол читается отдельно и необязательно: полка этюдов не должна пропасть
      // оттого, что стол не ответил.
      try {
        laid = (await api.getBattleDeck(token)).laid;
      } catch {
        laid = false;
      }
    }
    ready = true;
  });

  $effect(() => {
    if (match) matchChrome.cover();
    else matchChrome.uncover();
  });

  onDestroy(() => matchChrome.uncover());

  $effect(() => {
    const id = page.url.searchParams.get('play');
    if (!ready || openedPlay || !id || !signedIn || busy || match) return;
    const found = challenges.find((c) => c.id === id);
    if (!found) return;
    openedPlay = true;
    void takeUp(found);
  });

  /** Живая комната без входа: только в dev, `?chamber=1`. Расстановка — с этюда. */
  $effect(() => {
    if (!import.meta.env.DEV) return;
    if (!ready || match || busy) return;
    if (page.url.searchParams.get('chamber') !== '1') return;
    if (openedChamber) return;
    openedChamber = true;
    const study =
      challenges.find((c) => c.playerSide === 'scripted') ?? challenges[0] ?? null;
    if (!study) return;
    taken = study;
    match = dressStudy(study, cards, page.url.searchParams.get('seal') === '1');
  });

  function bodyOf(card: BattleCard): BattleBodyCard {
    return {
      name: card.slug,
      cost: card.cost,
      health: card.health,
      power: card.power,
      armor: card.armor,
      ward: card.ward,
      reach: card.reach,
      step: card.step,
      mend: card.mend,
      channel: card.attackChannel,
    };
  }

  function dressStudy(study: BattleChallenge, deck: BattleCard[], sealed = false): BattleMatch {
    const find = (slug: string) => deck.find((c) => c.slug === slug);
    const units: BattleUnit[] = [];
    const board: { cell: { x: number; y: number }; unit: number }[] = [];
    let next = 0;
    let playerId = -1;
    let keeperId = -1;
    const put = (
      slug: string,
      x: number,
      y: number,
      owner: 'player' | 'keeper',
    ) => {
      const card = find(slug);
      if (!card) return;
      const id = next++;
      const body = bodyOf(card);
      units[id] = {
        id,
        owner,
        reach: body.reach,
        step: body.step,
        mend: body.mend,
        channel: body.channel,
        acted: false,
        moved: false,
        retaliated: false,
        card: body,
        health: { current: body.health, max: body.health },
        power: body.power,
        armor: body.armor,
        ward: body.ward,
        shield: 0,
        statuses: [],
        immune: null,
      };
      board.push({ cell: { x, y }, unit: id });
      if (owner === 'player' && playerId < 0) playerId = id;
      if (owner === 'keeper' && keeperId < 0) keeperId = id;
    };
    for (const p of study.setup.playerBoard) put(p.card, p.x, p.y, 'player');
    for (const p of study.setup.keeperBoard) put(p.card, p.x, p.y, 'keeper');
    const handOf = (slugs: string[]) =>
      slugs.map((s) => find(s)).filter(Boolean).map((c) => bodyOf(c as BattleCard));
    const playerHand = handOf(study.setup.playerHand);
    const occupied = new Set(board.map((s) => `${s.cell.x},${s.cell.y}`));
    const legal: BattleAction[] = ['endTurn'];
    for (const spot of board) {
      const u = units[spot.unit];
      if (!u || u.owner !== 'player') continue;
      legal.push({
        move: {
          unit: u.id,
          to: { x: (spot.cell.x + 1) % 3, y: spot.cell.y },
        },
      });
      for (const other of board) {
        const t = units[other.unit];
        if (t && t.owner === 'keeper') {
          legal.push({ attack: { attacker: u.id, target: t.id } });
          break;
        }
      }
    }
    for (let i = 0; i < playerHand.length; i++) {
      for (let x = 0; x < 3; x++) {
        for (const y of [3, 4, 5]) {
          if (!occupied.has(`${x},${y}`)) {
            legal.push({ play: { handIndex: i, cell: { x, y } } });
            occupied.add(`${x},${y}`);
            break;
          }
        }
      }
    }
    const events: BattleMatch['events'] = [];
    if (playerId >= 0 && board[0]) {
      const from = board.find((s) => s.unit === playerId)?.cell ?? { x: 0, y: 4 };
      events.push({
        moved: { unit: playerId, from, to: { x: from.x, y: from.y } },
      });
    }
    if (playerId >= 0 && keeperId >= 0) {
      events.push({
        damaged: {
          target: keeperId,
          by: playerId,
          toHealth: 2,
          toShield: 0,
          channel: 'physical',
          source: 'attack',
          trail: [{ step: 'pointBlank', from: 3, to: 2 }],
        },
      });
    }
    events.push({ turnEnded: { side: 'player', round: 1 } });
    return {
      id: 'chamber-preview',
      challengeId: study.id,
      seq: 0,
      state: {
        units,
        board,
        player: { hand: playerHand, mana: 3, manaMax: 5 },
        keeper: {
          hand: handOf(study.setup.keeperHand),
          mana: 2,
          manaMax: 2,
        },
        round: 1,
        active: 'player',
        outcome: sealed ? 'player' : null,
        rules: study.setup.rules ?? HOUSE_RULES,
        openingAttacksUsed: 0,
        actsThisTurn: 0,
      },
      legalActions: legal,
      events,
      outcome: sealed ? 'player' : null,
      rewardDust: sealed ? 12 : 0,
      marks: sealed
        ? { acts: 16, bodiesLost: 2, bestKnown: 9, record: false }
        : undefined,
    };
  }

  /** Встреча, на которую идут с неразложенным столом. */
  const needsTable = (c: BattleChallenge) => c.playerSide === 'deck' && !laid;

  function playWord(challenge: BattleChallenge) {
    if (challenge.openMatchId) return $t('battleStudyContinue');
    if (needsTable(challenge)) return $t('battleLayYourTable');
    return $t('battleStudyPlay');
  }

  async function takeUp(challenge: BattleChallenge) {
    const token = authStore.token;
    if (!token) return;
    if (needsTable(challenge)) {
      void goto('/battles/table');
      return;
    }
    busy = true;
    complaint = null;
    taken = challenge;
    try {
      match = await api.beginBattleMatch(token, challenge.id);
    } catch (e) {
      complaint = String(e).includes('nothingToBring')
        ? $t('battleNothingToBring')
        : $t('battleActionLost');
    } finally {
      busy = false;
    }
  }

  /** Тот же этюд с начала. Открытая партия сбрасывается, не считается проигрышем. */
  async function again() {
    const token = authStore.token;
    if (!token || !taken) return;
    busy = true;
    complaint = null;
    leaving = false;
    try {
      match = await api.restartBattleMatch(token, taken.id);
      challenges = await api.getBattleChallenges(token);
    } catch (e) {
      complaint = String(e).includes('nothingToBring')
        ? $t('battleNothingToBring')
        : $t('battleActionLost');
    } finally {
      busy = false;
    }
  }

  function previewAct(action: BattleAction) {
    const now = match;
    if (!now || now.id !== 'chamber-preview') return;
    const from = now.state.board.find((s) =>
      typeof action !== 'string' && 'move' in action ? s.unit === action.move.unit : false,
    );
    if (action === 'endTurn') {
      match = {
        ...now,
        seq: now.seq + 1,
        events: [{ turnEnded: { side: 'player', round: now.state.round } }],
        state: { ...now.state, active: 'keeper' },
        legalActions: [],
      };
      return;
    }
    if (typeof action !== 'string' && 'move' in action) {
      match = {
        ...now,
        seq: now.seq + 1,
        events: [
          {
            moved: {
              unit: action.move.unit,
              from: from?.cell ?? action.move.to,
              to: action.move.to,
            },
          },
        ],
        state: {
          ...now.state,
          board: now.state.board.map((s) =>
            s.unit === action.move.unit ? { ...s, cell: action.move.to } : s,
          ),
        },
      };
      return;
    }
    if (typeof action !== 'string' && 'attack' in action) {
      const target = now.state.units[action.attack.target];
      const hp = target ? Math.min(2, target.health.current) : 2;
      match = {
        ...now,
        seq: now.seq + 1,
        events: [
          {
            damaged: {
              target: action.attack.target,
              by: action.attack.attacker,
              toHealth: hp,
              toShield: 0,
              channel: 'physical',
              source: 'strike',
              trail: [{ step: 'floor', from: hp + 1, to: 1 }],
            },
          },
        ],
      };
    }
  }

  async function play(action: BattleAction) {
    const token = authStore.token;
    if (!match) return;
    if (match.id === 'chamber-preview') {
      previewAct(action);
      return;
    }
    if (!token) return;
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

  /**
   * «Если сделать это и на этом закончить ход — чем ответит хранитель».
   *
   * Считает сервер: сцена не знает ни одного правила, а страница — ни одного
   * адреса сверх этого вызова. Ничего не пишет и ни на что не влияет, поэтому
   * молчаливый отказ здесь — правильный ответ: предвестие, которое не пришло,
   * должно просто не появиться, а не сообщать об ошибке посреди партии.
   */
  async function foresee(action: BattleAction): Promise<Foresight | null> {
    if (match?.id === 'chamber-preview') return null;
    const token = authStore.token;
    if (!token || !match) return null;
    try {
      return await api.foreseeBattleMatch(token, match.id, match.seq, action);
    } catch {
      return null;
    }
  }

  function askLeave() {
    if (match && !match.outcome) {
      leaving = true;
      return;
    }
    putBack();
  }

  function keepMatch() {
    leaving = false;
    putBack();
  }

  async function giveField() {
    const token = authStore.token;
    if (!token || !match) return;
    busy = true;
    leaving = false;
    complaint = null;
    try {
      match = await api.yieldBattleMatch(token, match.id);
      challenges = await api.getBattleChallenges(token);
    } catch {
      complaint = $t('battleActionLost');
    } finally {
      busy = false;
    }
  }

  function putBack() {
    match = null;
    taken = null;
    complaint = null;
    leaving = false;
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape' && leaving) {
      leaving = false;
      e.stopPropagation();
    }
  }
</script>

<svelte:window onkeydown={onkey} />

<svelte:head>
  <title>{$t('battleStudies')} — {$brandName}</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<div class="root" class:root--match={!!match}>
  <div class="grain" aria-hidden="true"></div>
  <div class="page" class:page--match={!!match}>
    {#if !match}
      <BattleDoor />

      <header class="masthead">
        <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('battlesPageKicker')}</p>
        <h1 class="page-title">{$t('battleStudies')}</h1>
        <p class="page-rule">{$t('battleStudiesRule')}</p>
      </header>
    {/if}

    {#if complaint}
      <p class="fault" transition:fade={{ duration: 150 }}>{complaint}</p>
    {/if}

    {#if match}
      <BattleScene
        {match}
        {cards}
        {frames}
        {motions}
        {busy}
        fill
        onact={play}
        onforesee={foresee}
        onleave={putBack}
        onexit={askLeave}
        onreplay={taken ? again : undefined}
      />

    {:else}
      {#if !signedIn}
        <p class="sign">
          <a href={loginFrom()}>{$t('battleStudySignIn')}</a>
        </p>
      {/if}

      {#if !challenges.length}
        <p class="empty">{$t('battleStudiesEmpty')}</p>
      {:else}
        <ul class="list">
          {#each challenges as challenge (challenge.id)}
            <li class="row">
              <div class="copy">
                <span class="name">{titleOf(challenge)}</span>
                {#if noteOf(challenge)}
                  <span class="note">{noteOf(challenge)}</span>
                {/if}
                <!-- Два рода записей на одной полке: этюд расставлен рукой
                     целиком, во встречу вы приводите своё. -->
                <span class="kind">
                  {challenge.playerSide === 'deck' ? $t('battleMeetingNote') : $t('battleStudyNote')}
                </span>
                <!-- Чем этот бой отличается от соседнего. Названо только
                     отличие: свод, в котором перечислены все десять правил,
                     не сообщает ничего — читатель обязан помнить наизусть,
                     какие из них обычные. -->
                {#if apart(challenge).length}
                  <span class="apart">
                    {$t('battleRulesOwn')}:
                    {#each apart(challenge) as line, i (line.key)}{i > 0
                        ? ' · '
                        : ' '}{$t(line.key)}{line.amount === null
                        ? ''
                        : ` — ${line.amount}`}{/each}
                  </span>
                {/if}
                <!-- Три печати вместо одной. Победа была двоичной: прошёл и
                     забыл. «За пять дел вместо семи» — утверждение о человеке,
                     и оно не зависит от того, кому досталась монета. -->
                {#if challenge.marks}
                  <span class="marks">
                    <!-- «Доведено» — печать проигравшего: победа говорит то же
                         самое и громче, и две печати рядом читались бы как две
                         разные вещи. -->
                    {#if challenge.marks.finished && !challenge.marks.won}
                      <span class="mark mark--done">{$t('battleMarkFinished')}</span>
                    {/if}
                    {#if challenge.marks.won}
                      <span class="mark mark--won">{$t('battleMarkWon')}</span>
                    {/if}
                    {#if challenge.marks.clean}
                      <span class="mark mark--clean">{$t('battleMarkClean')}</span>
                    {/if}
                    {#if challenge.marks.yourBest != null}
                      <span class="line"
                        >{$t('battleMarkYourLine')} — {challenge.marks.yourBest}</span
                      >
                    {/if}
                    {#if challenge.marks.bestKnown != null}
                      <span class="line line--bar"
                        >{$t('battleMarkBestLine')} — {challenge.marks.bestKnown}</span
                      >
                    {/if}
                  </span>
                {/if}
              </div>
              {#if challenge.rewardDust > 0 || challenge.rewardFinishDust > 0}
                <span class="reward">
                  {#if challenge.alreadyPaid}
                    {$t('battleStudyPaid')}
                  {:else}
                    {#if challenge.rewardDust > 0}
                      {challenge.rewardDust} {$t('battleStudyReward')}
                    {/if}
                    <!-- Сказано ЗАРАНЕЕ, а не после проигрыша: человек, знающий,
                         что за доведённую до конца партию платят, садится за
                         доску иначе, чем тот, кто думает, что играет за всё или
                         ничего. -->
                    {#if challenge.rewardFinishDust > 0}
                      <span class="reward-finish"
                        >{challenge.rewardFinishDust} {$t('battleStudyFinishReward')}</span
                      >
                    {/if}
                  {/if}
                </span>
              {/if}
              {#if signedIn}
                <button
                  type="button"
                  class="play"
                  disabled={busy}
                  onclick={() => takeUp(challenge)}
                >{playWord(challenge)}</button>
              {:else}
                <a class="play play--door" href={loginFrom(challenge)}>{$t('battleStudySignInStart')}</a>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </div>
</div>

{#if leaving}
  <div
    class="veil"
    role="presentation"
    onclick={() => (leaving = false)}
    transition:fade={{ duration: 200 }}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="ask"
      role="dialog"
      aria-modal="true"
      aria-label={$t('battleLeave')}
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
    >
      <p class="ask-word">{$t('battleLeaveAsk')}</p>
      <p class="ask-doors">
        <button type="button" class="ask-keep" onclick={keepMatch}>{$t('battleLeaveKeep')}</button>
        <button type="button" class="ask-yield" onclick={giveField}>{$t('battleLeaveYield')}</button>
      </p>
    </div>
  </div>
{/if}

<style>
  .root {
    position: relative;
    min-height: 100vh;
    background: #f8f1e7;
    color: #34251c;
  }

  .root--match {
    height: 100dvh;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    background: #0b0806;
    color: #e6dcc8;
  }

  .root--match .grain {
    display: none;
  }

  .grain {
    position: fixed;
    inset: 0;
    pointer-events: none;
    opacity: 0.05;
    background-image: radial-gradient(#34251c 0.5px, transparent 0.5px);
    background-size: 4px 4px;
  }

  .page {
    position: relative;
    max-width: 1180px;
    margin: 0 auto;
    padding: 3rem 1.5rem 6rem;
  }

  .page--match {
    flex: 1 1 auto;
    min-height: 0;
    max-width: none;
    width: 100%;
    padding: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
  }

  .page--match :global(.room) {
    flex: 1 1 auto;
    min-height: 0;
  }

  .eyebrow {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    margin: 0 0 1rem;
    font-size: 0.68rem;
    letter-spacing: 0.28em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .eyebrow-rule {
    width: 3.5rem;
    height: 1px;
    background: #d8c6b1;
  }

  .page-title {
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: clamp(2rem, 5vw, 3.2rem);
    font-weight: 400;
    line-height: 1.1;
  }

  .page-rule {
    max-width: 56ch;
    margin: 1rem 0 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1rem;
    line-height: 1.6;
    opacity: 0.74;
  }

  .fault {
    margin: 0 0 1.25rem;
    font-size: 0.9rem;
    color: #8f2f22;
  }

  .sign,
  .empty {
    margin: 1.5rem 0 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.95rem;
    font-style: italic;
    color: #5f4636;
  }

  .sign a {
    color: inherit;
    text-decoration: underline;
    text-underline-offset: 0.18em;
  }

  .sign a:hover {
    color: #c65f3c;
  }

  .list {
    margin: 2rem 0 0;
    padding: 0;
    list-style: none;
    border-top: 1px solid #d8c6b1;
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.7rem 1.1rem;
    padding: 0.9rem 0;
    border-bottom: 1px solid #d8c6b1;
  }

  .copy {
    flex: 1 1 14rem;
    min-width: 0;
  }

  .name {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.15rem;
  }

  .note,
  .kind {
    display: block;
    margin-top: 0.2rem;
    font-size: 0.78rem;
    line-height: 1.45;
    color: #8a6a55;
  }

  .kind {
    font-style: italic;
  }

  .apart {
    display: block;
    margin-top: 0.25rem;
    font-size: 0.72rem;
    line-height: 1.5;
    color: #6f3b24;
  }

  /* Печати. Не значки и не медали: три коротких слова в строку — комната,
     которая хвалит громче, чем говорит, перестаёт быть этой комнатой. */
  .marks {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.35rem 0.6rem;
    margin-top: 0.35rem;
  }

  .mark {
    padding: 0.08rem 0.4rem;
    font-size: 0.66rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #6f3b24;
    border: 1px solid #d8c6b1;
  }

  .mark--won {
    color: #c65f3c;
    border-color: #c65f3c;
  }

  .mark--clean {
    color: #f8f1e7;
    background: #6f3b24;
    border-color: #6f3b24;
  }

  .line {
    font-size: 0.7rem;
    font-variant-numeric: tabular-nums;
    color: #8a6a55;
  }

  /* Планка дома. Она и есть причина вернуться, поэтому читается как чужой
     результат, а не как своя строка. */
  .line--bar {
    font-style: italic;
  }

  .reward-finish {
    display: block;
    opacity: 0.75;
  }

  .reward {
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
    color: #8a6a55;
    white-space: nowrap;
  }

  .play {
    padding: 0.4rem 0.7rem;
    font: inherit;
    font-size: 0.68rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #6f3b24;
    background: none;
    border: 1px solid #d8c6b1;
    text-decoration: none;
    white-space: nowrap;
    cursor: pointer;
  }

  .play:hover:not(:disabled) {
    color: #c65f3c;
    border-color: #c65f3c;
  }

  .play:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .play--door {
    display: inline-block;
  }

  .leave-row {
    display: none;
  }

  .page--match .fault {
    position: absolute;
    top: 4.6rem;
    left: 50%;
    z-index: 30;
    transform: translateX(-50%);
    margin: 0;
    padding: 0.35rem 0.8rem;
    background: rgba(18, 10, 8, 0.88);
    border: 1px solid #8a7040;
    color: #f0c8b0;
  }

  .veil {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    background: rgba(8, 6, 5, 0.72);
    backdrop-filter: blur(2px);
  }

  .ask {
    max-width: 28rem;
    padding: 1.6rem 1.7rem 1.4rem;
    background: #1a1410;
    border: 1px solid #8a7040;
    outline: 1px solid #8a7040;
    outline-offset: 4px;
    transform: rotate(-1deg);
    color: #e6dcc8;
  }

  .ask-word {
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.05rem;
    line-height: 1.5;
    color: #e6dcc8;
  }

  .ask-doors {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem 1.4rem;
    margin: 1.1rem 0 0;
  }

  .ask-keep,
  .ask-yield {
    padding: 0;
    font: inherit;
    font-size: 0.68rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    background: none;
    border: 0;
    cursor: pointer;
  }

  .ask-keep {
    color: #e8c96a;
  }

  .ask-yield {
    color: #a09070;
  }

  .ask-keep:hover,
  .ask-yield:hover {
    color: #c65f3c;
  }
</style>
