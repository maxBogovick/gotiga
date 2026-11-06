<script lang="ts">
  // Стол маршрутов забега.
  //
  // Маршрут — цепочка волн. Поле, местность, правила и врагов с начала волна
  // берёт у боя-основы (вкладка «Поле боя»); здесь — расписание подкрепления,
  // условие победы, число карт, награда и проба ботами по трём ступеням отрядов.
  // Числа и рубильник режима лежат внизу.
  import { onMount, onDestroy } from 'svelte';
  import { t, lang, localized } from '$lib/i18n';
  import { api, ApiError } from '$lib/api';
  import RouteEntryPicker from '$lib/components/admin/RouteEntryPicker.svelte';
  import type {
    AdminRoute,
    AdminWave,
    BattleCard,
    BattleChallenge,
    BattleRace,
    RouteProbeState,
    RunSettings,
  } from '$lib/types/api';

  let { flash } = $props<{ flash: (message: string, ms?: number) => void }>();

  let routes = $state<AdminRoute[]>([]);
  let challenges = $state<BattleChallenge[]>([]);
  let cards = $state<BattleCard[]>([]);
  let races = $state<BattleRace[]>([]);
  let settings = $state<RunSettings | null>(null);
  let draft = $state<AdminRoute | null>(null);
  /** Что показано справа: маршрут или настройки режима. */
  let view = $state<'route' | 'settings'>('route');
  /** У какой волны раскрыто подкрепление. */
  let openWave = $state<number | null>(null);
  let busy = $state(false);
  let probe = $state<RouteProbeState | null>(null);
  let probeTimer: ReturnType<typeof setInterval> | null = null;

  const blank = (): AdminRoute => ({
    id: null,
    slug: '',
    titleEn: '',
    titleRu: '',
    titleUk: '',
    titleEs: '',
    titleDe: '',
    titleIt: '',
    guestWaves: null,
    status: 'draft',
    sortOrder: null,
    waves: [],
    warnings: [],
  });
  const blankWave = (n: number): AdminWave => ({
    waveNo: n,
    challengeId: challenges[0]?.id ?? '',
    reinforcements: [],
    goal: null,
    minCards: 1,
    maxCards: 3,
    rewardDust: 10,
    raceHint: null,
  });

  onMount(async () => {
    try {
      [routes, challenges, cards, races, settings] = await Promise.all([
        api.adminListBattleRoutes(),
        api.adminListBattleChallenges(),
        api.getBattleEtudeCards().then(async (gear) => [...(await api.getBattleCards()), ...gear]),
        api.getBattleRaces(),
        api.adminGetRunSettings(),
      ]);
      if (routes.length) open(routes[0]);
    } catch (err) {
      flash(err instanceof ApiError ? err.message : String(err));
    }
  });
  onDestroy(() => {
    if (probeTimer) clearInterval(probeTimer);
  });

  /** Как маршрут лежит на сервере: проба играет его, а не то, что на экране. */
  let savedText = $state('');
  const snapshotOf = (r: AdminRoute) => JSON.stringify({ ...r, warnings: [] });
  const dirty = $derived(!!draft && snapshotOf($state.snapshot(draft) as AdminRoute) !== savedText);

  const open = (r: AdminRoute | null) => {
    view = 'route';
    openWave = null;
    draft = r ? structuredClone($state.snapshot(r)) : blank();
    savedText = r ? snapshotOf(r) : '';
    probe = null;
  };

  async function guarded(work: () => Promise<void>) {
    if (busy) return;
    busy = true;
    try {
      await work();
    } catch (err) {
      const text = err instanceof ApiError ? err.message : '';
      flash(
        text.includes('retire it')
          ? $t('adminRouteDeleteRefused')
          : text.includes('active runs')
            ? $t('adminRouteShrinkRefused')
            : text || String(err),
      );
    } finally {
      busy = false;
    }
  }

  const save = () =>
    guarded(async () => {
      if (!draft) return;
      const saved = await api.adminSaveBattleRoute($state.snapshot(draft) as AdminRoute);
      routes = await api.adminListBattleRoutes();
      draft = structuredClone(saved);
      savedText = snapshotOf(saved);
      flash($t('adminRouteSaved'));
    });

  const remove = () =>
    guarded(async () => {
      if (!draft?.id) return;
      await api.adminDeleteBattleRoute(draft.id);
      routes = await api.adminListBattleRoutes();
      draft = null;
    });

  const runProbe = () =>
    guarded(async () => {
      if (!draft?.id) return;
      const { id } = await api.adminStartRouteProbe(draft.id);
      probe = { state: 'running' };
      if (probeTimer) clearInterval(probeTimer);
      probeTimer = setInterval(async () => {
        try {
          const state = await api.adminReadRouteProbe(id);
          probe = state;
          if (state.state === 'done' && probeTimer) {
            clearInterval(probeTimer);
            probeTimer = null;
          }
        } catch {
          if (probeTimer) clearInterval(probeTimer);
          probeTimer = null;
        }
      }, 3000);
    });

  const saveSettings = () =>
    guarded(async () => {
      if (!settings) return;
      settings = await api.adminSaveRunSettings($state.snapshot(settings) as RunSettings);
      flash($t('adminRunSettingsSaved'));
    });

  function addWave() {
    if (draft && draft.waves.length < 12) draft.waves.push(blankWave(draft.waves.length + 1));
  }
  function dropWave() {
    if (openWave === draft?.waves.length) openWave = null;
    draft?.waves.pop();
  }

  type GoalKind = 'none' | 'holdRounds' | 'defendStructure' | 'killTarget';
  const goalKind = (w: AdminWave): GoalKind => w.goal?.kind ?? 'none';
  function setGoal(w: AdminWave, kind: GoalKind) {
    w.goal =
      kind === 'none'
        ? null
        : kind === 'holdRounds'
          ? { kind, rounds: 6 }
          : kind === 'defendStructure'
            ? { kind }
            : { kind, name: '' };
  }

  const fmt = (key: Parameters<typeof $t>[0], vars: Record<string, string | number>) =>
    Object.entries(vars).reduce((s, [k, v]) => s.replace(`{${k}}`, String(v)), $t(key));

  function warning(code: string): string {
    const [kind, a, b] = code.split(':');
    switch (kind) {
      case 'noBoss':
        return fmt('adminRouteWarnNoBoss', { n: a, name: b });
      case 'defendNeedsStructure':
        return fmt('adminRouteWarnDefend', { n: a });
      case 'entranceBlocked':
        return fmt('adminRouteWarnEntrance', { n: a, name: b });
      case 'plainWave':
        return fmt('adminRouteWarnPlain', { n: a });
      case 'trophyNotPublished':
        return fmt('adminRouteWarnTrophyPublished', { name: a });
      case 'trophyNoPrice':
        return fmt('adminRouteWarnTrophyPrice', { name: a });
      default:
        return code;
    }
  }

  const challengeTitle = (c: BattleChallenge) => localized(c, 'title', $lang);
  const routeTitle = (r: AdminRoute) => r.titleRu || r.titleEn || r.slug;

  /** Попала ли ступень в цель из задания. */
  function inTarget(tier: string, avg: number, won: number, played: number): boolean {
    if (tier === 'weakest') return avg >= 1 && avg <= 2;
    if (tier === 'middle') return avg >= 3 && avg <= 4;
    const share = played ? (won / played) * 100 : 0;
    return share >= 50 && share <= 80;
  }
  const tierLabel = (tier: string) =>
    tier === 'weakest'
      ? $t('adminRouteProbeWeakest')
      : tier === 'middle'
        ? $t('adminRouteProbeMiddle')
        : $t('adminRouteProbeStrong');
  const tierGoal = (tier: string) =>
    tier === 'weakest'
      ? $t('adminRouteProbeGoalWeakest')
      : tier === 'middle'
        ? $t('adminRouteProbeGoalMiddle')
        : $t('adminRouteProbeGoalStrong');

  /** Слаги врагов маршрутов: для ручной цены трофея. */
  let enemySlugs = $derived.by(() => {
    const set = new Set<string>();
    for (const r of routes) {
      for (const w of r.waves) {
        const c = challenges.find((x) => x.id === w.challengeId);
        c?.setup.keeperBoard.forEach((p) => set.add(p.card));
        c?.setup.keeperHand.forEach((s) => set.add(s));
        w.reinforcements.forEach((e) => set.add(e.card));
      }
    }
    return [...set].filter((s) => cards.some((c) => c.slug === s && c.kind !== 'structure')).sort();
  });
  const tierOf = (slug: string) => cards.find((c) => c.slug === slug)?.tier ?? 1;
  function setPrice(slug: string, raw: string) {
    if (!settings) return;
    if (raw.trim() === '') delete settings.trophyPrices[slug];
    else settings.trophyPrices[slug] = Math.max(0, Number(raw) || 0);
  }

  const field = 'border border-[#34251c]/20 bg-transparent px-2 py-1 text-sm';
  const cell = 'border border-[#34251c]/20 bg-transparent px-1.5 py-1 text-sm';
  const dotColor = (status: string) =>
    status === 'published' ? '#4a6b3a' : status === 'retired' ? '#8a6a55' : '#b08a3a';
  const statusText = (status: string) =>
    status === 'published'
      ? $t('adminRouteStatusPublished')
      : status === 'retired'
        ? $t('adminRouteStatusRetired')
        : $t('adminRouteStatusDraft');
  const btn =
    'px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40';
  const btnMain =
    'px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40';
  const th = 'text-left text-[10px] uppercase tracking-[0.14em] font-normal text-[#8a6a55] px-2 py-2';
</script>

<div class="flex-1 flex min-h-0">
  <aside class="w-56 flex-shrink-0 flex flex-col border-r border-[#34251c]/10">
    <div class="p-3 border-b border-[#34251c]/10">
      <button class="{btn} w-full" onclick={() => open(null)}>+ {$t('adminRouteNew')}</button>
    </div>
    <ul class="flex-1 overflow-y-auto">
      {#each routes as r (r.id)}
        <li>
          <button
            class="w-full text-left px-3 py-2.5 border-b border-[#34251c]/10 hover:bg-[#34251c]/5 {view === 'route' &&
            draft?.id === r.id
              ? 'bg-[#34251c]/10'
              : ''}"
            onclick={() => open(r)}
          >
            <span class="flex items-center gap-2 text-sm">
              <span class="inline-block w-2 h-2 rounded-full flex-shrink-0" style="background:{dotColor(r.status)}"></span>
              <span class="truncate">{routeTitle(r)}</span>
            </span>
            <span class="block mt-0.5 pl-4 text-[11px] text-[#8a6a55]">
              {statusText(r.status)} · {fmt('adminRouteWavesTotal', { n: r.waves.length })}{r.warnings.length
                ? ` · ⚠ ${r.warnings.length}`
                : ''}
            </span>
          </button>
        </li>
      {/each}
    </ul>
    {#if settings}
      <button
        class="w-full text-left px-3 py-3 border-t border-[#34251c]/10 hover:bg-[#34251c]/5 text-sm flex items-center gap-2 {view ===
        'settings'
          ? 'bg-[#34251c]/10'
          : ''}"
        onclick={() => {
          view = 'settings';
        }}
      >
        <span
          class="inline-block w-2 h-2 rounded-full flex-shrink-0"
          style="background:{settings.enabled ? '#4a6b3a' : '#8a6a55'}"
        ></span>
        {$t('adminRunSettings')}
      </button>
    {/if}
  </aside>

  {#if view === 'settings' && settings}
    <section class="flex-1 min-w-0 flex flex-col">
      <header class="flex items-center gap-3 px-4 py-3 border-b border-[#34251c]/10">
        <h2 class="text-lg flex-1">{$t('adminRunSettings')}</h2>
        <button class={btnMain} onclick={saveSettings} disabled={busy}>{$t('adminRunSave')}</button>
      </header>
      <div class="flex-1 overflow-y-auto p-4 space-y-4">
        <label
          class="flex items-center gap-3 max-w-2xl border p-3 cursor-pointer {settings.enabled
            ? 'border-[#4a6b3a]/50 bg-[#4a6b3a]/[0.07]'
            : 'border-[#34251c]/15'}"
        >
          <input type="checkbox" class="w-4 h-4" bind:checked={settings.enabled} />
          <span class="text-sm">{$t('adminRunEnabled')}</span>
        </label>

        <div class="max-w-2xl border border-[#34251c]/15 p-3">
          <h3 class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] mb-3">{$t('adminRunGroupReward')}</h3>
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
            {#each [['dailyDustCap', 'adminRunDailyCap'], ['lossSharePercent', 'adminRunLossShare'], ['rewardDust', 'adminRunRewardDust'], ['rewardHeal', 'adminRunRewardHeal']] as [key, label] (key)}
              <label class="text-xs flex flex-col gap-1 justify-between">{$t(label as 'adminRunDailyCap')}
                <input class="{field} w-28" type="number" min="0" bind:value={(settings as any)[key]} />
              </label>
            {/each}
          </div>
        </div>

        <div class="max-w-2xl border border-[#34251c]/15 p-3">
          <h3 class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] mb-3">{$t('adminRunGroupSwap')}</h3>
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
            {#each [['swapBase', 'adminRunSwapBase'], ['swapStep', 'adminRunSwapStep']] as [key, label] (key)}
              <label class="text-xs flex flex-col gap-1 justify-between">{$t(label as 'adminRunSwapBase')}
                <input class="{field} w-28" type="number" min="0" bind:value={(settings as any)[key]} />
              </label>
            {/each}
          </div>
        </div>

        <div class="max-w-2xl border border-[#34251c]/15 p-3">
          <h3 class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] mb-3">{$t('adminRunGroupTrophy')}</h3>
          <p class="text-xs mb-2">{$t('adminRunTrophyTiers')}</p>
          <div class="flex gap-2 flex-wrap">
            {#each settings.trophyTierPrices as _, i (i)}
              <label class="text-[10px] text-[#8a6a55] flex flex-col gap-0.5">{i + 1}
                <input class="{field} w-20" type="number" min="0" bind:value={settings.trophyTierPrices[i]} />
              </label>
            {/each}
          </div>
          {#if enemySlugs.length}
            <details class="mt-4">
              <summary class="text-xs cursor-pointer">{$t('adminRunTrophyPrices')} · {enemySlugs.length}</summary>
              <div class="mt-2 grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-1">
                {#each enemySlugs as slug (slug)}
                  <label class="flex items-center justify-between gap-2 text-sm">
                    <span class="truncate">{slug}</span>
                    <input
                      class="{cell} w-20 flex-shrink-0"
                      type="number"
                      min="0"
                      placeholder={String(settings.trophyTierPrices[Math.min(Math.max(tierOf(slug), 1), 5) - 1])}
                      value={settings.trophyPrices[slug] ?? ''}
                      oninput={(e) => setPrice(slug, e.currentTarget.value)}
                    />
                  </label>
                {/each}
              </div>
            </details>
          {/if}
        </div>
      </div>
    </section>
  {:else if draft}
    <section class="flex-1 min-w-0 flex flex-col">
      <header class="flex items-center gap-x-3 gap-y-2 flex-wrap px-4 py-3 border-b border-[#34251c]/10">
        <h2 class="text-lg mr-auto truncate">{draft.id ? routeTitle(draft) : $t('adminRouteNew')}</h2>
        {#if dirty}<span class="text-xs text-[#8f6a2a]">● {$t('adminRouteUnsaved')}</span>{/if}
        <select class="{cell} text-xs" bind:value={draft.status} aria-label={$t('adminRouteStatus')}>
          <option value="draft">{$t('adminRouteStatusDraft')}</option>
          <option value="published">{$t('adminRouteStatusPublished')}</option>
          <option value="retired">{$t('adminRouteStatusRetired')}</option>
        </select>
        {#if draft.id}
          <button
            class={btn}
            onclick={runProbe}
            disabled={busy || dirty}
            title={dirty ? $t('adminRouteProbeSaveFirst') : undefined}>{$t('adminRouteProbe')}</button
          >
        {/if}
        <button class={btnMain} onclick={save} disabled={busy}>{$t('adminRouteSave')}</button>
      </header>

      <div class="flex-1 overflow-y-auto p-4 space-y-5">
        {#if draft.warnings.length}
          <div class="border border-[#c65f3c]/40 bg-[#c65f3c]/[0.06] px-3 py-2">
            <p class="text-[10px] uppercase tracking-[0.16em] text-[#8f2f22] mb-1">
              {$t('adminRouteWarnings')} · {draft.warnings.length}
            </p>
            <ul class="text-xs list-disc pl-4 space-y-0.5">
              {#each draft.warnings as w (w)}<li>{warning(w)}</li>{/each}
            </ul>
          </div>
        {/if}

        {#if probe}
          <div>
            {#if probe.state === 'running'}
              <p class="text-sm text-[#8a6a55]">{$t('adminRouteProbeRunning')}</p>
            {:else}
              <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
                {#each probe.report.tiers as row (row.tier)}
                  {@const ok = inTarget(row.tier, row.avgCleared, row.won, row.played)}
                  <div class="border p-3 {ok ? 'border-[#4a6b3a]/50' : 'border-[#8f2f22]/40'}">
                    <p class="text-sm">{tierLabel(row.tier)}</p>
                    <p class="text-xs mt-0.5 {ok ? 'text-[#4a6b3a]' : 'text-[#8f2f22]'}">
                      {ok ? $t('adminRouteProbeIn') : $t('adminRouteProbeOut')}
                    </p>
                    <p class="text-xs mt-2">
                      {fmt('adminRouteProbeAvg', { n: row.avgCleared.toFixed(1), total: probe.report.waves })}
                    </p>
                    <p class="text-xs">{fmt('adminRouteProbeWon', { n: row.won, m: row.played })}</p>
                    <p class="text-[11px] text-[#8a6a55] mt-1">{tierGoal(row.tier)}</p>
                  </div>
                {/each}
              </div>
              {#if !probe.report.finished}
                <p class="text-xs text-[#8f2f22] mt-2">{$t('adminRouteProbeUnfinished')}</p>
              {/if}
              <p class="text-[11px] text-[#8a6a55] mt-2">{$t('adminRouteProbeNote')}</p>
            {/if}
          </div>
        {/if}

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 max-w-2xl">
          <label class="text-xs flex flex-col gap-1">{$t('adminRouteSlug')}
            <input class="{field} w-full" bind:value={draft.slug} />
          </label>
          <label class="text-xs flex flex-col gap-1">{$t('adminRouteGuestWaves')}
            <input
              class="{field} w-24"
              type="number"
              min="1"
              max="6"
              value={draft.guestWaves ?? ''}
              oninput={(e) => {
                const v = e.currentTarget.value;
                if (draft) draft.guestWaves = v === '' ? null : Number(v);
              }}
            />
          </label>
        </div>

        <details class="max-w-2xl" open={!draft.id}>
          <summary class="text-xs cursor-pointer">{$t('adminRouteNames')}</summary>
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mt-2">
            {#each [['titleEn', 'EN'], ['titleRu', 'RU'], ['titleUk', 'UK'], ['titleEs', 'ES'], ['titleDe', 'DE'], ['titleIt', 'IT']] as [key, tag] (key)}
              <label class="text-xs flex flex-col gap-1">{tag}
                <input class="{field} w-full" bind:value={(draft as any)[key]} />
              </label>
            {/each}
          </div>
        </details>

        <div>
          <div class="border border-[#34251c]/15 overflow-x-auto">
            <table class="w-full min-w-[42rem] text-sm">
              <thead class="border-b border-[#34251c]/15 bg-[#34251c]/[0.03]">
                <tr>
                  <th class={th}>№</th>
                  <th class={th}>{$t('adminRouteColBase')}</th>
                  <th class={th}>{$t('adminRouteColGoal')}</th>
                  <th class={th}>{$t('adminRouteColCards')}</th>
                  <th class={th}>{$t('adminRouteColDust')}</th>
                  <th class={th}>{$t('adminRouteColRace')}</th>
                  <th class={th}>{$t('adminRouteColReinf')}</th>
                </tr>
              </thead>
              <tbody>
                {#each draft.waves as w (w.waveNo)}
                  <tr class="border-b border-[#34251c]/10 align-top">
                    <td class="px-2 py-2 text-[#8a6a55]">{w.waveNo}</td>
                    <td class="px-1 py-1.5">
                      <select class="{cell} w-36" bind:value={w.challengeId} aria-label={$t('adminRouteBase')}>
                        {#each challenges as c (c.id)}
                          <option value={c.id}>{challengeTitle(c)}</option>
                        {/each}
                      </select>
                    </td>
                    <td class="px-1 py-1.5">
                      <div class="flex flex-col gap-1">
                        <select
                          class="{cell} w-36"
                          value={goalKind(w)}
                          aria-label={$t('adminRouteGoal')}
                          onchange={(e) => setGoal(w, e.currentTarget.value as GoalKind)}
                        >
                          <option value="none">{$t('adminRouteGoalNone')}</option>
                          <option value="holdRounds">{$t('adminRouteGoalHold')}</option>
                          <option value="defendStructure">{$t('adminRouteGoalDefend')}</option>
                          <option value="killTarget">{$t('adminRouteGoalKill')}</option>
                        </select>
                        {#if w.goal?.kind === 'holdRounds'}
                          <input
                            class="{cell} w-20"
                            type="number"
                            min="1"
                            max="30"
                            bind:value={w.goal.rounds}
                            aria-label={$t('adminRouteGoalRounds')}
                            placeholder={$t('adminRouteGoalRounds')}
                          />
                        {:else if w.goal?.kind === 'killTarget'}
                          <input
                            class="{cell} w-36"
                            bind:value={w.goal.name}
                            list="enemy-slugs"
                            aria-label={$t('adminRouteGoalTarget')}
                            placeholder={$t('adminRouteGoalTarget')}
                          />
                        {/if}
                      </div>
                    </td>
                    <td class="px-1 py-1.5 whitespace-nowrap">
                      <input
                        class="{cell} w-12"
                        type="number"
                        min="1"
                        max="6"
                        bind:value={w.minCards}
                        aria-label={$t('adminRouteCardsMin')}
                      />
                      –
                      <input
                        class="{cell} w-12"
                        type="number"
                        min="1"
                        max="6"
                        bind:value={w.maxCards}
                        aria-label={$t('adminRouteCardsMax')}
                      />
                    </td>
                    <td class="px-1 py-1.5">
                      <input
                        class="{cell} w-16"
                        type="number"
                        min="0"
                        max="1000"
                        bind:value={w.rewardDust}
                        aria-label={$t('adminRouteRewardDust')}
                      />
                    </td>
                    <td class="px-1 py-1.5">
                      <select class="{cell} w-24" bind:value={w.raceHint} aria-label={$t('adminRouteRace')}>
                        <option value={null}>—</option>
                        {#each races as r (r.id)}
                          <option value={r.slug}>{localized(r, 'name', $lang)}</option>
                        {/each}
                      </select>
                    </td>
                    <td class="px-1 py-1.5">
                      <button
                        class="{btn} whitespace-nowrap"
                        aria-expanded={openWave === w.waveNo}
                        onclick={() => (openWave = openWave === w.waveNo ? null : w.waveNo)}
                      >
                        {w.reinforcements.length} {openWave === w.waveNo ? '▴' : '▾'}
                      </button>
                    </td>
                  </tr>
                  {#if openWave === w.waveNo}
                    <tr class="border-b border-[#34251c]/10 bg-[#34251c]/[0.03]">
                      <td></td>
                      <td colspan="6" class="px-2 py-3">
                        <p class="text-xs mb-2">{fmt('adminRouteWave', { n: w.waveNo })} · {$t('adminRouteColReinf')}</p>
                        <RouteEntryPicker
                          setup={challenges.find((c) => c.id === w.challengeId)?.setup}
                          bind:entries={w.reinforcements}
                          {cards}
                        />
                      </td>
                    </tr>
                  {/if}
                {/each}
              </tbody>
            </table>
          </div>
          <datalist id="enemy-slugs">
            {#each cards as c (c.id)}<option value={c.slug}></option>{/each}
          </datalist>
          <div class="flex gap-2 mt-3 items-center">
            <button class={btn} onclick={addWave} disabled={draft.waves.length >= 12}>+ {$t('adminRouteWaveAdd')}</button>
            <button class={btn} onclick={dropWave} disabled={!draft.waves.length}>{$t('adminRouteWaveRemove')}</button>
            {#if draft.id}
              <button class="{btn} text-[#8f2f22] ml-auto" onclick={remove} disabled={busy}>{$t('adminRouteDelete')}</button>
            {/if}
          </div>
        </div>
      </div>
    </section>
  {:else}
    <section class="flex-1 grid place-items-center text-sm text-[#8a6a55]">{$t('adminRouteNew')}</section>
  {/if}
</div>
