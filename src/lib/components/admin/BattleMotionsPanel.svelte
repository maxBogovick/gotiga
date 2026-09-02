<script lang="ts">
  // Стол движений.
  //
  // Три части, и порядок не случаен: слева ЯЩИК (что вообще есть у дома),
  // посередине СЦЕНА (как это выглядит), справа ЖЕСТ В РУКЕ (что именно сейчас
  // правится). Ровно как на столе рамок, и по той же причине: хранитель уже
  // выучил эту раскладку, и вторая раскладка для второй вещи — это вторая
  // вещь, которую надо учить.
  //
  // Сцена показывает движение на НАСТОЯЩИХ картах в настоящих клетках, тем же
  // `stage()`, которым его играет комната. Не квадратик и не схема: превью,
  // которое считает иначе, чем комната, — это превью, которое однажды соврёт
  // (`CLAUDE.md`), и на движении это заметят позже всего.
  import { onMount, untrack } from 'svelte';
  import { api } from '$lib/api';
  import { t, lang } from '$lib/i18n';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import BattleMotionStage from '$lib/components/BattleMotionStage.svelte';
  import BattleAssetPicker from '$lib/components/admin/BattleAssetPicker.svelte';
  import {
    GESTURE_BODIES,
    GESTURE_FADES,
    GESTURE_LAYERS,
    GESTURE_NUDGE_MAX,
    GESTURE_SIZE_MAX,
    GESTURE_TURNS,
    GESTURE_WHOMS,
    DEFAULT_ASPECT,
    frameForCard,
    GESTURES_MAX,
    MOTION_FRAMES_MAX,
    MOTION_MS_MAX,
    MOTION_OCCASIONS,
    MOTIONS_MAX,
    completeSlices,
    motionSpan,
    motionTitle,
    newGesture,
    newMotion,
    stage,
    type Staged,
  } from '$lib/battles';
  import type {
    BattleCard as BattleCardDto,
    BattleFrame,
    GestureBody,
    GestureFade,
    GestureTurn,
    GestureWhom,
    Motion,
    MotionGesture,
    MotionOccasion,
  } from '$lib/types/api';
  import type { TranslationKey } from '$lib/i18n';

  let { flash }: { flash?: (text: string) => void } = $props();

  let motions = $state<Motion[]>([]);
  let cards = $state<BattleCardDto[]>([]);
  let frames = $state<BattleFrame[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let complaint = $state('');

  /** Каким сохранённый свод пришёл в последний раз. Всё, что от него
   *  отличается, — несохранённое, и об этом говорится вслух: вечер работы
   *  теряется молча ровно один раз. */
  let stored = $state('');
  let dirty = $derived(stored !== '' && JSON.stringify(motions) !== stored);

  let heldId = $state<string | null>(null);
  let held = $derived(motions.find((m) => m.id === heldId) ?? null);
  /** Какой жест правится. Индекс, а не ссылка: список перестраивается. */
  let gestureAt = $state(0);
  let gesture = $derived(held?.gestures[gestureAt] ?? null);

  // ── Сцена ────────────────────────────────────────────────────────────────
  //
  // Малое поле 3×4, а не 3×6: движение показывается между двумя телами, и
  // четырёх рядов хватает на самую дальнюю пару, какая бывает интересна.
  // Отношение клетки — то же 3:4, что на доске, иначе угол стрелы здесь и
  // в комнате разошёлся бы.
  const SPAN_X = 3;
  const SPAN_Y = 4;
  let reach = $state(3);
  let sideways = $state(false);
  let strikerCard = $state<string>('');
  let targetCard = $state<string>('');

  let from = $derived({ x: sideways ? 0 : 1, y: SPAN_Y - 1 });
  let to = $derived({ x: 1, y: SPAN_Y - 1 - Math.min(reach, SPAN_Y - 1) });

  /** Снимок разложенного движения. Меняется только по нажатию «показать»:
   *  сцена, переигрывающая себя на каждую правку числа, — это сцена, на
   *  которой нельзя настроить число. */
  let play = $state<Staged | null>(null);
  let playKey = $state(0);
  let playing = $state(false);
  let timer: ReturnType<typeof setTimeout> | null = null;

  const dtoOf = (id: string) => cards.find((c) => c.id === id) ?? null;

  /** Отношение сторон рамки этой карты. Клетке надо знать его ЗАРАНЕЕ — см.
   *  правило ширины ниже. */
  const aspectOf = (dto: BattleCardDto) =>
    frameForCard(dto, frames).aspect || DEFAULT_ASPECT;
  let strikerDto = $derived(dtoOf(strikerCard));
  let targetDto = $derived(dtoOf(targetCard));

  function show() {
    if (!held) return;
    if (timer) clearTimeout(timer);
    // `calm: false` здесь нарочно: стол — это стол, и хранитель, у которого
    // в системе выключены движения, обязан всё-таки видеть, что он собирает.
    // Комната при той же настройке не покажет ничего, и это разные вещи.
    play = stage(held, from, to, { spanX: SPAN_X, spanY: SPAN_Y, along: false, calm: false });
    playKey += 1;
    playing = true;
    timer = setTimeout(() => (playing = false), Math.max(200, motionSpan(held)));
  }

  function stopShowing() {
    if (timer) clearTimeout(timer);
    playing = false;
    play = null;
  }

  // ── Ящик ─────────────────────────────────────────────────────────────────

  function add(occasion: MotionOccasion) {
    if (motions.length >= MOTIONS_MAX) return;
    const born = newMotion(occasion);
    born.nameRu = $t('adminMotionsUntitled');
    born.nameEn = 'New motion';
    motions = [...motions, born];
    heldId = born.id;
    gestureAt = 0;
    stopShowing();
  }

  function duplicate(motion: Motion) {
    if (motions.length >= MOTIONS_MAX) return;
    const twin: Motion = {
      ...structuredClone($state.snapshot(motion)),
      id: newMotion().id,
      nameRu: `${motion.nameRu} ·`,
      nameEn: `${motion.nameEn} ·`,
    };
    motions = [...motions, twin];
    heldId = twin.id;
    gestureAt = 0;
    stopShowing();
  }

  function drop(id: string) {
    motions = motions.filter((m) => m.id !== id);
    if (heldId === id) heldId = motions[0]?.id ?? null;
    stopShowing();
  }

  // ── Жесты ────────────────────────────────────────────────────────────────

  function addGesture(whom: GestureWhom) {
    if (!held || held.gestures.length >= GESTURES_MAX) return;
    held.gestures = [...held.gestures, newGesture(whom)];
    gestureAt = held.gestures.length - 1;
  }

  function dropGesture(i: number) {
    if (!held) return;
    held.gestures = held.gestures.filter((_, k) => k !== i);
    gestureAt = Math.max(0, Math.min(gestureAt, held.gestures.length - 1));
  }

  /** Список — это порядок наложения. Тот же приём, что на столе рамок: два
   *  жеста, деливших слой, ложились в порядке, которого никто не выбирал. */
  function shift(i: number, by: number) {
    if (!held) return;
    const to2 = i + by;
    if (to2 < 0 || to2 >= held.gestures.length) return;
    const list = [...held.gestures];
    [list[i], list[to2]] = [list[to2], list[i]];
    held.gestures = list;
    gestureAt = to2;
  }

  /** Летящему нечем шевелиться. Правится здесь же, а не только на сервере:
   *  форма, предлагающая невозможное, — это форма, которая учит неправде. */
  function setWhom(g: MotionGesture, whom: GestureWhom) {
    g.whom = whom;
    if (whom === 'flight' || whom === 'field') g.body = 'none';
    else if (g.body === 'none' && !g.image) g.body = 'lunge';
  }

  // ── Картинка жеста ───────────────────────────────────────────────────────

  let picking = $state(false);
  let uploading = $state(false);

  async function upload(file: File) {
    if (!gesture) return;
    uploading = true;
    try {
      // Тот же загрузчик, что у деталей рам: лоссовый WebP с альфой. Второй
      // загрузчик того же самого был бы вторым местом, где однажды разойдутся
      // настройки (`BATTLE-MOTION.md` §5).
      const got = await api.adminUploadBattleFrameArt(file);
      gesture.image = got.url;
    } catch (e) {
      complaint = String(e);
    } finally {
      uploading = false;
    }
  }

  // ── Чтение и запись ──────────────────────────────────────────────────────

  async function load() {
    loading = true;
    try {
      const [moving, deck, dressing] = await Promise.all([
        api.getBattleMotions(),
        api.adminListBattleCards(),
        api.getBattleFrames(),
      ]);
      motions = moving.motions;
      cards = deck;
      frames = (dressing.frames ?? []).map(completeSlices);
      stored = JSON.stringify(motions);
      heldId = motions[0]?.id ?? null;
      strikerCard = strikerCard || (cards[0]?.id ?? '');
      targetCard = targetCard || (cards[1]?.id ?? cards[0]?.id ?? '');
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
      if (heldId && !motions.some((m) => m.id === heldId)) heldId = motions[0]?.id ?? null;
      // Сервер выбрасывает пустое и подрезает длинное. Сказать об этом надо
      // словами: свод, вернувшийся короче отправленного, — это вечер, который
      // хранитель иначе потеряет молча.
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
    return () => {
      window.removeEventListener('beforeunload', guard);
      if (timer) clearTimeout(timer);
    };
  });

  // ── Слова ────────────────────────────────────────────────────────────────

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

  const gestureWord = (g: MotionGesture) =>
    `${$t(WHOM_KEY[g.whom])} · ${g.image ? $t('adminMotionsHasArt') : $t(BODY_KEY[g.body])}`;

  let span = $derived(motionSpan(held));
  const cells = Array.from({ length: SPAN_X * SPAN_Y }, (_, i) => ({
    x: i % SPAN_X,
    y: Math.floor(i / SPAN_X),
  }));
</script>

<div class="flex h-full min-h-0 bg-[#f8f1e7] text-[#34251c]">
  <!-- ── Ящик ───────────────────────────────────────────────────────────── -->
  <aside class="w-64 shrink-0 border-r border-[#34251c]/12 flex flex-col min-h-0">
    <div class="px-3 py-2 border-b border-[#34251c]/10">
      <p class="text-[10px] uppercase tracking-[0.16em] text-[#6f3b24]">
        {$t('adminMotionsDrawer')}
      </p>
      <p class="mt-1 text-[11px] leading-snug text-[#34251c]/65">
        {$t('adminMotionsDrawerNote')}
      </p>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto">
      {#each MOTION_OCCASIONS as occasion (occasion)}
        {@const kin = motions.filter((m) => m.occasion === occasion)}
        <div class="border-b border-[#34251c]/8">
          <div class="flex items-center gap-2 px-3 py-1.5">
            <span class="text-[10px] uppercase tracking-[0.14em] text-[#6f3b24]"
              >{$t(OCCASION_KEY[occasion])}</span
            >
            <button
              type="button"
              class="ml-auto text-[11px] text-[#c65f3c] hover:underline"
              onclick={() => add(occasion)}>+</button
            >
          </div>
          {#each kin as motion (motion.id)}
            <button
              type="button"
              onclick={() => {
                heldId = motion.id;
                gestureAt = 0;
                stopShowing();
              }}
              class="w-full text-left px-3 py-1.5 text-[12px] border-l-2 {heldId === motion.id
                ? 'border-[#c65f3c] bg-[#c65f3c]/[0.07]'
                : 'border-transparent hover:bg-[#34251c]/[0.04]'}"
            >
              {motionTitle(motion, $lang)}
              <span class="ml-1 text-[10px] text-[#34251c]/45"
                >{motionSpan(motion)}{$t('adminMotionsMs')}</span
              >
            </button>
          {/each}
        </div>
      {/each}
      {#if loading}
        <p class="px-3 py-3 text-[11px] text-[#34251c]/50">{$t('adminMotionsLoading')}</p>
      {/if}
    </div>

    <div class="px-3 py-2 border-t border-[#34251c]/10 flex items-center gap-2">
      <button
        type="button"
        disabled={saving || !dirty}
        onclick={save}
        class="px-3 py-1 text-[11px] uppercase tracking-[0.14em] border border-[#34251c]/25 disabled:opacity-40"
        >{$t('adminMotionsSave')}</button
      >
      {#if dirty}
        <span class="text-[10px] text-[#c65f3c]">{$t('adminMotionsUnsaved')}</span>
      {/if}
    </div>
  </aside>

  <!-- ── Сцена ──────────────────────────────────────────────────────────── -->
  <section class="flex-1 min-w-0 min-h-0 overflow-y-auto p-4">
    {#if !held}
      <p class="text-[12px] text-[#34251c]/55">{$t('adminMotionsNothingHeld')}</p>
    {:else}
      <div class="flex flex-wrap items-end gap-3">
        <label class="block">
          <span class="block text-[10px] uppercase tracking-[0.14em] text-[#6f3b24]"
            >{$t('adminMotionsNameRu')}</span
          >
          <input
            bind:value={held.nameRu}
            class="mt-0.5 w-44 border-b border-[#34251c]/25 bg-transparent text-[13px] py-0.5"
          />
        </label>
        <label class="block">
          <span class="block text-[10px] uppercase tracking-[0.14em] text-[#6f3b24]"
            >{$t('adminMotionsNameEn')}</span
          >
          <input
            bind:value={held.nameEn}
            class="mt-0.5 w-44 border-b border-[#34251c]/25 bg-transparent text-[13px] py-0.5"
          />
        </label>
        <label class="block">
          <span class="block text-[10px] uppercase tracking-[0.14em] text-[#6f3b24]"
            >{$t('adminMotionsOccasion')}</span
          >
          <select
            bind:value={held.occasion}
            class="mt-0.5 border-b border-[#34251c]/25 bg-transparent text-[13px] py-0.5"
          >
            {#each MOTION_OCCASIONS as o (o)}
              <option value={o}>{$t(OCCASION_KEY[o])}</option>
            {/each}
          </select>
        </label>
        <button
          type="button"
          onclick={() => duplicate(held)}
          class="px-2 py-1 text-[11px] border border-[#34251c]/20">{$t('adminMotionsCopy')}</button
        >
        <button
          type="button"
          onclick={() => drop(held.id)}
          class="px-2 py-1 text-[11px] border border-[#8f2f22]/30 text-[#8f2f22]"
          >{$t('adminMotionsDrop')}</button
        >
      </div>

      <div class="mt-4 flex flex-wrap gap-6">
        <!-- Настоящие карты в настоящих клетках. Сцена, показывающая
             квадратики, врёт про размер, про рамку и про угол. -->
        <div>
          <div class="stage" style="--x:{SPAN_X};--y:{SPAN_Y}">
            <div class="grid">
              {#each cells as cell (`${cell.x},${cell.y}`)}
                {@const isFrom = cell.x === from.x && cell.y === from.y}
                {@const isTo = cell.x === to.x && cell.y === to.y}
                <div class="cell" class:cell--used={isFrom || isTo}>
                  {#key playKey}
                    {#if isFrom && strikerDto}
                      <span
                        class="figure"
                        style:--fit={aspectOf(strikerDto)}
                        style={play?.striker}
                      >
                        <BattleCard
                          card={strikerDto}
                          {frames}
                          owned={true}
                          transition={false}
                          interactive={false}
                        />
                      </span>
                    {:else if isTo && targetDto}
                      <span
                        class="figure"
                        style:--fit={aspectOf(targetDto)}
                        style={play?.target}
                      >
                        <BattleCard
                          card={targetDto}
                          {frames}
                          owned={true}
                          transition={false}
                          interactive={false}
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
          </div>

          <div class="mt-2 flex flex-wrap items-center gap-3 text-[11px]">
            <button
              type="button"
              onclick={show}
              class="px-3 py-1 uppercase tracking-[0.14em] border border-[#34251c]/25"
              >{playing ? $t('adminMotionsPlaying') : $t('adminMotionsShow')}</button
            >
            <label class="flex items-center gap-1">
              {$t('adminMotionsReach')}
              <input type="range" min="1" max={SPAN_Y - 1} bind:value={reach} class="w-24" />
              <span class="tabular-nums">{reach}</span>
            </label>
            <label class="flex items-center gap-1">
              <input type="checkbox" bind:checked={sideways} />
              {$t('adminMotionsSideways')}
            </label>
            <span class="text-[#34251c]/50">{span}{$t('adminMotionsMs')}</span>
          </div>

          <div class="mt-2 flex flex-wrap gap-3 text-[11px]">
            <label class="flex items-center gap-1">
              {$t('adminMotionsStrikerCard')}
              <select bind:value={strikerCard} class="border-b border-[#34251c]/25 bg-transparent">
                {#each cards as c (c.id)}
                  <option value={c.id}>{$lang === 'ru' ? c.titleRu : c.titleEn}</option>
                {/each}
              </select>
            </label>
            <label class="flex items-center gap-1">
              {$t('adminMotionsTargetCard')}
              <select bind:value={targetCard} class="border-b border-[#34251c]/25 bg-transparent">
                {#each cards as c (c.id)}
                  <option value={c.id}>{$lang === 'ru' ? c.titleRu : c.titleEn}</option>
                {/each}
              </select>
            </label>
          </div>
        </div>

        <!-- Полоса времени: где какой жест стоит. Не украшение — без неё
             «на 220-й» и «длится 160» остаются двумя числами, между которыми
             человек считает в уме. -->
        <div class="min-w-[16rem] flex-1">
          <p class="text-[10px] uppercase tracking-[0.14em] text-[#6f3b24]">
            {$t('adminMotionsGestures')}
          </p>
          <ul class="mt-1 space-y-1">
            {#each held.gestures as g, i (i)}
              <li>
                <button
                  type="button"
                  onclick={() => (gestureAt = i)}
                  class="w-full text-left px-2 py-1 border {gestureAt === i
                    ? 'border-[#c65f3c] bg-[#c65f3c]/[0.06]'
                    : 'border-[#34251c]/12'}"
                >
                  <span class="flex items-center gap-2 text-[11px]">
                    <span>{gestureWord(g)}</span>
                    <span class="ml-auto tabular-nums text-[#34251c]/45"
                      >{g.at}–{g.at + g.dur}</span
                    >
                  </span>
                  <span class="mt-1 block h-1 bg-[#34251c]/10">
                    <span
                      class="block h-full bg-[#6f3b24]"
                      style="margin-left:{span ? (g.at / span) * 100 : 0}%;width:{span
                        ? (g.dur / span) * 100
                        : 0}%"
                    ></span>
                  </span>
                </button>
              </li>
            {/each}
          </ul>
          <div class="mt-2 flex flex-wrap gap-2 text-[11px]">
            {#each GESTURE_WHOMS as w (w)}
              <button
                type="button"
                onclick={() => addGesture(w)}
                disabled={held.gestures.length >= GESTURES_MAX}
                class="px-2 py-1 border border-[#34251c]/20 disabled:opacity-40"
                >+ {$t(WHOM_KEY[w])}</button
              >
            {/each}
          </div>
        </div>
      </div>
    {/if}
  </section>

  <!-- ── Жест в руке ────────────────────────────────────────────────────── -->
  <aside class="w-72 shrink-0 border-l border-[#34251c]/12 overflow-y-auto p-3">
    {#if !gesture || !held}
      <p class="text-[11px] text-[#34251c]/50">{$t('adminMotionsNoGesture')}</p>
    {:else}
      <div class="flex items-center gap-2">
        <p class="text-[10px] uppercase tracking-[0.16em] text-[#6f3b24]">
          {$t('adminMotionsGesture')}
        </p>
        <button
          type="button"
          class="ml-auto text-[11px]"
          onclick={() => shift(gestureAt, -1)}>↑</button
        >
        <button type="button" class="text-[11px]" onclick={() => shift(gestureAt, 1)}>↓</button>
        <button
          type="button"
          class="text-[11px] text-[#8f2f22]"
          onclick={() => dropGesture(gestureAt)}>×</button
        >
      </div>

      <label class="mt-3 block text-[11px]">
        {$t('adminMotionsWhom')}
        <select
          value={gesture.whom}
          onchange={(e) => setWhom(gesture, e.currentTarget.value as GestureWhom)}
          class="mt-0.5 w-full border border-[#34251c]/20 bg-transparent px-1 py-0.5 text-[12px]"
        >
          {#each GESTURE_WHOMS as w (w)}
            <option value={w}>{$t(WHOM_KEY[w])}</option>
          {/each}
        </select>
      </label>

      <label class="mt-2 block text-[11px]">
        {$t('adminMotionsBody')}
        <select
          bind:value={gesture.body}
          disabled={gesture.whom === 'flight' || gesture.whom === 'field'}
          class="mt-0.5 w-full border border-[#34251c]/20 bg-transparent px-1 py-0.5 text-[12px] disabled:opacity-40"
        >
          {#each GESTURE_BODIES as b (b)}
            <option value={b}>{$t(BODY_KEY[b])}</option>
          {/each}
        </select>
      </label>
      {#if gesture.whom === 'flight' || gesture.whom === 'field'}
        <p class="mt-1 text-[10px] leading-snug text-[#34251c]/55">
          {$t('adminMotionsNoBodyHere')}
        </p>
      {/if}

      <div class="mt-3 border-t border-[#34251c]/10 pt-2">
        <p class="text-[10px] uppercase tracking-[0.14em] text-[#6f3b24]">
          {$t('adminMotionsArt')}
        </p>
        {#if gesture.image}
          <div class="mt-1 h-16 bg-[#34251c]/5 bg-contain bg-center bg-no-repeat"
            style="background-image:url('{gesture.image}')"></div>
        {/if}
        <div class="mt-1 flex flex-wrap gap-2 text-[11px]">
          <button
            type="button"
            onclick={() => (picking = true)}
            class="px-2 py-1 border border-[#34251c]/20">{$t('adminMotionsFromStore')}</button
          >
          <label class="px-2 py-1 border border-[#34251c]/20 cursor-pointer">
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
            <button
              type="button"
              onclick={() => (gesture.image = '')}
              class="px-2 py-1 border border-[#34251c]/20">{$t('adminMotionsClearArt')}</button
            >
          {/if}
        </div>

        {#if gesture.image}
          <label class="mt-2 block text-[11px]">
            {$t('adminMotionsFrames')}
            <input
              type="number"
              min="1"
              max={MOTION_FRAMES_MAX}
              bind:value={gesture.frames}
              class="mt-0.5 w-20 border border-[#34251c]/20 bg-transparent px-1 py-0.5 text-[12px]"
            />
          </label>
          <p class="mt-1 text-[10px] leading-snug text-[#34251c]/55">
            {$t('adminMotionsFramesNote')}
          </p>

          <label class="mt-2 block text-[11px]">
            {$t('adminMotionsSize')} <span class="tabular-nums">{gesture.size}</span>
            <input
              type="range"
              min="0"
              max={GESTURE_SIZE_MAX}
              bind:value={gesture.size}
              class="w-full"
            />
          </label>
          <div class="mt-1 grid grid-cols-2 gap-2 text-[11px]">
            <label>
              {$t('adminMotionsNudgeX')}
              <input
                type="number"
                min={-GESTURE_NUDGE_MAX}
                max={GESTURE_NUDGE_MAX}
                bind:value={gesture.nudgeX}
                class="mt-0.5 w-full border border-[#34251c]/20 bg-transparent px-1 py-0.5"
              />
            </label>
            <label>
              {$t('adminMotionsNudgeY')}
              <input
                type="number"
                min={-GESTURE_NUDGE_MAX}
                max={GESTURE_NUDGE_MAX}
                bind:value={gesture.nudgeY}
                class="mt-0.5 w-full border border-[#34251c]/20 bg-transparent px-1 py-0.5"
              />
            </label>
          </div>

          <label class="mt-2 block text-[11px]">
            {$t('adminMotionsTurn')}
            <select
              bind:value={gesture.turn}
              class="mt-0.5 w-full border border-[#34251c]/20 bg-transparent px-1 py-0.5 text-[12px]"
            >
              {#each GESTURE_TURNS as v (v)}
                <option value={v}>{$t(TURN_KEY[v])}</option>
              {/each}
            </select>
          </label>

          <label class="mt-2 block text-[11px]">
            {$t('adminMotionsFade')}
            <select
              bind:value={gesture.fade}
              class="mt-0.5 w-full border border-[#34251c]/20 bg-transparent px-1 py-0.5 text-[12px]"
            >
              {#each GESTURE_FADES as v (v)}
                <option value={v}>{$t(FADE_KEY[v])}</option>
              {/each}
            </select>
          </label>

          <label class="mt-2 block text-[11px]">
            {$t('adminMotionsLayer')} <span class="tabular-nums">{gesture.layer}</span>
            <input type="range" min="1" max={GESTURE_LAYERS} bind:value={gesture.layer} class="w-full" />
          </label>
        {/if}
      </div>

      <div class="mt-3 border-t border-[#34251c]/10 pt-2">
        <p class="text-[10px] uppercase tracking-[0.14em] text-[#6f3b24]">
          {$t('adminMotionsWhen')}
        </p>
        <label class="mt-1 block text-[11px]">
          {$t('adminMotionsAt')} <span class="tabular-nums">{gesture.at}</span>
          <input type="range" min="0" max={MOTION_MS_MAX} step="10" bind:value={gesture.at} class="w-full" />
        </label>
        <label class="mt-1 block text-[11px]">
          {$t('adminMotionsDur')} <span class="tabular-nums">{gesture.dur}</span>
          <input type="range" min="0" max={MOTION_MS_MAX} step="10" bind:value={gesture.dur} class="w-full" />
        </label>
        <p class="mt-1 text-[10px] leading-snug text-[#34251c]/55">
          {$t('adminMotionsWhenNote')}
        </p>
      </div>
    {/if}

    {#if complaint}
      <p class="mt-3 text-[11px] text-[#8f2f22]">{complaint}</p>
    {/if}
  </aside>
</div>

{#if picking && gesture}
  <BattleAssetPicker
    role="motion"
    onPick={(asset) => {
      gesture.image = asset.url;
      picking = false;
    }}
    onClose={() => (picking = false)}
  />
{/if}

<style>
  /* Клетка 3:4 — то же отношение, что на доске. Иначе угол, под которым здесь
     летит стрела, разошёлся бы с тем, под каким она летит в комнате. */
  .stage {
    position: relative;
    width: calc(var(--x) * 6.5rem);
    max-width: 100%;
  }

  /* `minmax(0, 1fr)`, а не `1fr`: у `1fr` нижняя граница — минимальная ширина
     содержимого, и карта, оказавшаяся шире клетки, раздвинула бы колонку, а за
     ней стол. Второй замок к правилу ширины ниже: круг разорван там, а здесь
     ему не дают дотянуться до раскладки, даже если однажды вернётся. */
  .grid {
    display: grid;
    grid-template-columns: repeat(var(--x), minmax(0, 1fr));
    gap: 2px;
  }

  /* Не `flex`, и это не мелочь: во флексе `.figure` становится элементом,
     сжимаемым по содержимому, а процент внутри неё разрешается тогда против
     сжатой ширины — то есть против нуля. В сцене клетка тоже обычный блок,
     и `.figure` заполняет её как блочная сетка. */
  .cell {
    position: relative;
    min-width: 0;
    aspect-ratio: 3 / 4;
    background: rgba(52, 37, 28, 0.04);
    border: 1px solid rgba(52, 37, 28, 0.08);
  }

  .cell--used {
    background: rgba(52, 37, 28, 0.07);
  }

  .figure {
    display: grid;
    place-items: center;
    height: 100%;
    padding: 2px;
    will-change: transform;
  }

  /* Ширина СЧИТАЕТСЯ, а не выводится из содержимого — то же правило и та же
     причина, что в сцене (`BattleScene.svelte`, `.figure > .slot`). Карта
     объявила себя контейнером и меряет своё содержимое в `cqi`; позволить
     такому боксу мерить ширину по содержимому — значит замкнуть круг: ширина
     из содержимого, содержимое из ширины. Круг проворачивается на каждом
     пересчёте, то есть на каждом наведении, и стол расползался. Процент от
     клетки его разрывает: клетка 3:4, значит её высота — 133⅓% ширины, и
     карта ростом в клетку шириной `133⅓% × отношение рамки`. */
  .figure > :global(.slot) {
    width: min(100%, calc(133.3333% * var(--fit, 0.714)));
  }
</style>
