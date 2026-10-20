<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { isOwnerBrowser, setOwnerBrowser } from '$lib/analytics';
  import { leafCopy } from '$lib/gazette';
  import { readingMinutes } from '$lib/tales';
  import type {
    BattleReport,
    BehaviourReport,
    EngagementShape,
    GazetteLeaf,
    HealthReport,
    InteractionRow,
    TaleReadRow,
    VitalsRow,
  } from '$lib/types/api';

  /** The panel's current range, UTC days. */
  let { range }: { range: { from: string; to: string } } = $props();

  let behaviour = $state<BehaviourReport | null>(null);
  let health = $state<HealthReport | null>(null);
  let battles = $state<BattleReport | null>(null);
  let tales = $state<GazetteLeaf[]>([]);
  let behaviourError = $state('');
  let healthError = $state('');
  let battlesError = $state('');
  let loading = $state(true);
  let owner = $state(false);

  // A slow reply for a range the admin has since left must not overwrite a
  // faster one for the range now on screen.
  let token = 0;
  async function load(from: string, to: string) {
    const mine = ++token;
    loading = true;
    const query = { from, to };
    const [b, h, w, t] = await Promise.allSettled([
      api.getBehaviourReport(query),
      api.getHealthReport(query),
      api.getBattleReport(query),
      api.getTales(),
    ]);
    if (mine !== token) return;
    behaviour = b.status === 'fulfilled' ? b.value : null;
    behaviourError = b.status === 'rejected' ? String(b.reason?.message ?? b.reason) : '';
    health = h.status === 'fulfilled' ? h.value : null;
    healthError = h.status === 'rejected' ? String(h.reason?.message ?? h.reason) : '';
    battles = w.status === 'fulfilled' ? w.value : null;
    battlesError = w.status === 'rejected' ? String(w.reason?.message ?? w.reason) : '';
    tales = t.status === 'fulfilled' ? t.value : [];
    loading = false;
  }

  onMount(() => {
    owner = isOwnerBrowser();
  });
  $effect(() => {
    void load(range.from, range.to);
  });

  function toggleOwner() {
    owner = !owner;
    setOwnerBrowser(owner);
  }

  // ── Formatting ────────────────────────────────────────────────────────
  const nf = new Intl.NumberFormat('en');
  const fmt = (n: number | null | undefined) => (n == null ? '—' : nf.format(Math.round(n)));
  const pct = (part: number, whole: number) => (whole > 0 ? `${Math.round((part / whole) * 100)}%` : '—');
  const share = (part: number, whole: number) => (whole > 0 ? (part / whole) * 100 : 0);
  function seconds(ms: number | null | undefined) {
    if (ms == null) return '—';
    const s = ms / 1000;
    if (s < 90) return `${Math.round(s)} s`;
    return `${(s / 60).toFixed(1)} min`;
  }
  function millis(ms: number | null | undefined) {
    if (ms == null) return '—';
    return ms >= 1000 ? `${(ms / 1000).toFixed(1)} s` : `${Math.round(ms)} ms`;
  }

  const ROOMS: Record<string, string> = {
    home: 'Home',
    archive: 'Archive',
    work: 'A work page',
    passport: 'Passport',
    hall: 'Hall',
    upcoming: 'Upcoming',
    author: 'Author',
    workshop: 'Workshop',
    commission: 'Commission',
    acquire: 'How to acquire',
    cellar: 'Cellar',
    tale: 'Tales',
    gazette: 'Gazette',
    battles: 'Battles',
    studio: 'Studio',
    impressions: 'Impressions',
    account: 'Profile & bookings',
    auth: 'Sign in / register',
    letter: 'Letter links',
    legal: 'Privacy & rights',
    other: 'Unknown & 404',
  };
  const room = (key: string) => ROOMS[key] ?? key;

  function exitLabel(exitTo: string): string {
    if (exitTo === 'hide') return 'closed or switched away';
    if (exitTo.startsWith('ext:')) return `↗ ${exitTo.slice(4)}`;
    if (exitTo.startsWith('to:')) return `→ ${room(exitTo.slice(3))}`;
    return exitTo;
  }

  // ── Behaviour: do they stay ───────────────────────────────────────────
  const TIME_BUCKETS: { key: keyof EngagementShape; label: string; tone: string }[] = [
    { key: 'durLt10', label: '< 10 s', tone: 'b1' },
    { key: 'dur1030', label: '10–30 s', tone: 'b2' },
    { key: 'dur3060', label: '30–60 s', tone: 'b3' },
    { key: 'dur60180', label: '1–3 min', tone: 'b4' },
    { key: 'durGe180', label: '3 min +', tone: 'b5' },
  ];
  const SCROLL_BUCKETS: { key: keyof EngagementShape; label: string; tone: string }[] = [
    { key: 'scrollLt25', label: '< 25%', tone: 'b1' },
    { key: 'scroll2549', label: '25–49%', tone: 'b2' },
    { key: 'scroll5074', label: '50–74%', tone: 'b3' },
    { key: 'scrollGe75', label: '75%+', tone: 'b5' },
  ];
  /** Rooms with fewer measured visits than this are shown but not judged. */
  const ENOUGH = 20;

  let shapeRows = $derived(
    (behaviour?.shape ?? []).filter((r) => r.visits > 0),
  );

  function bucketTotal(row: EngagementShape, buckets: { key: keyof EngagementShape }[]) {
    return buckets.reduce((sum, b) => sum + (row[b.key] as number), 0);
  }

  let totals = $derived.by(() => {
    const rows = behaviour?.shape ?? [];
    const sum = (f: (r: EngagementShape) => number) => rows.reduce((a, r) => a + f(r), 0);
    return {
      visits: sum((r) => r.visits),
      newVisits: sum((r) => r.newVisits),
      returning: sum((r) => r.returningVisits),
      newLong: sum((r) => r.newLong),
      returningLong: sum((r) => r.returningLong),
    };
  });

  // ── Behaviour: where they go ─────────────────────────────────────────
  function group<T>(rows: T[], key: (r: T) => string): Map<string, T[]> {
    const map = new Map<string, T[]>();
    for (const r of rows) {
      const k = key(r);
      const list = map.get(k);
      if (list) list.push(r);
      else map.set(k, [r]);
    }
    return map;
  }
  let exitsByRoom = $derived(group(behaviour?.exits ?? [], (r) => r.pathGroup));
  let sectionsByRoom = $derived(group(behaviour?.sections ?? [], (r) => r.pathGroup));
  let exitRooms = $derived(
    [...exitsByRoom.entries()]
      .map(([k, rows]) => ({ key: k, rows, total: rows.reduce((a, r) => a + r.visits, 0) }))
      .sort((a, b) => b.total - a.total),
  );

  // ── Behaviour: what they do ──────────────────────────────────────────
  let byName = $derived(group(behaviour?.interactions ?? [], (r) => r.name));
  const rowsOf = (name: string): InteractionRow[] => byName.get(name) ?? [];
  const eventsOf = (name: string) => rowsOf(name).reduce((a, r) => a + r.events, 0);
  const visitorsOf = (name: string, detail?: string) =>
    rowsOf(name)
      .filter((r) => detail === undefined || r.detail === detail)
      .reduce((a, r) => a + r.visitors, 0);

  let searches = $derived(
    rowsOf('search')
      .slice()
      .sort((a, b) => b.events - a.events)
      .slice(0, 14),
  );
  let emptySearches = $derived(
    rowsOf('search')
      .filter((r) => r.zeroEvents > 0)
      .sort((a, b) => b.zeroEvents - a.zeroEvents)
      .slice(0, 10),
  );
  // «Спросить сайт»: сколько спрашивали и о чём; вопросы без ответа — список того, чему учить сайт.
  let askTotal = $derived(eventsOf('site_ask'));
  let askKinds = $derived(
    (() => {
      const kinds = new Map<string, number>();
      for (const r of rowsOf('site_ask')) {
        const kind = r.detail.split(':')[1] ?? r.detail;
        kinds.set(kind, (kinds.get(kind) ?? 0) + r.events);
      }
      return [...kinds.entries()].sort((a, b) => b[1] - a[1]);
    })(),
  );
  let askMisses = $derived(
    rowsOf('site_ask_miss')
      .slice()
      .sort((a, b) => b.events - a.events)
      .slice(0, 20),
  );
  let filters = $derived(rowsOf('filter').slice().sort((a, b) => b.events - a.events).slice(0, 12));

  /** `surface:place` rows folded into one histogram per surface. */
  function placeHistogram(name: string) {
    const bySurface = new Map<string, Map<number, number>>();
    for (const r of rowsOf(name)) {
      const [surface, place] = r.detail.split(':');
      const n = Number(place);
      if (!surface || !Number.isFinite(n)) continue;
      const hist = bySurface.get(surface) ?? new Map<number, number>();
      hist.set(n, (hist.get(n) ?? 0) + r.events);
      bySurface.set(surface, hist);
    }
    return [...bySurface.entries()].map(([surface, hist]) => {
      const places = [...hist.entries()].sort((a, b) => a[0] - b[0]);
      const max = Math.max(1, ...places.map(([, c]) => c));
      return { surface, places, max, total: places.reduce((a, [, c]) => a + c, 0) };
    });
  }
  let workClicks = $derived(placeHistogram('work_click'));
  let taleClicks = $derived(placeHistogram('tale_click'));

  let galleryFrames = $derived(
    rowsOf('gallery_frame')
      .map((r) => ({ frame: Number(r.detail), events: r.events }))
      .filter((r) => Number.isFinite(r.frame))
      .sort((a, b) => a.frame - b.frame),
  );
  let galleryMax = $derived(Math.max(1, ...galleryFrames.map((f) => f.events)));

  const SMALL_ACTS: { name: string; label: string; hint: string }[] = [
    { name: 'gallery_open', label: 'Opened the big view', hint: 'A photo opened full-size.' },
    { name: 'lens', label: 'Used the magnifier', hint: 'Switched the brass lens on.' },
    { name: 'raking_light', label: 'Used raking light', hint: 'Switched the raking light on.' },
    { name: 'share', label: 'Shared a work', hint: 'Pressed share on a work page.' },
    { name: 'story_open', label: 'Opened the story card', hint: 'Opened the picture card of a work.' },
    { name: 'archive_end', label: 'Went on to the archive from the reel', hint: 'Pressed the last pane of the home reel.' },
    { name: 'tale_next', label: 'Followed "the next tale"', hint: 'Pressed the invitation under a tale.' },
  ];

  // ── Behaviour: forms and the way in ──────────────────────────────────
  const formName = (detail: string) => detail.split(':')[0];
  interface FormLine {
    form: string;
    started: number;
    ok: number;
    failed: number;
    abandoned: number;
    refused: number;
    stoppedAt: { field: string; events: number }[];
  }
  let formLines = $derived.by(() => {
    const lines = new Map<string, FormLine>();
    const line = (form: string) => {
      let l = lines.get(form);
      if (!l) {
        l = { form, started: 0, ok: 0, failed: 0, abandoned: 0, refused: 0, stoppedAt: [] };
        lines.set(form, l);
      }
      return l;
    };
    for (const r of rowsOf('form_start')) line(formName(r.detail)).started += r.events;
    for (const r of rowsOf('form_ok')) line(formName(r.detail)).ok += r.events;
    for (const r of rowsOf('form_fail')) line(formName(r.detail)).failed += r.events;
    for (const r of rowsOf('form_error')) line(formName(r.detail)).refused += r.events;
    for (const r of rowsOf('form_abandon')) {
      const l = line(formName(r.detail));
      l.abandoned += r.events;
      l.stoppedAt.push({ field: r.detail.split(':')[1] ?? '', events: r.events });
    }
    for (const l of lines.values()) l.stoppedAt.sort((a, b) => b.events - a.events);
    return [...lines.values()].sort((a, b) => b.started - a.started);
  });

  const FORM_LABELS: Record<string, string> = {
    order: 'Order request',
    request: 'Request (order / booking / waitlist / commission)',
    commission: 'Commission letter',
    impression: 'Book of impressions',
    comment: 'Comment',
    contact: 'Message to the author',
    subscribe: 'Visitor book (subscribe)',
    letter_slip: 'Letter slip under a tale',
    gazette_watch: 'Gazette watch',
  };

  /** Steps of a sign-in ribbon, counted as visits that reached each one. */
  function steps(prefix: string) {
    const reached = new Map<number, number>();
    for (const r of rowsOf('auth_step')) {
      const [door, step] = r.detail.split(':');
      if (door !== prefix) continue;
      const n = Number(step);
      if (Number.isFinite(n)) reached.set(n, (reached.get(n) ?? 0) + r.visitors);
    }
    const list = [...reached.entries()].sort((a, b) => a[0] - b[0]);
    const first = list[0]?.[1] ?? 0;
    return { list, first };
  }
  let registerSteps = $derived(steps('register'));
  let loginSteps = $derived(steps('login'));
  let authDone = $derived(rowsOf('auth_done'));
  let authErrors = $derived(rowsOf('auth_error').slice().sort((a, b) => b.events - a.events).slice(0, 8));
  let telegramStages = $derived(
    rowsOf('auth_tg')
      .map((r) => ({ door: r.detail.split(':')[0], stage: r.detail.split(':')[1], visitors: r.visitors }))
      .sort((a, b) => a.door.localeCompare(b.door) || a.stage.localeCompare(b.stage)),
  );

  // ── Tales ────────────────────────────────────────────────────────────
  const taleBySlug = $derived(new Map(tales.map((t) => [t.slug, t])));
  interface TaleLine {
    slug: string;
    title: string;
    opened: number;
    engaged: number;
    quick: number;
    finished: number;
    cameBack: number;
    medianMs: number | null;
    expectedMs: number | null;
  }
  let taleLines = $derived.by(() => {
    const merged = new Map<string, TaleReadRow[]>();
    for (const r of behaviour?.tales ?? []) {
      const list = merged.get(r.slug) ?? [];
      list.push(r);
      merged.set(r.slug, list);
    }
    const out: TaleLine[] = [];
    for (const [slug, rows] of merged) {
      const opened = rows.reduce((a, r) => a + r.opened, 0);
      // The median of medians is not a median; the larger arm's figure is the
      // honest single number when a tale is split across experiment arms.
      const biggest = rows.slice().sort((a, b) => b.engaged - a.engaged)[0];
      const leaf = taleBySlug.get(slug);
      const words = leaf ? readingMinutes(leafCopy(leaf, 'ru').body || leafCopy(leaf, 'en').body) : 0;
      out.push({
        slug,
        title: leaf ? leafCopy(leaf, 'ru').title || leafCopy(leaf, 'en').title : slug,
        opened,
        engaged: rows.reduce((a, r) => a + r.engaged, 0),
        quick: rows.reduce((a, r) => a + r.quickExits, 0),
        finished: rows.reduce((a, r) => a + r.finished, 0),
        cameBack: rows.reduce((a, r) => a + r.cameBack, 0),
        medianMs: biggest?.medianActiveMs ?? null,
        expectedMs: words ? words * 60_000 : null,
      });
    }
    return out.sort((a, b) => b.opened - a.opened);
  });

  let sceneArms = $derived.by(() => {
    const arms = new Map<string, { opened: number; engaged: number; quick: number; finished: number; weighted: number }>();
    for (const r of behaviour?.tales ?? []) {
      if (!r.variant) continue;
      const arm = arms.get(r.variant) ?? { opened: 0, engaged: 0, quick: 0, finished: 0, weighted: 0 };
      arm.opened += r.opened;
      arm.engaged += r.engaged;
      arm.quick += r.quickExits;
      arm.finished += r.finished;
      arm.weighted += (r.medianActiveMs ?? 0) * r.engaged;
      arms.set(r.variant, arm);
    }
    return [...arms.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  });

  // ── Speed ────────────────────────────────────────────────────────────
  type Metric = 'lcp' | 'inp' | 'cls';
  const LIMITS: Record<Metric, { good: number; poor: number }> = {
    lcp: { good: 2500, poor: 4000 },
    inp: { good: 200, poor: 500 },
    cls: { good: 100, poor: 250 },
  };
  function grade(metric: Metric, value: number | null): 'good' | 'mid' | 'poor' | 'none' {
    if (value == null) return 'none';
    if (value <= LIMITS[metric].good) return 'good';
    return value <= LIMITS[metric].poor ? 'mid' : 'poor';
  }
  const clsText = (v: number | null) => (v == null ? '—' : (v / 1000).toFixed(2));

  /** Vitals folded across devices, per room, and per device across rooms. */
  function foldVitals(rows: VitalsRow[], key: (r: VitalsRow) => string) {
    const map = new Map<string, VitalsRow[]>();
    for (const r of rows) {
      const k = key(r);
      const list = map.get(k) ?? [];
      list.push(r);
      map.set(k, list);
    }
    const weighted = (list: VitalsRow[], value: (r: VitalsRow) => number | null, weight: (r: VitalsRow) => number) => {
      let sum = 0;
      let w = 0;
      for (const r of list) {
        const v = value(r);
        if (v == null) continue;
        sum += v * weight(r);
        w += weight(r);
      }
      return w ? sum / w : null;
    };
    return [...map.entries()]
      .map(([k, list]) => ({
        key: k,
        samples: list.reduce((a, r) => a + r.samples, 0),
        lcp: weighted(list, (r) => r.lcpP75, (r) => r.lcpN),
        inp: weighted(list, (r) => r.inpP75, (r) => r.inpN),
        cls: weighted(list, (r) => r.clsP75, (r) => r.clsN),
        ttfb: weighted(list, (r) => r.ttfbP75, (r) => r.samples),
        lcpGood: share(
          list.reduce((a, r) => a + r.lcpGood, 0),
          list.reduce((a, r) => a + r.lcpN, 0),
        ),
      }))
      .sort((a, b) => b.samples - a.samples);
  }
  let vitalsByRoom = $derived(foldVitals(health?.vitals ?? [], (r) => r.pathGroup));
  let vitalsByDevice = $derived(foldVitals(health?.vitals ?? [], (r) => r.deviceClass));

  // ── Trust ────────────────────────────────────────────────────────────
  let trustNotes = $derived.by(() => {
    const out: { tone: 'ok' | 'warn'; text: string }[] = [];
    if (!health) return out;
    const { trust, pipeline } = health;
    if (pipeline.failedBatches > 0) {
      out.push({ tone: 'warn', text: `${fmt(pipeline.failedEvents)} events were lost in ${fmt(pipeline.failedBatches)} failed database writes since the server last started — see the server log ("analytics batch insert failed").` });
    }
    if (pipeline.droppedQueueFull > 0) {
      out.push({ tone: 'warn', text: `${fmt(pipeline.droppedQueueFull)} events were dropped because the write queue was full.` });
    }
    if (trust.pageViews >= 30) {
      const have = trust.pageEngaged / trust.pageViews;
      out.push(
        have < 0.6
          ? { tone: 'warn', text: `Only ${pct(trust.pageEngaged, trust.pageViews)} of page views have a matching time-on-page row. Some browsers drop the last message when a tab closes; the numbers below under-count short visits.` }
          : { tone: 'ok', text: `${pct(trust.pageEngaged, trust.pageViews)} of page views have a time-on-page row.` },
      );
    }
    if (trust.returningTotal >= 30 && trust.returningKnown / trust.returningTotal < 0.8) {
      out.push({ tone: 'warn', text: `New-or-returning is unknown for ${pct(trust.returningTotal - trust.returningKnown, trust.returningTotal)} of views (browser storage closed, or older events).` });
    }
    return out;
  });

  // ── Battles ──────────────────────────────────────────────────────────
  let battleTotals = $derived.by(() => {
    const rows = battles?.challenges ?? [];
    const sum = (f: (r: (typeof rows)[number]) => number) => rows.reduce((a, r) => a + f(r), 0);
    return { started: sum((r) => r.started), finished: sum((r) => r.finished), won: sum((r) => r.won), abandoned: sum((r) => r.abandoned) };
  });
</script>

<div class="insights">
  <div class="owner-row">
    <label>
      <input type="checkbox" checked={owner} onchange={toggleOwner} />
      Don't count my own visits in this browser
    </label>
    <span class="muted">
      Set automatically when you sign in to the admin. The server cannot tell your visits from a
      visitor's, so the browser itself stays silent.
    </span>
  </div>

  {#if loading && !behaviour && !health}
    <div class="state">Loading behaviour…</div>
  {/if}

  <!-- ── DO THEY STAY ─────────────────────────────────────────────────── -->
  <section class="a-block">
    <h3 class="block-label"><span>Do they stay</span></h3>
    {#if behaviourError}
      <p class="state state--error">{behaviourError}</p>
    {:else if behaviour}
      <p class="section-note">
        Time is <strong>foreground time</strong>: the clock stops when the tab is hidden or the visitor
        has not touched anything for two minutes. Each bar is the share of visits in that bucket.
        Measured since the new tracker went live; days before it are empty or use the old, wall-clock figures.
      </p>
      {#if totals.visits === 0}
        <p class="empty">No measured visits in this range yet.</p>
      {:else}
        <div class="kpis">
          <div class="kpi">
            <span class="kpi-n">{fmt(totals.visits)}</span>
            <span class="kpi-l">measured visits</span>
          </div>
          <div class="kpi" title="Visits by a browser that had been on the site on an earlier day, among visits where that is known.">
            <span class="kpi-n">{pct(totals.returning, totals.returning + totals.newVisits)}</span>
            <span class="kpi-l">come back</span>
          </div>
          <div class="kpi" title="Of the visits longer than 30 s, how many were by returning browsers.">
            <span class="kpi-n">{pct(totals.returningLong, totals.returningLong + totals.newLong)}</span>
            <span class="kpi-l">of long visits are returning</span>
          </div>
          <div class="kpi" title="Share of new visitors who stayed longer than 30 s, against returning ones.">
            <span class="kpi-n">{pct(totals.newLong, totals.newVisits)} / {pct(totals.returningLong, totals.returning)}</span>
            <span class="kpi-l">stay 30 s+: new / returning</span>
          </div>
        </div>

        <div class="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Room</th>
                <th class="right">Visits</th>
                <th>Time on the page</th>
                <th>How far down</th>
                <th class="right" title="Share of visits by a browser seen on an earlier day.">Returning</th>
              </tr>
            </thead>
            <tbody>
              {#each shapeRows as row (row.pathGroup)}
                {@const timeTotal = bucketTotal(row, TIME_BUCKETS)}
                {@const scrollTotal = bucketTotal(row, SCROLL_BUCKETS)}
                <tr class:thin={row.visits < ENOUGH}>
                  <td>{room(row.pathGroup)}</td>
                  <td class="right">{fmt(row.visits)}</td>
                  <td>
                    <div class="stack" role="img" aria-label="Time on the page, by bucket">
                      {#each TIME_BUCKETS as b}
                        {@const n = row[b.key] as number}
                        {#if n > 0}<i class="seg {b.tone}" style="width:{share(n, timeTotal)}%" title="{b.label}: {fmt(n)} ({pct(n, timeTotal)})"></i>{/if}
                      {/each}
                    </div>
                  </td>
                  <td>
                    <div class="stack" role="img" aria-label="Scroll depth, by bucket">
                      {#each SCROLL_BUCKETS as b}
                        {@const n = row[b.key] as number}
                        {#if n > 0}<i class="seg {b.tone}" style="width:{share(n, scrollTotal)}%" title="{b.label}: {fmt(n)} ({pct(n, scrollTotal)})"></i>{/if}
                      {/each}
                    </div>
                  </td>
                  <td class="right">{pct(row.returningVisits, row.returningVisits + row.newVisits)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <div class="legend">
          {#each TIME_BUCKETS as b}<span><i class="seg {b.tone}"></i>{b.label}</span>{/each}
          <span class="muted">· lighter = left sooner · rooms with fewer than {ENOUGH} visits are greyed</span>
        </div>
      {/if}
    {/if}
  </section>

  <!-- ── WHERE THEY GO ────────────────────────────────────────────────── -->
  <section class="a-block">
    <h3 class="block-label"><span>Where they go, and where they stop</span></h3>
    {#if behaviour}
      {#if exitRooms.length === 0}
        <p class="empty">No exits recorded yet.</p>
      {:else}
        <p class="section-note">
          For each room, the commonest ways out (the next room, another site, or the tab being
          closed or switched away), and the last heading the visitor's scroll reached
          (last 30 days only: the headings are text and are not kept in the permanent rollup).
        </p>
        <div class="cards">
          {#each exitRooms.slice(0, 12) as r (r.key)}
            <div class="card">
              <h4>{room(r.key)} <small class="muted">· {fmt(r.total)} visits</small></h4>
              <ul class="bars">
                {#each r.rows as e}
                  <li>
                    <span class="bar-label">{exitLabel(e.exitTo)}</span>
                    <span class="bar-track"><i style="width:{share(e.visits, r.total)}%"></i></span>
                    <span class="bar-n">{pct(e.visits, r.total)}</span>
                  </li>
                {/each}
              </ul>
              {#if sectionsByRoom.get(r.key)?.length}
                <p class="mini-title">Last heading reached</p>
                <ul class="plain">
                  {#each sectionsByRoom.get(r.key) ?? [] as s}
                    <li><span>{s.section}</span> <small class="muted">{fmt(s.visits)}{s.medianActiveMs != null ? ` · ${seconds(s.medianActiveMs)}` : ''}</small></li>
                  {/each}
                </ul>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </section>

  <!-- ── WHAT THEY DO ─────────────────────────────────────────────────── -->
  <section class="a-block">
    <h3 class="block-label"><span>What they do</span></h3>
    {#if behaviour}
      {#if (behaviour.interactions ?? []).length === 0}
        <p class="empty">No acts recorded yet.</p>
      {:else}
        <div class="cards">
          {#if searches.length}
            <div class="card">
              <h4>Archive search <small class="muted">· {fmt(eventsOf('search'))} searches</small></h4>
              <table class="mini">
                <thead><tr><th>Phrase</th><th class="right">Times</th><th class="right">Found</th></tr></thead>
                <tbody>
                  {#each searches as s}
                    <tr>
                      <td>{s.detail}</td>
                      <td class="right">{fmt(s.events)}</td>
                      <td class="right" class:bad={s.zeroEvents > 0}>
                        {s.events ? (s.zeroEvents === s.events ? 'nothing' : `~${Math.round(s.valueSum / s.events)}`) : '—'}
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
              {#if emptySearches.length}
                <p class="note-bad">
                  Searches that found nothing: {emptySearches.map((s) => `“${s.detail}” ×${s.zeroEvents}`).join(', ')}.
                  Each is a work to make, or a word to teach the archive.
                </p>
              {/if}
            </div>
          {/if}

          {#if askTotal > 0 || askMisses.length}
            <div class="card">
              <h4>Ask the site <small class="muted">· {fmt(askTotal)} questions</small></h4>
              {#if askKinds.length}
                <ul class="bars">
                  {#each askKinds as [kind, n]}
                    <li>
                      <span class="bar-label">{kind}</span>
                      <span class="bar-track"><i style="width:{share(n, askKinds[0][1])}%"></i></span>
                      <span class="bar-n">{fmt(n)}</span>
                    </li>
                  {/each}
                </ul>
              {/if}
              {#if askMisses.length}
                <p class="mini-title">Questions the site could not answer</p>
                <table class="mini">
                  <thead><tr><th>Question</th><th class="right">Times</th></tr></thead>
                  <tbody>
                    {#each askMisses as m}
                      <tr><td>{m.detail}</td><td class="right">{fmt(m.events)}</td></tr>
                    {/each}
                  </tbody>
                </table>
                <p class="muted small">Each is a kind of question to teach in <code>site-ask.ts</code>. Counted once per visit.</p>
              {/if}
            </div>
          {/if}

          {#if filters.length}
            <div class="card">
              <h4>Archive filters <small class="muted">· {fmt(eventsOf('filter'))} changes</small></h4>
              <ul class="bars">
                {#each filters as f}
                  <li>
                    <span class="bar-label">{f.detail}</span>
                    <span class="bar-track"><i style="width:{share(f.events, filters[0].events)}%"></i></span>
                    <span class="bar-n">{fmt(f.events)}</span>
                  </li>
                {/each}
              </ul>
            </div>
          {/if}

          {#each workClicks as surface}
            <div class="card">
              <h4>Which work was pressed: {surface.surface.replace('_', ' ')} <small class="muted">· {fmt(surface.total)} clicks</small></h4>
              <p class="mini-title">By place in the block (1 = first)</p>
              <div class="hist">
                {#each surface.places as [place, count]}
                  <div class="hist-col" title="Place {place}: {fmt(count)} clicks ({pct(count, surface.total)})">
                    <i style="height:{share(count, surface.max)}%"></i>
                    <span>{place}</span>
                  </div>
                {/each}
              </div>
            </div>
          {/each}

          {#each taleClicks as surface}
            <div class="card">
              <h4>Which tale was pressed on the shelf <small class="muted">· {fmt(surface.total)} clicks</small></h4>
              <div class="hist">
                {#each surface.places as [place, count]}
                  <div class="hist-col" title="Place {place}: {fmt(count)} clicks">
                    <i style="height:{share(count, surface.max)}%"></i>
                    <span>{place}</span>
                  </div>
                {/each}
              </div>
            </div>
          {/each}

          {#if galleryFrames.length}
            <div class="card">
              <h4>Photos on a work page <small class="muted">· which frame was opened</small></h4>
              <div class="hist">
                {#each galleryFrames as f}
                  <div class="hist-col" title="Frame {f.frame}: {fmt(f.events)} visits">
                    <i style="height:{share(f.events, galleryMax)}%"></i>
                    <span>{f.frame}</span>
                  </div>
                {/each}
              </div>
              <p class="muted small">Counted once per visit per frame. A drop-off after frame 1 says the later photographs are not being looked for.</p>
            </div>
          {/if}

          <div class="card">
            <h4>Small acts</h4>
            <ul class="plain">
              {#each SMALL_ACTS as a}
                {#if eventsOf(a.name) > 0}
                  <li title={a.hint}><span>{a.label}</span> <small class="muted">{fmt(visitorsOf(a.name))} visits</small></li>
                {/if}
              {/each}
            </ul>
          </div>
        </div>
      {/if}
    {/if}
  </section>

  <!-- ── TALES ────────────────────────────────────────────────────────── -->
  <section class="a-block">
    <h3 class="block-label"><span>Are the tales read</span></h3>
    {#if behaviour}
      {#if taleLines.length === 0}
        <p class="empty">No tale visits in the last 30 days.</p>
      {:else}
        <p class="section-note">
          “Read to the end” is the last line reaching the screen, counted for everyone (the seal on the
          shelf is kept only in the reader's own browser). “Time vs. text” is the reader's median
          foreground time against the time the text takes at 170 words a minute: near 100% means it was
          read; well under 40% means it was skimmed or left.
        </p>
        <div class="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Tale</th>
                <th class="right">Opened</th>
                <th class="right" title="Left in under 10 seconds of presence.">Left at once</th>
                <th class="right">Read to the end</th>
                <th class="right">Median time</th>
                <th class="right">Time vs. text</th>
                <th class="right" title="Visits by a browser seen on an earlier day.">Came back</th>
              </tr>
            </thead>
            <tbody>
              {#each taleLines as t (t.slug)}
                <tr class:thin={t.opened < 10}>
                  <td><a href="/tales/{t.slug}" target="_blank" rel="noreferrer">{t.title}</a></td>
                  <td class="right">{fmt(t.opened)}</td>
                  <td class="right">{pct(t.quick, t.engaged)}</td>
                  <td class="right">{pct(t.finished, t.opened)}</td>
                  <td class="right">{seconds(t.medianMs)}</td>
                  <td class="right">{t.medianMs != null && t.expectedMs ? pct(t.medianMs, t.expectedMs) : '—'}</td>
                  <td class="right">{pct(t.cameBack, t.opened)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        {#if sceneArms.length}
          <h4 class="sub">The scene experiment</h4>
          <div class="table-scroll">
            <table class="mini">
              <thead>
                <tr><th>Arm</th><th class="right">Opened</th><th class="right">Left at once</th><th class="right">Read to the end</th><th class="right">Typical time</th></tr>
              </thead>
              <tbody>
                {#each sceneArms as [arm, a]}
                  <tr>
                    <td>{arm}</td>
                    <td class="right">{fmt(a.opened)}</td>
                    <td class="right">{pct(a.quick, a.engaged)}</td>
                    <td class="right">{pct(a.finished, a.opened)}</td>
                    <td class="right">{seconds(a.engaged ? a.weighted / a.engaged : null)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          <p class="muted small">Time is foreground time since the tracker was corrected; an older, wall-clock figure for the same tale is not comparable.</p>
        {/if}
      {/if}
    {/if}
  </section>

  <!-- ── FORMS AND THE WAY IN ─────────────────────────────────────────── -->
  <section class="a-block">
    <h3 class="block-label"><span>Forms and the way in</span></h3>
    {#if behaviour}
      {#if formLines.length === 0 && registerSteps.list.length === 0 && loginSteps.list.length === 0}
        <p class="empty">No form or sign-in activity recorded yet.</p>
      {/if}
      {#if formLines.length}
        <div class="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Form</th>
                <th class="right" title="Visits in which someone put the cursor in a field.">Started</th>
                <th class="right">Sent</th>
                <th class="right" title="Sent, but the server refused it.">Refused</th>
                <th class="right" title="Started and left without sending.">Left half-filled</th>
                <th>Where they stopped</th>
              </tr>
            </thead>
            <tbody>
              {#each formLines as f (f.form)}
                <tr>
                  <td>{FORM_LABELS[f.form] ?? f.form}</td>
                  <td class="right">{fmt(f.started)}</td>
                  <td class="right">{fmt(f.ok)} <small class="muted">{f.started ? pct(f.ok, f.started) : ''}</small></td>
                  <td class="right" class:bad={f.failed > 0}>{fmt(f.failed)}</td>
                  <td class="right">{fmt(f.abandoned)}</td>
                  <td>
                    {#each f.stoppedAt.slice(0, 3) as s, i}{i ? ', ' : ''}<code>{s.field}</code> ×{s.events}{/each}
                    {#if f.refused}<small class="muted"> · {fmt(f.refused)} field checks failed</small>{/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <p class="muted small">“Where they stopped” names the field they were last in — never what was typed. Sent is counted from the server's answer, not the click.</p>
      {/if}

      {#if registerSteps.list.length || loginSteps.list.length}
        <div class="cards">
          {#each [['Creating a name', registerSteps], ['Signing in', loginSteps]] as [title, flow]}
            {@const f = flow as typeof registerSteps}
            {#if f.list.length}
              <div class="card">
                <h4>{title}</h4>
                <ul class="bars">
                  {#each f.list as [step, n]}
                    <li>
                      <span class="bar-label">Step {step}</span>
                      <span class="bar-track"><i style="width:{share(n, f.first)}%"></i></span>
                      <span class="bar-n">{fmt(n)} <small class="muted">{pct(n, f.first)}</small></span>
                    </li>
                  {/each}
                </ul>
                <p class="muted small">Visits that reached each screen of the key. A long fall between two steps is where the ribbon loses people.</p>
              </div>
            {/if}
          {/each}
          {#if authDone.length || telegramStages.length}
            <div class="card">
              <h4>Doors</h4>
              <ul class="plain">
                {#each authDone as d}<li><span>Signed in / created: {d.detail.replace('_', ' via ')}</span> <small class="muted">{fmt(d.visitors)}</small></li>{/each}
                {#each telegramStages as s}<li><span>Telegram · {s.door} · {s.stage}</span> <small class="muted">{fmt(s.visitors)}</small></li>{/each}
              </ul>
              {#if authErrors.length}
                <p class="mini-title">Refusals</p>
                <ul class="plain">
                  {#each authErrors as e}<li><span>{e.detail}</span> <small class="muted">×{e.events}</small></li>{/each}
                </ul>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    {/if}
  </section>

  <!-- ── SPEED ────────────────────────────────────────────────────────── -->
  <section class="a-block">
    <h3 class="block-label"><span>How fast it loads, on their devices</span></h3>
    {#if healthError}
      <p class="state state--error">{healthError}</p>
    {:else if health}
      {#if vitalsByRoom.length === 0}
        <p class="empty">No speed samples yet.</p>
      {:else}
        <p class="section-note">
          75th percentile — three in four visits were at least this fast. Green is “good” by the
          published thresholds: LCP under 2.5 s, INP under 200 ms, CLS under 0.1. Only the page a
          visit landed on is measured.
        </p>
        <div class="table-scroll">
          <table>
            <thead>
              <tr>
                <th>{vitalsByDevice.length > 1 ? 'Room / device' : 'Room'}</th>
                <th class="right">Samples</th>
                <th class="right" title="Largest contentful paint: when the main picture or text appeared.">LCP</th>
                <th class="right" title="Share of loads under 2.5 s.">LCP good</th>
                <th class="right" title="Interaction to next paint: how long the page took to answer a tap.">INP</th>
                <th class="right" title="Cumulative layout shift: how much the page jumped.">CLS</th>
                <th class="right" title="Time to first byte from the server.">Server</th>
              </tr>
            </thead>
            <tbody>
              {#each vitalsByDevice as d (d.key)}
                <tr class="group-row">
                  <td>{d.key}</td>
                  <td class="right">{fmt(d.samples)}</td>
                  <td class="right {grade('lcp', d.lcp)}">{millis(d.lcp)}</td>
                  <td class="right">{d.lcp == null ? '—' : `${Math.round(d.lcpGood)}%`}</td>
                  <td class="right {grade('inp', d.inp)}">{millis(d.inp)}</td>
                  <td class="right {grade('cls', d.cls)}">{clsText(d.cls)}</td>
                  <td class="right">{millis(d.ttfb)}</td>
                </tr>
              {/each}
              {#each vitalsByRoom as r (r.key)}
                <tr class:thin={r.samples < 10}>
                  <td>{room(r.key)}</td>
                  <td class="right">{fmt(r.samples)}</td>
                  <td class="right {grade('lcp', r.lcp)}">{millis(r.lcp)}</td>
                  <td class="right">{r.lcp == null ? '—' : `${Math.round(r.lcpGood)}%`}</td>
                  <td class="right {grade('inp', r.inp)}">{millis(r.inp)}</td>
                  <td class="right {grade('cls', r.cls)}">{clsText(r.cls)}</td>
                  <td class="right">{millis(r.ttfb)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    {/if}
  </section>

  <!-- ── WHAT BREAKS ──────────────────────────────────────────────────── -->
  <section class="a-block">
    <h3 class="block-label"><span>What breaks</span></h3>
    {#if health}
      {#if health.errors.length === 0}
        <p class="empty">Nothing broke in this range — or nothing was reported yet.</p>
      {:else}
        <div class="table-scroll">
          <table>
            <thead>
              <tr><th>Kind</th><th>Where</th><th>What</th><th class="right">Times</th><th class="right">Visits</th></tr>
            </thead>
            <tbody>
              {#each health.errors as e}
                <tr>
                  <td><span class="kind kind--{e.kind}">{e.kind.replace('_', ' ')}</span></td>
                  <td>{room(e.pathGroup)}</td>
                  <td class="what">{e.detail || '—'}</td>
                  <td class="right">{fmt(e.events)}</td>
                  <td class="right">{fmt(e.visitors)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
      {#if health.deadLinks.length}
        <h4 class="sub">Dead addresses, and who sent people there</h4>
        <div class="table-scroll">
          <table class="mini">
            <thead><tr><th>Asked for</th><th>Came from</th><th class="right">Times</th></tr></thead>
            <tbody>
              {#each health.deadLinks as d}
                <tr><td><code>{d.path}</code></td><td>{d.cameFrom}</td><td class="right">{fmt(d.events)}</td></tr>
              {/each}
            </tbody>
          </table>
        </div>
        <p class="muted small">“Came from” is the outside site when the link was followed from one, otherwise the room of this house. Fix the link at the source, or add a redirect.</p>
      {/if}
    {/if}
  </section>

  <!-- ── BATTLES ──────────────────────────────────────────────────────── -->
  <section class="a-block">
    <h3 class="block-label"><span>Battles, from the games themselves</span></h3>
    {#if battlesError}
      <p class="state state--error">{battlesError}</p>
    {:else if battles}
      {#if battles.challenges.length === 0}
        <p class="empty">No games started in this range.</p>
      {:else}
        <div class="kpis">
          <div class="kpi"><span class="kpi-n">{fmt(battles.visitors)}</span><span class="kpi-l">signed-in people came through the door</span></div>
          <div class="kpi"><span class="kpi-n">{fmt(battleTotals.started)}</span><span class="kpi-l">games started</span></div>
          <div class="kpi"><span class="kpi-n">{pct(battleTotals.finished, battleTotals.started)}</span><span class="kpi-l">finished</span></div>
          <div class="kpi"><span class="kpi-n">{pct(battleTotals.won, battleTotals.finished)}</span><span class="kpi-l">of finished are won</span></div>
        </div>
        <div class="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Challenge</th>
                <th class="right">Started</th>
                <th class="right">Players</th>
                <th class="right">Finished</th>
                <th class="right">Won</th>
                <th class="right" title="Started more than a day ago and never finished.">Abandoned</th>
                <th class="right" title="How many actions the abandoned games had reached when they stopped.">Stopped after</th>
                <th class="right">Rounds</th>
                <th class="right">Length</th>
              </tr>
            </thead>
            <tbody>
              {#each battles.challenges as c}
                <tr>
                  <td>{c.title ?? c.slug ?? 'Free game'}</td>
                  <td class="right">{fmt(c.started)}</td>
                  <td class="right">{fmt(c.players)}</td>
                  <td class="right">{pct(c.finished, c.started)}</td>
                  <td class="right">{pct(c.won, c.finished)}</td>
                  <td class="right" class:bad={c.started > 0 && c.abandoned / c.started > 0.4}>{fmt(c.abandoned)}</td>
                  <td class="right">{c.avgActionsWhenAbandoned == null ? '—' : `${Math.round(c.avgActionsWhenAbandoned)} actions`}</td>
                  <td class="right">{c.avgRounds == null ? '—' : c.avgRounds.toFixed(1)}</td>
                  <td class="right">{c.avgMinutes == null ? '—' : `${c.avgMinutes.toFixed(1)} min`}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <p class="muted small">Counted from the match records, so it includes everything since the games began, not only since the tracker.</p>
      {/if}
    {/if}
  </section>

  <!-- ── CAN THE NUMBERS BE TRUSTED ───────────────────────────────────── -->
  <section class="a-block">
    <h3 class="block-label"><span>Can these numbers be trusted</span></h3>
    {#if health}
      <ul class="notes">
        {#each trustNotes as n}<li class="note note--{n.tone}">{n.text}</li>{/each}
        {#if trustNotes.length === 0}<li class="note note--ok">Nothing to flag.</li>{/if}
      </ul>
      <div class="kpis">
        <div class="kpi" title="Events the server accepted since it last started."><span class="kpi-n">{fmt(health.pipeline.accepted)}</span><span class="kpi-l">events accepted</span></div>
        <div class="kpi" title="Written to the database."><span class="kpi-n">{fmt(health.pipeline.written)}</span><span class="kpi-l">written</span></div>
        <div class="kpi" title="Requests recognised as robots (crawlers, scripts, headless browsers) and not counted."><span class="kpi-n">{fmt(health.pipeline.skippedBot)}</span><span class="kpi-l">robots filtered</span></div>
        <div class="kpi" title="Visitors who send Do Not Track and are not counted."><span class="kpi-n">{fmt(health.pipeline.skippedDnt)}</span><span class="kpi-l">Do Not Track</span></div>
      </div>
      <p class="muted small">
        Pipeline counters are since the server last started. The share of visitors who block tracking
        is not knowable from here by definition; assume the real traffic is higher than shown.
      </p>
    {/if}
  </section>

  <details class="glossary">
    <summary>What do these terms mean?</summary>
    <dl>
      <dt>Foreground time</dt><dd>How long the visitor was actually there: the tab visible and something touched in the last two minutes. It replaces the older wall-clock time, which counted tabs left open and background tabs.</dd>
      <dt>Returning</dt><dd>A browser that had opened the site on an earlier day. The browser keeps only a date; no identifier is stored or sent, and the daily visitor hash still changes every day.</dd>
      <dt>Quick exit / left at once</dt><dd>Present for under ten seconds.</dd>
      <dt>Last heading reached</dt><dd>The last heading on the page that the visitor's deepest scroll got to — where on the page the visit ended.</dd>
      <dt>Read to the end</dt><dd>The last line of a tale reached the screen.</dd>
      <dt>Left half-filled</dt><dd>The visitor put the cursor in a field and left the page without the form being accepted.</dd>
      <dt>LCP / INP / CLS</dt><dd>When the main thing appeared, how fast the page answered a tap, and how much it jumped while loading — each measured on the visitor's own device.</dd>
    </dl>
  </details>
</div>

<style>
  .insights {
    display: grid;
    gap: 0.4rem;
    color: #34251c;
  }
  h3, h4, p { margin: 0; }
  .a-block { margin-bottom: 1.4rem; }
  .block-label {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 0.7rem;
    color: #6f3b24;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.78rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .block-label::after {
    content: '';
    flex: 1;
    height: 1px;
    background: #d8c6b1;
  }
  .muted { color: #8a6f5c; }
  .small { font-size: 0.7rem; margin-top: 0.45rem; }
  .section-note { margin-bottom: 0.7rem; color: #5f4636; font-size: 0.74rem; line-height: 1.5; max-width: 62rem; }
  .empty, .state { padding: 0.9rem; border: 1px dashed #d8c6b1; color: #8a6f5c; background: #fff; }
  .state--error { color: #9a2d1c; border-color: #c65f3c; }

  .owner-row {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.4rem 1rem;
    padding: 0.6rem 0.8rem;
    margin-top: 0.9rem;
    border: 1px solid #d8c6b1;
    background: #fff;
    font-size: 0.74rem;
  }
  .owner-row label { display: flex; gap: 0.4rem; align-items: center; color: #6f3b24; }

  .kpis { display: grid; grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr)); gap: 0.6rem; margin-bottom: 0.8rem; }
  .kpi { border: 1px solid #d8c6b1; background: #fff; padding: 0.6rem 0.75rem; display: grid; gap: 0.1rem; }
  .kpi-n { font-family: Georgia, 'Fraunces', serif; font-size: 1.25rem; color: #6f3b24; }
  .kpi-l { font-size: 0.68rem; color: #8a6f5c; }

  .table-scroll { overflow-x: auto; background: #fff; border: 1px solid #d8c6b1; }
  table { width: 100%; border-collapse: collapse; font-size: 0.76rem; }
  th, td { padding: 0.45rem 0.6rem; border-bottom: 1px solid #efe4d3; text-align: left; vertical-align: middle; }
  th { color: #6f3b24; font-weight: 600; font-size: 0.68rem; letter-spacing: 0.04em; text-transform: uppercase; white-space: nowrap; }
  td.right, th.right { text-align: right; font-variant-numeric: tabular-nums; }
  tr.thin td { color: #a8916f; }
  tr.group-row td { background: #faf4ea; font-weight: 600; }
  table.mini { font-size: 0.72rem; }
  td.bad, .bad { color: #9a2d1c; }
  td.good { color: #3d6b3a; }
  td.mid { color: #9a6a12; }
  td.poor { color: #9a2d1c; font-weight: 600; }
  code { font-size: 0.72rem; color: #6f3b24; }
  .what { max-width: 34rem; word-break: break-word; }
  a { color: #6f3b24; }

  .stack { display: flex; height: 0.7rem; min-width: 9rem; background: #efe4d3; }
  .seg { display: inline-block; height: 100%; }
  .legend .seg { width: 0.7rem; height: 0.7rem; margin-right: 0.25rem; vertical-align: middle; }
  .legend { display: flex; flex-wrap: wrap; gap: 0.3rem 0.9rem; margin-top: 0.5rem; font-size: 0.68rem; color: #5f4636; }
  .b1 { background: #c65f3c; }
  .b2 { background: #d99a6c; }
  .b3 { background: #cdb592; }
  .b4 { background: #9aa874; }
  .b5 { background: #5f8a5a; }

  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(19rem, 1fr)); gap: 0.8rem; margin-bottom: 0.8rem; }
  .card { border: 1px solid #d8c6b1; background: #fff; padding: 0.75rem 0.85rem; display: grid; gap: 0.45rem; align-content: start; }
  .card h4 { font-family: Georgia, 'Fraunces', serif; font-size: 0.86rem; color: #6f3b24; font-weight: 600; }
  .sub { margin: 0.9rem 0 0.45rem; font-family: Georgia, 'Fraunces', serif; font-size: 0.84rem; color: #6f3b24; }
  .mini-title { font-size: 0.66rem; letter-spacing: 0.05em; text-transform: uppercase; color: #8a6f5c; margin-top: 0.3rem; }
  .plain { list-style: none; margin: 0; padding: 0; display: grid; gap: 0.25rem; font-size: 0.76rem; }
  .plain li { display: flex; justify-content: space-between; gap: 0.6rem; }

  .bars { list-style: none; margin: 0; padding: 0; display: grid; gap: 0.3rem; }
  .bars li { display: grid; grid-template-columns: minmax(0, 9.5rem) 1fr auto; gap: 0.5rem; align-items: center; font-size: 0.74rem; }
  .bar-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bar-track { height: 0.55rem; background: #efe4d3; }
  .bar-track i { display: block; height: 100%; background: #c65f3c; }
  .bar-n { font-variant-numeric: tabular-nums; white-space: nowrap; }

  .hist { display: flex; align-items: flex-end; gap: 0.25rem; height: 6rem; }
  .hist-col { flex: 1; min-width: 0.7rem; height: 100%; display: flex; flex-direction: column; justify-content: flex-end; align-items: center; gap: 0.2rem; font-size: 0.62rem; color: #8a6f5c; }
  .hist-col i { display: block; width: 100%; background: #c65f3c; min-height: 2px; }

  .note-bad { font-size: 0.72rem; color: #9a2d1c; line-height: 1.45; }
  .kind { padding: 0.1rem 0.4rem; border: 1px solid #d8c6b1; font-size: 0.66rem; text-transform: uppercase; letter-spacing: 0.04em; }
  .kind--js, .kind--promise, .kind--render { border-color: #c65f3c; color: #9a2d1c; }
  .kind--api { border-color: #9a6a12; color: #9a6a12; }

  .notes { list-style: none; margin: 0 0 0.8rem; padding: 0; display: grid; gap: 0.35rem; }
  .note { padding: 0.5rem 0.7rem; font-size: 0.74rem; border-left: 3px solid #d8c6b1; background: #fff; }
  .note--warn { border-left-color: #c65f3c; color: #7a2e1a; }
  .note--ok { border-left-color: #5f8a5a; color: #3d6b3a; }

  .glossary { border: 1px solid #d8c6b1; background: #fff; padding: 0.6rem 0.8rem; font-size: 0.74rem; }
  .glossary summary { cursor: pointer; color: #6f3b24; }
  .glossary dl { margin: 0.6rem 0 0; display: grid; gap: 0.5rem; }
  .glossary dt { font-weight: 600; color: #6f3b24; }
  .glossary dd { margin: 0; color: #5f4636; line-height: 1.5; }
</style>
