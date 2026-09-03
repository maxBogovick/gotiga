<script lang="ts">
  // Стол такта. Не копия стола рамок: движение живёт во времени.
  //
  // Лица сверху — дом, заготовки, ящик. Сцена — настоящие карты на поле 3×6,
  // тот же `stage()`, что в комнате, и крутится сама. Партитура — четыре
  // дорожки; колодец слов ставит замах или свет, а не селект из семнадцати.
  // Повод на записи — подсказка, не замок: карта вешает вид на любой повод.
  import { onMount, untrack } from 'svelte';
  import { api } from '$lib/api';
  import { t, lang } from '$lib/i18n';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import BattleMotionStage from '$lib/components/BattleMotionStage.svelte';
  import BattleAssetPicker from '$lib/components/admin/BattleAssetPicker.svelte';
  import {
    DEFAULT_ASPECT,
    DEFAULT_MOTIONS,
    GESTURE_FADES,
    GESTURE_LAYERS,
    GESTURE_NUDGE_MAX,
    GESTURE_SIZE_MAX,
    GESTURE_TURNS,
    MOTION_FRAMES_MAX,
    MOTION_MS_MAX,
    MOTION_OCCASIONS,
    MOTIONS_MAX,
    STOCK_MOTIONS,
    completeSlices,
    frameForCard,
    isLight,
    isMove,
    isSlot,
    motionSpan,
    motionTitle,
    motionWound,
    newGesture,
    newMotion,
    newSlot,
    oneStirPerBody,
    parseMotionWear,
    stage,
    struckOf,
    takeHouse,
    takeStock,
    type HitWear,
    type Staged,
  } from '$lib/battles';
  import type {
    BattleCard as BattleCardDto,
    BattleFrame,
    BattleRace,
    GestureBody,
    GestureFade,
    GestureTurn,
    GestureWhom,
    Motion,
    MotionGesture,
    MotionOccasion,
  } from '$lib/types/api';
  import type { TranslationKey } from '$lib/i18n';

  let {
    flash,
    onSaved,
  }: {
    flash?: (text: string) => void;
    onSaved?: (motions: Motion[]) => void;
  } = $props();

  let motions = $state<Motion[]>([]);
  let cards = $state<BattleCardDto[]>([]);
  let races = $state<BattleRace[]>([]);
  let frames = $state<BattleFrame[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let complaint = $state('');

  let stored = $state('');
  let dirty = $derived(stored !== '' && JSON.stringify(motions) !== stored);

  type FaceKind = 'house' | 'stock' | 'mine';
  let faceKind = $state<FaceKind>('house');
  let houseAt = $state(0);
  let stockAt = $state(0);
  let heldId = $state<string | null>(null);

  let held = $derived.by((): Motion | null => {
    if (faceKind === 'house') return DEFAULT_MOTIONS[houseAt] ?? null;
    if (faceKind === 'stock') {
      const ready = STOCK_MOTIONS[stockAt];
      if (!ready) return null;
      return {
        id: `stock-${stockAt}`,
        nameEn: ready.nameEn,
        nameRu: ready.nameRu,
        occasion: ready.occasion,
        gestures: ready.gestures.map((g) => ({ ...g })),
      };
    }
    return motions.find((m) => m.id === heldId) ?? null;
  });

  let editable = $derived(faceKind === 'mine');
  let gestureAt = $state(0);
  let gesture = $derived(held?.gestures[gestureAt] ?? null);
  let track = $state<GestureWhom>('striker');

  const WIDTH = 3;
  const DEPTH = 6;
  let reach = $state(3);
  let along = $state(false);
  let strikerCard = $state('');
  let targetCard = $state('');

  let from = $derived({ x: 1, y: DEPTH - 1 });
  let to = $derived({
    x: 1,
    y: Math.max(0, DEPTH - 1 - Math.min(reach, DEPTH - 1)),
  });

  let spanX = $derived(along ? DEPTH : WIDTH);
  let spanY = $derived(along ? WIDTH : DEPTH);

  let spots = $derived(
    along
      ? Array.from({ length: WIDTH }, (_, x) =>
          Array.from({ length: DEPTH }, (_, y) => ({ x, y })),
        ).flat()
      : Array.from({ length: DEPTH }, (_, y) =>
          Array.from({ length: WIDTH }, (_, x) => ({ x, y })),
        ).flat(),
  );

  let play = $state<Staged | null>(null);
  let playKey = $state(0);
  let playGen = 0;
  let playRev = $state(0);
  let looping = $state(true);
  let pinned = $state(false);
  let playing = $state(false);

  const dtoOf = (id: string) => cards.find((c) => c.id === id) ?? null;
  const aspectOf = (dto: BattleCardDto) =>
    frameForCard(dto, frames).aspect || DEFAULT_ASPECT;
  let strikerDto = $derived(dtoOf(strikerCard));
  let targetDto = $derived(dtoOf(targetCard));

  /** Стол показывает след удара на цели, как комната: синяк и обломок на
   *  ударе, чернила на чаре. Числа учебные, не из партии. */
  function deskHit(motion: Motion): HitWear | null {
    const at = motionWound(motion);
    if (motion.occasion === 'blow') {
      return { remain: 0.42, blow: 0.28, seed: 3, channel: 'physical', at };
    }
    if (motion.occasion === 'spell') {
      return { remain: 0.7, blow: 0.2, seed: 3, channel: 'magic', at };
    }
    return null;
  }

  let previewHit = $derived(held ? deskHit(held) : null);
  let previewStruck = $derived(struckOf(previewHit));
  let woundOn = $state(false);

  function bump() {
    playRev += 1;
  }

  // Писать play/playKey/playing здесь нельзя без untrack: `playKey += 1`
  // и читает, и пишет одно поле — эффект гоняет себя до
  // `effect_update_depth_exceeded` и вешает вкладку.
  $effect(() => {
    faceKind;
    houseAt;
    stockAt;
    heldId;
    playRev;
    along;
    looping;
    pinned;
    const alongNow = along;
    const loop = looping;
    const pin = pinned;

    const put = () => {
      const motion = held;
      if (!motion) {
        play = null;
        playing = false;
        woundOn = false;
        return 0;
      }
      woundOn = false;
      play = stage(motion, from, to, {
        spanX: alongNow ? DEPTH : WIDTH,
        spanY: alongNow ? WIDTH : DEPTH,
        along: alongNow,
        calm: false,
      });
      playGen += 1;
      playKey = playGen;
      playing = true;
      const gen = playGen;
      const delay = motionWound(motion);
      if (delay <= 0) woundOn = true;
      else {
        setTimeout(() => {
          if (playKey === gen) woundOn = true;
        }, delay);
      }
      return motionSpan(motion);
    };

    const spanMs = untrack(put);
    if (!loop || pin || !spanMs) return;
    const wait = Math.max(480, spanMs + 380);
    const id = setInterval(() => untrack(put), wait);
    return () => clearInterval(id);
  });

  function ensureMine(): Motion | null {
    if (faceKind === 'mine') return motions.find((m) => m.id === heldId) ?? null;
    if (motions.length >= MOTIONS_MAX) return null;
    const src = held;
    if (!src) return null;
    const born: Motion = {
      ...structuredClone($state.snapshot(src)),
      id: newMotion().id,
    };
    motions = [...motions, born];
    faceKind = 'mine';
    heldId = born.id;
    return born;
  }

  function takeFace() {
    if (faceKind === 'house') {
      if (motions.length >= MOTIONS_MAX) return;
      const born = takeHouse(houseAt);
      if (!born) return;
      motions = [...motions, born];
      faceKind = 'mine';
      heldId = born.id;
      gestureAt = 0;
      bump();
      return;
    }
    if (faceKind === 'stock') {
      if (motions.length >= MOTIONS_MAX) return;
      const born = takeStock(stockAt);
      if (!born) return;
      motions = [...motions, born];
      faceKind = 'mine';
      heldId = born.id;
      gestureAt = 0;
      bump();
    }
  }

  function addBlank() {
    if (motions.length >= MOTIONS_MAX) return;
    const born = newMotion('blow');
    born.nameRu = $t('adminMotionsUntitled');
    born.nameEn = 'New motion';
    motions = [...motions, born];
    faceKind = 'mine';
    heldId = born.id;
    gestureAt = 0;
    bump();
  }

  function duplicate() {
    const src = ensureMine();
    if (!src || motions.length >= MOTIONS_MAX) return;
    const twin: Motion = {
      ...structuredClone($state.snapshot(src)),
      id: newMotion().id,
      nameRu: `${src.nameRu} ·`,
      nameEn: `${src.nameEn} ·`,
    };
    motions = [...motions, twin];
    heldId = twin.id;
    gestureAt = 0;
    bump();
  }

  function dropHeld() {
    if (faceKind !== 'mine' || !heldId) return;
    motions = motions.filter((m) => m.id !== heldId);
    heldId = motions[0]?.id ?? null;
    if (!heldId) {
      faceKind = 'house';
      houseAt = 0;
    }
    gestureAt = 0;
    bump();
  }

  function putBody(body: GestureBody) {
    if (track === 'flight' || track === 'field') return;
    const mine = ensureMine();
    if (!mine) return;
    const pred = isLight(body) ? isLight : isMove;
    const at = mine.gestures.findIndex((g) => g.whom === track && pred(g.body));
    if (at >= 0) {
      mine.gestures[at].body = body;
      gestureAt = at;
    } else {
      const g = { ...newGesture(track), body, fade: 'hold' as const };
      mine.gestures = oneStirPerBody([...mine.gestures, g]);
      gestureAt = mine.gestures.findIndex((x) => x.whom === track && x.body === body);
    }
    mine.gestures = [...mine.gestures];
    bump();
  }

  function openSlot(whom: 'flight' | 'field') {
    const mine = ensureMine();
    if (!mine) return;
    const at = mine.gestures.findIndex((g) => g.whom === whom);
    if (at >= 0) {
      gestureAt = at;
      track = whom;
      return;
    }
    mine.gestures = [...mine.gestures, newSlot(whom)];
    gestureAt = mine.gestures.length - 1;
    track = whom;
    bump();
  }

  function dropGesture(i: number) {
    const mine = ensureMine();
    if (!mine) return;
    mine.gestures = mine.gestures.filter((_, k) => k !== i);
    gestureAt = Math.max(0, Math.min(i, mine.gestures.length - 1));
    bump();
  }

  let picking = $state(false);
  let uploading = $state(false);

  async function upload(file: File) {
    const mine = ensureMine();
    const g = mine?.gestures[gestureAt];
    if (!g) return;
    uploading = true;
    try {
      const got = await api.adminUploadBattleFrameArt(file);
      g.image = got.url;
      mine.gestures = [...mine.gestures];
      bump();
    } catch (e) {
      complaint = String(e);
    } finally {
      uploading = false;
    }
  }

  function setArt(url: string) {
    const mine = ensureMine();
    const g = mine?.gestures[gestureAt];
    if (!g) return;
    g.image = url;
    mine.gestures = [...mine.gestures];
    bump();
  }

  // ── Партитура: бар тянут ─────────────────────────────────────────────────

  let drag = $state<{
    i: number;
    mode: 'at' | 'dur';
    origin: number;
    startAt: number;
    startDur: number;
    width: number;
  } | null>(null);

  function msFromPx(dx: number, width: number) {
    if (width <= 0) return 0;
    return Math.round(((dx / width) * MOTION_MS_MAX) / 10) * 10;
  }

  function barDown(e: PointerEvent, i: number, mode: 'at' | 'dur') {
    const mine = ensureMine();
    const g = mine?.gestures[i];
    if (!g) return;
    const row = (e.currentTarget as HTMLElement).closest('.score-row');
    const width = row?.getBoundingClientRect().width ?? 1;
    pinned = true;
    gestureAt = i;
    track = g.whom;
    drag = {
      i,
      mode,
      origin: e.clientX,
      startAt: g.at,
      startDur: g.dur,
      width,
    };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function barMove(e: PointerEvent) {
    if (!drag) return;
    const mine = motions.find((m) => m.id === heldId);
    const g = mine?.gestures[drag.i];
    if (!g) return;
    const delta = msFromPx(e.clientX - drag.origin, drag.width);
    if (drag.mode === 'at') {
      g.at = Math.max(0, Math.min(MOTION_MS_MAX - g.dur, drag.startAt + delta));
    } else {
      g.dur = Math.max(40, Math.min(MOTION_MS_MAX - g.at, drag.startDur + delta));
    }
    mine.gestures = [...mine.gestures];
  }

  function barUp() {
    drag = null;
    pinned = false;
    bump();
  }

  let turnRun = 0;
  async function playTurn() {
    const token = ++turnRun;
    looping = false;
    pinned = true;
    bump();
    const beat = () => Math.max(280, motionSpan(held) + 80);
    await new Promise((r) => setTimeout(r, beat()));
    if (token !== turnRun) return;
    await new Promise((r) => setTimeout(r, 500));
    if (token !== turnRun) return;
    bump();
    await new Promise((r) => setTimeout(r, beat()));
    if (token !== turnRun) return;
    await new Promise((r) => setTimeout(r, 500));
    if (token !== turnRun) return;
    pinned = false;
    looping = true;
    bump();
  }

  async function load() {
    loading = true;
    try {
      const [moving, deck, dressing, kin] = await Promise.all([
        api.getBattleMotions(),
        api.adminListBattleCards(),
        api.getBattleFrames(),
        api.getBattleRaces(),
      ]);
      motions = moving.motions;
      cards = deck;
      races = kin;
      frames = (dressing.frames ?? []).map(completeSlices);
      stored = JSON.stringify(motions);
      strikerCard = strikerCard || (cards[0]?.id ?? '');
      targetCard = targetCard || (cards[1]?.id ?? cards[0]?.id ?? '');
      if (motions[0]) {
        faceKind = 'mine';
        heldId = motions[0].id;
      }
    } catch (e) {
      complaint = String(e);
    } finally {
      loading = false;
    }
  }

  async function save() {
    saving = true;
    complaint = '';
    try {
      const sent = $state.snapshot(motions);
      const back = await api.adminSaveBattleMotions({ motions: sent });
      motions = back.motions;
      stored = JSON.stringify(motions);
      onSaved?.(motions);
      if (heldId && !motions.some((m) => m.id === heldId)) {
        heldId = motions[0]?.id ?? null;
        if (!heldId) faceKind = 'house';
      }
      const lost = sent.length - back.motions.length;
      flash?.(lost > 0 ? `${$t('adminMotionsTrimmed')} ${lost}` : $t('adminMotionsSaved'));
    } catch (e) {
      complaint = String(e);
    } finally {
      saving = false;
    }
  }

  onMount(() => {
    void load();
    const guard = (e: BeforeUnloadEvent) => {
      if (untrack(() => dirty)) e.preventDefault();
    };
    window.addEventListener('beforeunload', guard);
    return () => window.removeEventListener('beforeunload', guard);
  });

  const OCCASION_KEY: Record<MotionOccasion, TranslationKey> = {
    blow: 'adminMotionsOccasionBlow',
    spell: 'adminMotionsOccasionSpell',
    mend: 'adminMotionsOccasionMend',
    arrive: 'adminMotionsOccasionArrive',
    fall: 'adminMotionsOccasionFall',
    unseen: 'adminMotionsOccasionUnseen',
  };
  const WHOM_KEY: Record<GestureWhom, TranslationKey> = {
    striker: 'adminMotionsWhomStriker',
    target: 'adminMotionsWhomTarget',
    flight: 'adminMotionsWhomFlight',
    field: 'adminMotionsWhomField',
  };
  const BODY_KEY: Record<GestureBody, TranslationKey> = {
    none: 'adminMotionsBodyNone',
    lunge: 'adminMotionsBodyLunge',
    flinch: 'adminMotionsBodyFlinch',
    shiver: 'adminMotionsBodyShiver',
    sink: 'adminMotionsBodySink',
    rise: 'adminMotionsBodyRise',
    swell: 'adminMotionsBodySwell',
    bow: 'adminMotionsBodyBow',
    draw: 'adminMotionsBodyDraw',
    recoil: 'adminMotionsBodyRecoil',
    heave: 'adminMotionsBodyHeave',
    shudder: 'adminMotionsBodyShudder',
    sway: 'adminMotionsBodySway',
    loom: 'adminMotionsBodyLoom',
    kindle: 'adminMotionsBodyKindle',
    blanch: 'adminMotionsBodyBlanch',
    wither: 'adminMotionsBodyWither',
  };
  const TURN_KEY: Record<GestureTurn, TranslationKey> = {
    none: 'adminMotionsTurnNone',
    toTarget: 'adminMotionsTurnToTarget',
    mirror: 'adminMotionsTurnMirror',
  };
  const FADE_KEY: Record<GestureFade, TranslationKey> = {
    hold: 'adminMotionsFadeHold',
    in: 'adminMotionsFadeIn',
    out: 'adminMotionsFadeOut',
    inOut: 'adminMotionsFadeInOut',
  };

  const WELL: { key: TranslationKey; bodies: GestureBody[] }[] = [
    { key: 'adminMotionsWellToward', bodies: ['lunge', 'heave', 'draw'] },
    { key: 'adminMotionsWellStruck', bodies: ['flinch', 'shiver', 'shudder', 'recoil'] },
    { key: 'adminMotionsWellPose', bodies: ['bow', 'sway', 'loom'] },
    { key: 'adminMotionsWellPresence', bodies: ['swell', 'sink', 'rise'] },
    { key: 'adminMotionsWellLight', bodies: ['kindle', 'blanch', 'wither'] },
  ];

  const TRACKS: GestureWhom[] = ['striker', 'target', 'flight', 'field'];

  let span = $derived(motionSpan(held));

  const barWord = (g: MotionGesture) => {
    if (isSlot(g)) return $t('adminMotionsSlot');
    if (g.image) return $t('adminMotionsHasArt');
    return $t(BODY_KEY[g.body]);
  };

  let tracePts = $derived({
    x1: (along ? from.y : from.x) + 0.5,
    y1: (along ? from.x : from.y) + 0.5,
    x2: (along ? to.y : to.x) + 0.5,
    y2: (along ? to.x : to.y) + 0.5,
  });

  let wearers = $derived.by(() => {
    if (faceKind !== 'mine' || !heldId) return [];
    const out: { name: string; occasion: MotionOccasion }[] = [];
    const nameOf = (ru: string, en: string) => ($lang === 'ru' ? ru : en);
    for (const c of cards) {
      const w = parseMotionWear(c.motionWear);
      for (const o of MOTION_OCCASIONS) {
        if (w[o] === heldId) out.push({ name: nameOf(c.titleRu, c.titleEn), occasion: o });
      }
    }
    for (const r of races) {
      const w = parseMotionWear(r.motionWear);
      for (const o of MOTION_OCCASIONS) {
        if (w[o] === heldId)
          out.push({ name: nameOf(r.nameRu, r.nameEn), occasion: o });
      }
    }
    return out;
  });

  const activeBody = $derived.by(() => {
    const g = held?.gestures.find((x) => x.whom === track && isMove(x.body));
    const light = held?.gestures.find((x) => x.whom === track && isLight(x.body));
    return { move: g?.body, light: light?.body };
  });
</script>

<div class="desk">
  <!-- ── Лица ──────────────────────────────────────────────────────────── -->
  <header class="faces">
    <div class="face-col">
      <p class="kicker">{$t('adminMotionsFacesHouse')}</p>
      <div class="chips">
        {#each DEFAULT_MOTIONS as motion, i (motion.id)}
          <button
            type="button"
            class="chip"
            class:chip--on={faceKind === 'house' && houseAt === i}
            onclick={() => {
              faceKind = 'house';
              houseAt = i;
              gestureAt = 0;
              bump();
            }}>{motionTitle(motion, $lang)}</button
          >
        {/each}
      </div>
    </div>
    <div class="face-col">
      <p class="kicker">{$t('adminMotionsFacesStock')}</p>
      <div class="chips">
        {#each STOCK_MOTIONS as ready, i (i)}
          <button
            type="button"
            class="chip"
            class:chip--on={faceKind === 'stock' && stockAt === i}
            onclick={() => {
              faceKind = 'stock';
              stockAt = i;
              gestureAt = 0;
              bump();
            }}>{ $lang === 'ru' ? ready.nameRu : ready.nameEn }</button
          >
        {/each}
      </div>
    </div>
    <div class="face-col">
      <p class="kicker">{$t('adminMotionsFacesMine')}</p>
      <div class="chips">
        {#each motions as motion (motion.id)}
          <button
            type="button"
            class="chip"
            class:chip--on={faceKind === 'mine' && heldId === motion.id}
            onclick={() => {
              faceKind = 'mine';
              heldId = motion.id;
              gestureAt = 0;
              bump();
            }}>{motionTitle(motion, $lang)}</button
          >
        {/each}
        <button type="button" class="chip chip--add" onclick={addBlank}>+</button>
      </div>
    </div>
    <div class="face-save">
      {#if faceKind !== 'mine'}
        <button type="button" class="btn" onclick={takeFace}>{$t('adminMotionsTake')}</button>
      {/if}
      <button type="button" class="btn" disabled={saving || !dirty} onclick={save}
        >{$t('adminMotionsSave')}</button
      >
      {#if dirty}
        <span class="warn">{$t('adminMotionsUnsaved')}</span>
      {/if}
    </div>
  </header>

  {#if loading}
    <p class="note px-4 py-3">{$t('adminMotionsLoading')}</p>
  {:else if !held}
    <p class="note px-4 py-3">{$t('adminMotionsNothingHeld')}</p>
  {:else}
    <div class="body">
      <!-- ── Сцена ─────────────────────────────────────────────────────── -->
      <section class="stage-wrap">
        {#if faceKind === 'mine'}
          <div class="names">
            <label>
              <span class="kicker">{$t('adminMotionsNameRu')}</span>
              <input bind:value={held.nameRu} />
            </label>
            <label>
              <span class="kicker">{$t('adminMotionsNameEn')}</span>
              <input bind:value={held.nameEn} />
            </label>
            <label>
              <span class="kicker">{$t('adminMotionsOccasion')}</span>
              <select bind:value={held.occasion}>
                {#each MOTION_OCCASIONS as o (o)}
                  <option value={o}>{$t(OCCASION_KEY[o])}</option>
                {/each}
              </select>
            </label>
            <p class="hint">{$t('adminMotionsHint')}</p>
            <button type="button" class="btn" onclick={duplicate}>{$t('adminMotionsCopy')}</button>
            <button type="button" class="btn btn--drop" onclick={dropHeld}
              >{$t('adminMotionsDrop')}</button
            >
          </div>
        {:else}
          <p class="hint">{$t('adminMotionsReadOnly')}</p>
        {/if}

        <div class="stage" class:stage--along={along} style="--x:{spanX};--y:{spanY}">
          <div class="grid">
            {#each spots as cell (`${cell.x},${cell.y}`)}
              {@const isFrom = cell.x === from.x && cell.y === from.y}
              {@const isTo = cell.x === to.x && cell.y === to.y}
              <div class="cell" class:cell--used={isFrom || isTo}>
                {#key playKey}
                  {#if isFrom && strikerDto}
                    <span class="figure" style:--fit={aspectOf(strikerDto)} style={play?.striker}>
                      <BattleCard
                        card={strikerDto}
                        {frames}
                        owned={true}
                        transition={false}
                        interactive={false}
                      />
                    </span>
                  {:else if isTo && targetDto}
                    <span class="figure" style:--fit={aspectOf(targetDto)} style={play?.target}>
                      <BattleCard
                        card={targetDto}
                        {frames}
                        owned={true}
                        transition={false}
                        interactive={false}
                        hurt={woundOn && previewStruck === 'bruise' ? (previewHit?.remain ?? 1) : 1}
                        wearSeed={3}
                        struck={woundOn && playing ? previewStruck : null}
                        scrap={woundOn && playing && previewStruck === 'bruise' && previewHit
                          ? {
                              blow: previewHit.blow,
                              remain: previewHit.remain,
                              seed: previewHit.seed,
                            }
                          : null}
                      />
                    </span>
                  {/if}
                {/key}
              </div>
            {/each}
          </div>
          {#key playKey}
            <BattleMotionStage motes={play?.motes ?? []} />
          {/key}
          <svg
            class="traces"
            viewBox="0 0 {spanX} {spanY}"
            preserveAspectRatio="none"
            aria-hidden="true"
          >
            <line
              class="trace"
              class:trace--fresh={playing}
              x1={tracePts.x1}
              y1={tracePts.y1}
              x2={tracePts.x2}
              y2={tracePts.y2}
              vector-effect="non-scaling-stroke"
            />
          </svg>
        </div>

        <div class="stage-bar">
          <button type="button" class="btn" onclick={playTurn}>{$t('adminMotionsTurnPlay')}</button>
          <label class="inline">
            {$t('adminMotionsReach')}
            <input type="range" min="1" max={DEPTH - 1} bind:value={reach} onpointerup={bump} />
            <span class="tabular-nums">{reach}</span>
          </label>
          <label class="inline">
            <input type="checkbox" bind:checked={along} />
            {$t('adminMotionsAlong')}
          </label>
          <span class="muted">{span}{$t('adminMotionsMs')}</span>
        </div>
        <div class="stage-bar">
          <label class="inline">
            {$t('adminMotionsStrikerCard')}
            <select bind:value={strikerCard}>
              {#each cards as c (c.id)}
                <option value={c.id}>{$lang === 'ru' ? c.titleRu : c.titleEn}</option>
              {/each}
            </select>
          </label>
          <label class="inline">
            {$t('adminMotionsTargetCard')}
            <select bind:value={targetCard}>
              {#each cards as c (c.id)}
                <option value={c.id}>{$lang === 'ru' ? c.titleRu : c.titleEn}</option>
              {/each}
            </select>
          </label>
        </div>
      </section>

      <!-- ── Партитура, колодец, картинка ─────────────────────────────── -->
      <aside class="side">
        <p class="kicker">{$t('adminMotionsScore')}</p>
        <div class="score">
          {#each TRACKS as whom (whom)}
            {@const kin = held.gestures
              .map((g, i) => ({ g, i }))
              .filter((x) => x.g.whom === whom)}
            <div
              class="score-row"
              class:score-row--on={track === whom}
              onpointerdown={() => (track = whom)}
            >
              <span class="score-whom">{$t(WHOM_KEY[whom])}</span>
              <span class="score-lane">
                {#each kin as { g, i } (i)}
                  <button
                    type="button"
                    class="bar"
                    class:bar--on={gestureAt === i}
                    class:bar--slot={isSlot(g)}
                    style="left:{(g.at / MOTION_MS_MAX) * 100}%;width:{Math.max(
                      3,
                      (g.dur / MOTION_MS_MAX) * 100,
                    )}%"
                    onpointerdown={(e) => {
                      e.stopPropagation();
                      barDown(e, i, 'at');
                    }}
                    onpointermove={barMove}
                    onpointerup={barUp}
                    onpointercancel={barUp}
                    title="{barWord(g)} {g.at}–{g.at + g.dur}"
                  >
                    <span class="bar-label">{barWord(g)}</span>
                    <i
                      class="bar-grip"
                      onpointerdown={(e) => {
                        e.stopPropagation();
                        barDown(e, i, 'dur');
                      }}
                    ></i>
                  </button>
                {/each}
                {#if !kin.length}
                  <span class="score-empty">{$t('adminMotionsTrackEmpty')}</span>
                {/if}
              </span>
              {#if whom === 'flight' || whom === 'field'}
                <button
                  type="button"
                  class="slot-btn"
                  onclick={(e) => {
                    e.stopPropagation();
                    openSlot(whom);
                  }}>{$t('adminMotionsAddSlot')}</button
                >
              {/if}
            </div>
          {/each}
        </div>

        <p class="kicker mt">
          {track === 'flight' || track === 'field'
            ? $t('adminMotionsArt')
            : $t(WHOM_KEY[track])}
        </p>
        {#if track === 'flight' || track === 'field'}
          <p class="hint">{$t('adminMotionsNoBodyHere')}</p>
        {:else}
          {#each WELL as group (group.key)}
            <p class="well-label">{$t(group.key)}</p>
            <div class="well">
              {#each group.bodies as b (b)}
                <button
                  type="button"
                  class="chip"
                  class:chip--on={activeBody.move === b || activeBody.light === b}
                  onclick={() => putBody(b)}>{$t(BODY_KEY[b])}</button
                >
              {/each}
            </div>
          {/each}
          <p class="hint">{$t('adminMotionsLightNote')}</p>
        {/if}

        {#if gesture && (gesture.image || gesture.whom === 'flight' || gesture.whom === 'field')}
          <div class="art">
            <p class="kicker">{$t('adminMotionsArt')}</p>
            {#if gesture.image}
              <div
                class="art-face"
                style="background-image:url('{gesture.image}')"
              ></div>
            {:else}
              <p class="hint">{$t('adminMotionsSlotNote')}</p>
            {/if}
            <div class="row">
              <button type="button" class="btn" onclick={() => (picking = true)}
                >{$t('adminMotionsFromStore')}</button
              >
              <label class="btn">
                {uploading ? $t('adminMotionsUploading') : $t('adminMotionsUpload')}
                <input
                  type="file"
                  accept="image/*"
                  class="hidden"
                  onchange={(e) => {
                    const file = e.currentTarget.files?.[0];
                    if (file) void upload(file);
                    e.currentTarget.value = '';
                  }}
                />
              </label>
              {#if gesture.image}
                <button type="button" class="btn" onclick={() => setArt('')}
                  >{$t('adminMotionsClearSlot')}</button
                >
              {/if}
              <button
                type="button"
                class="btn btn--drop"
                onclick={() => dropGesture(gestureAt)}>×</button
              >
            </div>
            {#if gesture.image}
              <label class="block">
                {$t('adminMotionsFrames')}
                <input
                  type="number"
                  min="1"
                  max={MOTION_FRAMES_MAX}
                  bind:value={gesture.frames}
                  onchange={bump}
                />
              </label>
              <label class="block">
                {$t('adminMotionsSize')} <span class="tabular-nums">{gesture.size}</span>
                <input
                  type="range"
                  min="0"
                  max={GESTURE_SIZE_MAX}
                  bind:value={gesture.size}
                  onpointerdown={() => (pinned = true)}
                  onpointerup={() => {
                    pinned = false;
                    bump();
                  }}
                />
              </label>
              <div class="nudge">
                <label>
                  {$t('adminMotionsNudgeX')}
                  <input type="number" min={-GESTURE_NUDGE_MAX} max={GESTURE_NUDGE_MAX} bind:value={gesture.nudgeX} onchange={bump} />
                </label>
                <label>
                  {$t('adminMotionsNudgeY')}
                  <input type="number" min={-GESTURE_NUDGE_MAX} max={GESTURE_NUDGE_MAX} bind:value={gesture.nudgeY} onchange={bump} />
                </label>
              </div>
              <label class="block">
                {$t('adminMotionsTurn')}
                <select bind:value={gesture.turn} onchange={bump}>
                  {#each GESTURE_TURNS as v (v)}
                    <option value={v}>{$t(TURN_KEY[v])}</option>
                  {/each}
                </select>
              </label>
              <label class="block">
                {$t('adminMotionsFade')}
                <select bind:value={gesture.fade} onchange={bump}>
                  {#each GESTURE_FADES as v (v)}
                    <option value={v}>{$t(FADE_KEY[v])}</option>
                  {/each}
                </select>
              </label>
              <label class="block">
                {$t('adminMotionsLayer')} <span class="tabular-nums">{gesture.layer}</span>
                <input type="range" min="1" max={GESTURE_LAYERS} bind:value={gesture.layer} onpointerup={bump} />
              </label>
            {/if}
          </div>
        {:else if gesture && (isMove(gesture.body) || isLight(gesture.body))}
          <button type="button" class="btn btn--drop mt" onclick={() => dropGesture(gestureAt)}
            >× {$t(BODY_KEY[gesture.body])}</button
          >
        {/if}

        {#if faceKind === 'mine'}
          <p class="kicker mt">{$t('adminMotionsWornBy')}</p>
          {#if wearers.length}
            <ul class="worn">
              {#each wearers as w, i (`${w.name}-${w.occasion}-${i}`)}
                <li>{w.name} · {$t(OCCASION_KEY[w.occasion])}</li>
              {/each}
            </ul>
          {:else}
            <p class="hint">{$t('adminMotionsWornNone')}</p>
          {/if}
        {/if}

        {#if complaint}
          <p class="warn mt">{complaint}</p>
        {/if}
      </aside>
    </div>
  {/if}
</div>

{#if picking && gesture}
  <BattleAssetPicker
    role="motion"
    onPick={(asset) => {
      setArt(asset.url);
      picking = false;
    }}
    onClose={() => (picking = false)}
  />
{/if}

<style>
  .desk {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: #f8f1e7;
    color: #34251c;
  }

  .faces {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    padding: 0.6rem 0.85rem;
    border-bottom: 1px solid rgba(52, 37, 28, 0.12);
  }

  .face-col {
    min-width: 0;
    flex: 1;
  }

  .face-save {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-left: auto;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-top: 0.25rem;
  }

  .chip {
    padding: 0.2rem 0.5rem;
    font-size: 11px;
    border: 1px solid rgba(52, 37, 28, 0.18);
    background: transparent;
    color: inherit;
  }

  .chip--on {
    border-color: #c65f3c;
    background: rgba(198, 95, 60, 0.08);
  }

  .chip--add {
    color: #c65f3c;
  }

  .kicker {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.14em;
    color: #6f3b24;
  }

  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .stage-wrap {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: 0.85rem 1rem;
  }

  .side {
    width: 22rem;
    flex-shrink: 0;
    border-left: 1px solid rgba(52, 37, 28, 0.12);
    overflow-y: auto;
    padding: 0.75rem;
  }

  .names {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    align-items: end;
    margin-bottom: 0.75rem;
  }

  .names input,
  .names select,
  .stage-bar select,
  .art input,
  .art select {
    border: none;
    border-bottom: 1px solid rgba(52, 37, 28, 0.25);
    background: transparent;
    font-size: 13px;
    padding: 0.15rem 0;
    color: inherit;
  }

  .stage {
    position: relative;
    width: calc(var(--x) * 4.4rem);
    max-width: 100%;
    overflow: visible;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(var(--x), minmax(0, 1fr));
    gap: 2px;
  }

  .cell {
    position: relative;
    min-width: 0;
    aspect-ratio: 3 / 4;
    background: rgba(52, 37, 28, 0.04);
    border: 1px solid rgba(52, 37, 28, 0.08);
    overflow: visible;
  }

  .cell--used {
    background: rgba(52, 37, 28, 0.07);
  }

  .figure {
    display: grid;
    place-items: center;
    height: 100%;
    padding: 2px;
    overflow: visible;
    will-change: transform;
  }

  .figure > :global(.slot) {
    width: min(100%, calc(133.3333% * var(--fit, 0.714)));
  }

  .traces {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 3;
  }

  .trace {
    stroke: #6f3b24;
    stroke-width: 1;
    opacity: 0.28;
  }

  .trace--fresh {
    opacity: 0.85;
  }

  .stage-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.75rem;
    margin-top: 0.5rem;
    font-size: 11px;
  }

  .inline {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .btn {
    padding: 0.2rem 0.55rem;
    font-size: 11px;
    border: 1px solid rgba(52, 37, 28, 0.22);
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  .btn:disabled {
    opacity: 0.4;
  }

  .btn--drop {
    border-color: rgba(143, 47, 34, 0.35);
    color: #8f2f22;
  }

  .score {
    margin-top: 0.35rem;
  }

  .score-row {
    display: grid;
    grid-template-columns: 4.6rem 1fr auto;
    gap: 0.35rem;
    align-items: center;
    margin-bottom: 0.35rem;
    font-size: 11px;
  }

  .score-row--on .score-whom {
    color: #c65f3c;
  }

  .score-lane {
    position: relative;
    height: 1.35rem;
    background: rgba(52, 37, 28, 0.06);
  }

  .bar {
    position: absolute;
    top: 0;
    height: 100%;
    padding: 0;
    border: none;
    background: #6f3b24;
    color: #f8f1e7;
    overflow: hidden;
    cursor: grab;
  }

  .bar--on {
    background: #c65f3c;
  }

  .bar--slot {
    background: transparent;
    color: #6f3b24;
    outline: 1px dashed rgba(111, 59, 36, 0.5);
    outline-offset: -1px;
  }

  .bar-label {
    display: block;
    padding: 0 0.3rem;
    font-size: 9px;
    line-height: 1.35rem;
    white-space: nowrap;
    overflow: hidden;
  }

  .bar-grip {
    position: absolute;
    right: 0;
    top: 0;
    width: 6px;
    height: 100%;
    cursor: ew-resize;
    background: rgba(248, 241, 231, 0.35);
  }

  .score-empty {
    padding: 0 0.35rem;
    color: rgba(52, 37, 28, 0.4);
    line-height: 1.35rem;
  }

  .slot-btn {
    font-size: 10px;
    color: #c65f3c;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
  }

  .well {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-bottom: 0.35rem;
  }

  .well-label {
    margin: 0.45rem 0 0.15rem;
    font-size: 10px;
    color: rgba(52, 37, 28, 0.55);
  }

  .hint,
  .note,
  .muted {
    font-size: 10px;
    line-height: 1.35;
    color: rgba(52, 37, 28, 0.55);
  }

  .warn {
    font-size: 10px;
    color: #c65f3c;
  }

  .art-face {
    height: 3.5rem;
    margin: 0.35rem 0;
    background: rgba(52, 37, 28, 0.06) center / contain no-repeat;
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .nudge {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.4rem;
    font-size: 11px;
  }

  .block {
    display: block;
    margin-top: 0.4rem;
    font-size: 11px;
  }

  .mt {
    margin-top: 0.85rem;
  }

  .worn {
    margin-top: 0.25rem;
    font-size: 11px;
    line-height: 1.45;
  }

  .hidden {
    display: none;
  }
</style>
