<script lang="ts">
  // Сцена на полях байки, какой её видит стол рассказов.
  //
  // Автор загружает кадры, подписывает, что на них, выбирает, как кадры
  // сменяют друг друга, — а когда именно, решает алгоритм по тексту
  // (`sceneTiming.ts`), и любой момент можно сдвинуть рукой. Сдвинутый рукой
  // помечается и пересчётом больше не трогается, пока его не вернут «в авто».
  //
  // Стоит на всю ширину стола вместо текста (кнопка «Сцена» на ленте стола), а
  // не в нижней полосе под ним: нижняя полоса не прокручивается, и открытая
  // секция выдавливала из стола бумагу, а сама не умещалась.
  //
  // Плита для просмотра — тот же `TaleScenePlate`, что у читателя, только число
  // ему даёт ползунок, а не прокрутка: просмотр, считающий иначе, чем страница,
  // соврал бы ровно о том, ради чего его открывают.
  import { api } from '$lib/api';
  import { t, type Lang } from '$lib/i18n';
  import { renderTale } from '$lib/tales';
  import { boundariesOf, placeTransitions, spansFor, type TimingReason } from '$lib/sceneTiming';
  import { taleSceneFor } from '$lib/taleScenes';
  import TaleScenePlate from '../TaleScenePlate.svelte';
  import type { SceneTransitionType, TaleSceneData } from '$lib/types/api';

  let {
    taleId,
    slug,
    bodyEn,
    bodyRu,
    lang,
  }: { taleId: string; slug: string; bodyEn: string; bodyRu: string; lang: Lang } = $props();

  const TYPES: SceneTransitionType[] = ['fade', 'push', 'bloom', 'wipe', 'rise', 'flush', 'surge'];
  const RATIOS = ['3 / 4', '4 / 5', '1 / 1', '1 / 2'];

  let scene = $state<TaleSceneData | null>(null);
  let loaded = $state(false);
  let saving = $state(false);
  let savedAt = $state<Date | null>(null);
  let uploading = $state('');
  let dirty = $state(false);
  let message = $state('');
  let scrub = $state(0.5);
  let reasons = $state<TimingReason[]>([]);
  let strip = $state<HTMLDivElement | null>(null);
  let dragging = $state<number | null>(null);
  let over = $state(false);

  // Байка, на которой считают: тот язык, на котором сейчас пишут, а если там
  // пусто — другой. Доли текста у версий совпадают, поэтому момент годится обеим.
  let blocks = $derived(renderTale((lang === 'ru' ? bodyRu || bodyEn : bodyEn || bodyRu) || ''));
  let bounds = $derived(boundariesOf(blocks));
  let codeScene = $derived(slug ? taleSceneFor(slug) : null);
  let ready = $derived(!!scene && scene.frames.length >= 2);

  $effect(() => {
    const id = taleId;
    loaded = false;
    scene = null;
    dirty = false;
    message = '';
    api
      .adminGetTaleScene(id)
      .then((s) => {
        if (id !== taleId) return;
        scene = s;
        if (s) retime(false);
      })
      .catch((e) => (message = String(e)))
      .finally(() => (loaded = true));
  });

  function blank(): TaleSceneData {
    return { enabled: true, ratio: '3 / 4', position: '50% 50%', frames: [], transitions: [] };
  }

  /** Переходов на один меньше, чем кадров. */
  function syncTransitions() {
    if (!scene) return;
    const need = Math.max(0, scene.frames.length - 1);
    while (scene.transitions.length < need) {
      scene.transitions.push({ type: 'fade', at: 0, span: 0.14, manual: false });
    }
    scene.transitions.length = need;
  }

  /**
   * Расставить неручные переходы по тексту. Ручные стоят, где поставлены, а
   * длительность у неручных пересчитывается от соседей: два перехода не должны
   * наезжать друг на друга.
   */
  function retime(markDirty = true) {
    if (!scene || scene.transitions.length === 0) {
      reasons = [];
      return;
    }
    const res = placeTransitions({
      blocks,
      captions: scene.frames.map((f) => f.caption),
      transitions: scene.transitions.map((x) => ({ at: x.at, manual: x.manual })),
    });
    const spans = spansFor(res.at);
    scene.transitions.forEach((x, k) => {
      if (!x.manual) {
        x.at = +res.at[k].toFixed(4);
        x.span = +spans[k].toFixed(4);
      }
    });
    reasons = res.reasons;
    if (markDirty) dirty = true;
  }

  /** Адрес картинки без чужого начала: на сервере читателя он свой. */
  function plain(url: string): string {
    try {
      const u = new URL(url, location.href);
      return u.pathname.startsWith('/static/') ? u.pathname : url;
    } catch {
      return url;
    }
  }

  async function uploadFiles(files: File[]) {
    const images = files.filter((f) => f.type.startsWith('image/'));
    if (images.length === 0) return;
    message = '';
    const next = scene ?? blank();
    try {
      for (let i = 0; i < images.length; i++) {
        uploading = $t('adminSceneUploadingN').replace('{n}', String(i + 1)).replace('{total}', String(images.length));
        const imported = await api.importMediaWithVariants(images[i], 'images', `${slug || 'tale'}-scene`);
        next.frames.push({ url: plain(imported.url), caption: '' });
      }
      scene = next;
      syncTransitions();
      retime();
    } catch (e) {
      // Что успело загрузиться, остаётся в сцене: терять половину пачки из-за
      // одного сбоя значило бы грузить её заново.
      if (next.frames.length > 0) {
        scene = next;
        syncTransitions();
        retime();
      }
      message = `${$t('adminSceneUploadFail')}: ${e}`;
    } finally {
      uploading = '';
    }
  }

  function pickFiles() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';
    input.multiple = true;
    input.onchange = () => void uploadFiles([...(input.files ?? [])]);
    input.click();
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    over = false;
    void uploadFiles([...(e.dataTransfer?.files ?? [])]);
  }

  function removeFrame(i: number) {
    if (!scene) return;
    scene.frames.splice(i, 1);
    syncTransitions();
    retime();
  }

  function moveFrame(i: number, by: -1 | 1) {
    if (!scene) return;
    const j = i + by;
    if (j < 0 || j >= scene.frames.length) return;
    [scene.frames[i], scene.frames[j]] = [scene.frames[j], scene.frames[i]];
    retime();
  }

  function setType(k: number, type: SceneTransitionType) {
    if (!scene) return;
    scene.transitions[k].type = type;
    dirty = true;
  }

  function unlock(k: number) {
    if (!scene) return;
    scene.transitions[k].manual = false;
    retime();
  }

  // Метки на ленте байки. Метку тянут мышью; отпущенная прилипает к ближайшей
  // границе абзацев, если та рядом: момент «посреди фразы» ничего не значит.
  function fracAt(clientX: number): number {
    const r = strip!.getBoundingClientRect();
    return Math.min(1, Math.max(0, (clientX - r.left) / r.width));
  }

  function snap(f: number): number {
    let best = f;
    let bd = 0.025;
    for (const b of bounds) {
      const d = Math.abs(b.frac - f);
      if (d < bd) {
        bd = d;
        best = b.frac;
      }
    }
    return best;
  }

  /** Между какими соседями переход может стоять. */
  function room(k: number): [number, number] {
    const tr = scene!.transitions;
    return [k === 0 ? 0 : tr[k - 1].at, k === tr.length - 1 ? 1 : tr[k + 1].at];
  }

  function markerDown(e: PointerEvent, k: number) {
    try {
      (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    } catch {
      // Синтетическое событие без живого указателя: тянуть можно и без захвата.
    }
    dragging = k;
  }

  function markerMove(e: PointerEvent, k: number) {
    if (dragging !== k || !scene) return;
    const [lo, hi] = room(k);
    const f = Math.min(hi, Math.max(lo, fracAt(e.clientX)));
    const x = scene.transitions[k];
    x.at = +f.toFixed(4);
    x.manual = true;
    scrub = f;
    dirty = true;
  }

  function markerUp(k: number) {
    if (dragging !== k || !scene) return;
    dragging = null;
    const [lo, hi] = room(k);
    const x = scene.transitions[k];
    x.at = +Math.min(hi, Math.max(lo, snap(x.at))).toFixed(4);
    scrub = x.at;
  }

  function markerKey(e: KeyboardEvent, k: number) {
    if (!scene || (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight')) return;
    e.preventDefault();
    const [lo, hi] = room(k);
    const step = (e.shiftKey ? 0.05 : 0.01) * (e.key === 'ArrowLeft' ? -1 : 1);
    const x = scene.transitions[k];
    x.at = +Math.min(hi, Math.max(lo, x.at + step)).toFixed(4);
    x.manual = true;
    scrub = x.at;
    dirty = true;
  }

  function importFromCode() {
    const c = codeScene;
    if (!c?.frames || !c.transitions) return;
    scene = {
      enabled: true,
      ratio: c.ratio ?? '3 / 4',
      position: c.position ?? '50% 50%',
      frames: c.frames.map((f) => ({ url: f.url, caption: '', position: f.position })),
      // Назначенные кодом моменты берём как ручные: пересчёт не должен молча
      // сдвинуть то, что автор уже видел и принял.
      transitions: c.transitions.map((x) => ({ ...x, manual: true })),
    };
    syncTransitions();
    dirty = true;
  }

  // Сохранение само: картинку загрузили, подпись набрали — и ничего нажимать
  // не нужно. Раньше сцена жила только в окне до нажатия «Сохранить», и
  // автор, загрузивший кадры и ушедший смотреть байку, видел страницу без
  // сцены: сохранять было нечего, потому что он этого не делал.
  const AUTOSAVE_MS = 1600;
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    // Читаем всё, что значит «сцена изменилась», чтобы эффект её видел.
    void JSON.stringify(scene);
    if (!dirty || !scene || scene.frames.length < 2) return;
    clearTimeout(timer);
    timer = setTimeout(() => void save(true), AUTOSAVE_MS);
    return () => clearTimeout(timer);
  });

  async function save(quiet = false) {
    if (!scene) return;
    if (!quiet) message = '';
    if (scene.frames.length < 2) {
      message = $t('adminSceneNeedTwo');
      return;
    }
    saving = true;
    try {
      const sent = JSON.stringify(scene);
      const back = await api.adminPutTaleScene(taleId, scene);
      // Пока писали, автор мог тронуть что-то ещё: тогда ответ сервера не
      // перекрывает его правку, а `dirty` остаётся, и следующий круг дописывает.
      if (JSON.stringify(scene) === sent) {
        scene = back;
        dirty = false;
      }
      savedAt = new Date();
      message = '';
    } catch (e) {
      message = `${$t('adminSceneFail')}: ${e}`;
    } finally {
      saving = false;
    }
  }

  async function drop() {
    if (!scene || !confirm($t('adminSceneDeleteAsk'))) return;
    try {
      await api.adminDeleteTaleScene(taleId);
      scene = null;
      dirty = false;
      message = '';
    } catch (e) {
      message = String(e);
    }
  }

  const pct = (f: number) => `${(f * 100).toFixed(1)}%`;
  const pct0 = (f: number) => `${Math.round(f * 100)}%`;
  const clock = (d: Date) => d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  const reasonText = (r: TimingReason | undefined) => {
    if (!r) return '';
    const base = $t(`adminSceneReason_${r.code}` as never);
    return r.code === 'caption' && r.word ? `${base}: «${r.word}…»` : base;
  };

  // Первые слова абзаца после границы: что читатель увидит сразу после перехода.
  function afterWords(frac: number): string {
    const b = bounds.reduce((best, x) => (Math.abs(x.frac - frac) < Math.abs(best.frac - frac) ? x : best), bounds[0]);
    const blk = b ? blocks[b.i + 1] : null;
    const text = blk && (blk.kind === 'p' || blk.kind === 'heading') ? blk.text : '';
    return text.length > 70 ? `${text.slice(0, 70)}…` : text;
  }

  let positionX = $derived(parseInt(scene?.position.split(' ')[0] ?? '50', 10) || 50);
  let positionY = $derived(parseInt(scene?.position.split(' ')[1] ?? '50', 10) || 50);
  function setPosition(x: number, y: number) {
    if (!scene) return;
    scene.position = `${x}% ${y}%`;
    dirty = true;
  }
  function setRatio(r: string) {
    if (!scene) return;
    scene.ratio = r;
    dirty = true;
  }
</script>

<section class="se">
  <header class="se-head">
    <div class="se-title">
      <h2>{$t('adminSceneTitle')}</h2>
      <p>{$t('adminSceneHint')}</p>
    </div>
    <div class="se-meta">
      <span class="se-status" aria-live="polite">
        {#if saving}{$t('adminTalesSaving')}
        {:else if dirty && ready}{$t('adminSceneUnsaved')}
        {:else if savedAt}{$t('adminSceneSavedAt').replace('{time}', clock(savedAt))}{/if}
      </span>
      {#if scene}
        <label class="se-check">
          <input type="checkbox" bind:checked={scene.enabled} onchange={() => (dirty = true)} />
          {$t('adminSceneEnabled')}
        </label>
        {#if slug}
          <a class="se-link" href="/tales/{slug}?scene=on" target="_blank" rel="noopener">{$t('adminSceneOpen')} ↗</a>
        {/if}
        <button class="se-link se-link--danger" onclick={drop}>{$t('adminSceneDelete')}</button>
      {/if}
    </div>
  </header>

  {#if message}<p class="se-error" role="alert">{message}</p>{/if}

  {#if !loaded}
    <p class="se-muted">…</p>
  {:else if !scene || scene.frames.length === 0}
    <div class="se-empty">
      <button
        type="button"
        class="se-drop se-drop--big"
        class:over
        ondragover={(e) => { e.preventDefault(); over = true; }}
        ondragleave={() => (over = false)}
        ondrop={onDrop}
        onclick={pickFiles}
        disabled={!!uploading}
      >
        <strong>{uploading || $t('adminSceneDrop')}</strong>
        <span>{$t('adminSceneDropHint')}</span>
      </button>
      {#if codeScene?.frames}
        <button class="se-link" onclick={importFromCode}>{$t('adminSceneFromCode')}</button>
      {/if}
    </div>
  {:else}
    <div class="se-grid">
      <div class="se-main">
        {#if scene.transitions.length > 0}
          <section class="se-card">
            <div class="se-card-head">
              <h3>{$t('adminSceneStripTitle')}</h3>
              <button class="se-link" onclick={() => retime()}>{$t('adminSceneRetime')}</button>
            </div>
            <div bind:this={strip} class="strip" role="group" aria-label={$t('adminSceneStrip')}>
              {#each bounds as b}
                {@const turn = b.why === 'ornament' || b.why === 'heading'}
                <i class="tick" class:tick--turn={turn} style="left: {pct(b.frac)}"></i>
              {/each}
              {#each scene.transitions as tr, k}
                <span class="band" style="left: {pct(Math.max(0, tr.at - tr.span / 2))}; width: {pct(Math.min(tr.span, 1))}"></span>
                <button
                  type="button"
                  class="marker"
                  class:marker--hand={tr.manual}
                  class:marker--drag={dragging === k}
                  style="left: {pct(tr.at)}"
                  onpointerdown={(e) => markerDown(e, k)}
                  onpointermove={(e) => markerMove(e, k)}
                  onpointerup={() => markerUp(k)}
                  onpointercancel={() => markerUp(k)}
                  onkeydown={(e) => markerKey(e, k)}
                  aria-label="{k + 1} → {k + 2}: {pct0(tr.at)}"
                >{k + 1}</button>
              {/each}
              <span class="needle" style="left: {pct(scrub)}"></span>
            </div>
            <div class="strip-ends"><span>{$t('adminSceneStart')}</span><span>{$t('adminSceneStrip')}</span><span>{$t('adminSceneEnd')}</span></div>
          </section>
        {/if}

        <section class="se-card">
          <h3>{$t('adminSceneFramesTitle')}</h3>
          <ol class="frames">
            {#each scene.frames as frame, i (frame.url + i)}
              <li>
                <div class="frame">
                  <span class="num">{i + 1}</span>
                  <img class="thumb" src={frame.url} alt="" />
                  <input
                    class="in"
                    bind:value={frame.caption}
                    oninput={() => (dirty = true)}
                    onchange={() => retime()}
                    placeholder={$t('adminSceneCaptionPh')}
                    aria-label={$t('adminSceneCaption')}
                  />
                  <div class="frame-tools">
                    <button class="icon" onclick={() => moveFrame(i, -1)} disabled={i === 0} aria-label={$t('adminSceneUp')} title={$t('adminSceneUp')}>↑</button>
                    <button class="icon" onclick={() => moveFrame(i, 1)} disabled={i === scene.frames.length - 1} aria-label={$t('adminSceneDown')} title={$t('adminSceneDown')}>↓</button>
                    <button class="icon icon--danger" onclick={() => removeFrame(i)} aria-label={$t('adminSceneRemove')} title={$t('adminSceneRemove')}>✕</button>
                  </div>
                </div>

                {#if i < scene.transitions.length}
                  {@const tr = scene.transitions[i]}
                  <div class="tr">
                    <div class="tr-row">
                      <span class="tr-n">{i + 1} → {i + 2}</span>
                      <select class="in in--select" value={tr.type} onchange={(e) => setType(i, e.currentTarget.value as SceneTransitionType)} aria-label={$t('adminSceneType')}>
                        {#each TYPES as ty}
                          <option value={ty}>{$t(`adminSceneT_${ty}` as never)}</option>
                        {/each}
                      </select>
                      <span class="tr-at">{$t('adminSceneAt')} <b>{pct0(tr.at)}</b></span>
                      <span class="chip" class:chip--hand={tr.manual}>{reasonText(reasons[i])}</span>
                      {#if tr.manual}
                        <button class="se-link" onclick={() => unlock(i)}>{$t('adminSceneAuto')}</button>
                      {/if}
                    </div>
                    {#if afterWords(tr.at)}
                      <p class="tr-quote">«{afterWords(tr.at)}»</p>
                    {/if}
                    <label class="tr-span">
                      <span>{$t('adminSceneSpan')}</span>
                      <input type="range" min="0.04" max="0.4" step="0.01" value={tr.span} oninput={(e) => { tr.span = +e.currentTarget.value; dirty = true; }} />
                      <b>{pct0(tr.span)}</b>
                    </label>
                  </div>
                {/if}
              </li>
            {/each}
          </ol>

          <button
            type="button"
            class="se-drop"
            class:over
            ondragover={(e) => { e.preventDefault(); over = true; }}
            ondragleave={() => (over = false)}
            ondrop={onDrop}
            onclick={pickFiles}
            disabled={!!uploading}
          >{uploading || $t('adminSceneAddFrames')}</button>
          {#if scene.frames.length < 2}<p class="se-muted">{$t('adminSceneNeedTwo')}</p>{/if}
        </section>
      </div>

      <aside class="se-side">
        <h3>{$t('adminScenePreview')}</h3>
        {#if ready}
          <div class="preview" style="--w: {scene.ratio === '1 / 2' ? '200px' : '250px'}">
            <TaleScenePlate
              figure={scene.frames[0].url}
              effect="frames"
              frames={scene.frames.map((f) => ({ url: f.url, position: f.position }))}
              transitions={scene.transitions}
              ratio={scene.ratio}
              position={scene.position}
              grow={scrub}
            />
          </div>
        {/if}
        <label class="scrub">
          <span>{$t('adminSceneScrub')} <b>{pct0(scrub)}</b></span>
          <input type="range" min="0" max="1" step="0.005" bind:value={scrub} />
        </label>

        <div class="side-block">
          <h3>{$t('adminSceneRatio')}</h3>
          <div class="seg" role="radiogroup" aria-label={$t('adminSceneRatio')}>
            {#each RATIOS as r}
              <button role="radio" aria-checked={scene.ratio === r} class:on={scene.ratio === r} onclick={() => setRatio(r)}>{r.replace(' / ', ':')}</button>
            {/each}
          </div>
        </div>

        <div class="side-block">
          <h3>{$t('adminScenePosition')}</h3>
          <label class="pos"><span>←→</span><input type="range" min="0" max="100" value={positionX} oninput={(e) => setPosition(+e.currentTarget.value, positionY)} /></label>
          <label class="pos"><span>↑↓</span><input type="range" min="0" max="100" value={positionY} oninput={(e) => setPosition(positionX, +e.currentTarget.value)} /></label>
        </div>
      </aside>
    </div>
  {/if}
</section>

<style>
  .se {
    --ink: #34251c;
    --muted: #8a6a55;
    --line: #d8c6b1;
    --ember: #c65f3c;
    color: var(--ink);
    font: 13px/1.5 Inter, system-ui, sans-serif;
    max-width: 1080px;
    margin: 0 auto;
    padding: 28px 24px 56px;
  }

  h2 {
    font: 400 24px/1.2 Fraunces, Georgia, serif;
    margin: 0 0 6px;
  }

  h3 {
    margin: 0 0 12px;
    font-size: 10px;
    font-weight: 500;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .se-head {
    display: flex;
    flex-wrap: wrap;
    gap: 12px 32px;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 20px;
  }

  .se-title p {
    margin: 0;
    max-width: 58ch;
    color: var(--muted);
  }

  .se-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 18px;
    font-size: 12px;
  }

  .se-status {
    min-width: 12ch;
    color: var(--muted);
  }

  .se-check {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
  }

  .se-check input {
    accent-color: var(--ember);
  }

  .se-link {
    padding: 0;
    border: 0;
    background: none;
    font: inherit;
    font-size: 12px;
    color: var(--muted);
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }

  .se-link:hover {
    color: var(--ink);
  }

  .se-link--danger:hover {
    color: var(--ember);
  }

  .se-muted {
    color: var(--muted);
    margin: 8px 0 0;
  }

  .se-error {
    margin: 0 0 16px;
    padding: 10px 14px;
    border: 1px solid rgba(198, 95, 60, 0.4);
    background: rgba(198, 95, 60, 0.06);
    color: #8a2f12;
  }

  .se-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 280px;
    gap: 24px;
    align-items: start;
  }

  @media (max-width: 900px) {
    .se-grid {
      grid-template-columns: 1fr;
    }
  }

  .se-card {
    border: 1px solid var(--line);
    background: rgba(255, 255, 255, 0.4);
    padding: 18px 20px 20px;
    margin-bottom: 20px;
  }

  .se-card-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
  }

  .se-card-head h3 {
    margin-bottom: 14px;
  }

  /* ── пустая сцена и приём файлов ─────────────────────────────────── */

  .se-empty {
    display: grid;
    gap: 14px;
    justify-items: start;
  }

  .se-drop {
    display: block;
    width: 100%;
    margin-top: 16px;
    padding: 14px;
    border: 1.5px dashed #c9b8a1;
    background: transparent;
    font: inherit;
    font-size: 12px;
    color: var(--muted);
    text-align: center;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s, color 0.15s;
  }

  .se-drop:hover,
  .se-drop.over {
    border-color: var(--ember);
    color: var(--ember);
    background: rgba(198, 95, 60, 0.04);
  }

  .se-drop:disabled {
    cursor: progress;
    opacity: 0.7;
  }

  .se-drop--big {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 56px 24px;
    max-width: 640px;
  }

  .se-drop--big strong {
    font: 400 18px/1.3 Fraunces, Georgia, serif;
    color: var(--ink);
  }

  /* ── лента байки ─────────────────────────────────────────────────── */

  .strip {
    position: relative;
    height: 68px;
    border: 1px solid var(--line);
    background: rgba(52, 37, 28, 0.03);
    user-select: none;
    touch-action: none;
  }

  .tick {
    position: absolute;
    bottom: 0;
    width: 1px;
    height: 22%;
    background: rgba(52, 37, 28, 0.16);
  }

  .tick--turn {
    height: 52%;
    background: rgba(52, 37, 28, 0.5);
  }

  .band {
    position: absolute;
    top: 0;
    bottom: 0;
    background: rgba(198, 95, 60, 0.12);
    pointer-events: none;
  }

  .needle {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: rgba(52, 37, 28, 0.7);
    pointer-events: none;
  }

  .marker {
    position: absolute;
    top: 10px;
    width: 26px;
    height: 26px;
    margin-left: -13px;
    padding: 0;
    border: 1.5px solid var(--ember);
    border-radius: 50%;
    background: #f8f1e7;
    color: var(--ember);
    font: 600 11px/1 Inter, system-ui, sans-serif;
    cursor: grab;
    box-shadow: 0 1px 4px rgba(52, 37, 28, 0.25);
  }

  .marker--hand {
    background: var(--ember);
    color: #fff;
  }

  .marker--drag {
    cursor: grabbing;
    transform: scale(1.12);
  }

  .marker:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: 2px;
  }

  .strip-ends {
    display: flex;
    justify-content: space-between;
    margin-top: 6px;
    font-size: 11px;
    color: var(--muted);
  }

  /* ── кадры и переходы ────────────────────────────────────────────── */

  .frames {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .frame {
    display: grid;
    grid-template-columns: 22px 54px minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
  }

  .num {
    font-size: 11px;
    text-align: center;
    color: var(--muted);
  }

  .thumb {
    width: 54px;
    aspect-ratio: 3 / 4;
    object-fit: cover;
    border: 1px solid var(--line);
    background: #1a120e;
  }

  .in {
    width: 100%;
    padding: 8px 10px;
    border: 1px solid rgba(52, 37, 28, 0.2);
    background: rgba(255, 255, 255, 0.55);
    color: var(--ink);
    font: inherit;
    font-size: 13px;
  }

  .in:focus {
    outline: none;
    border-color: var(--ink);
  }

  .in::placeholder {
    color: #b09b8a;
  }

  .in--select {
    width: auto;
    min-width: 190px;
  }

  .frame-tools {
    display: flex;
    gap: 2px;
  }

  .icon {
    width: 30px;
    height: 30px;
    padding: 0;
    border: 1px solid transparent;
    background: none;
    color: var(--muted);
    font-size: 14px;
    cursor: pointer;
  }

  .icon:hover:not(:disabled) {
    border-color: var(--line);
    color: var(--ink);
  }

  .icon--danger:hover:not(:disabled) {
    color: var(--ember);
  }

  .icon:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .tr {
    margin: 10px 0 10px 34px;
    padding: 12px 14px;
    border-left: 2px solid rgba(198, 95, 60, 0.5);
    background: rgba(198, 95, 60, 0.05);
    display: grid;
    gap: 8px;
  }

  .tr-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 14px;
  }

  .tr-n {
    font-size: 10px;
    letter-spacing: 0.14em;
    color: var(--muted);
  }

  .tr-at {
    font-size: 12px;
    color: #5f4636;
  }

  .chip {
    padding: 2px 8px;
    border: 1px solid var(--line);
    border-radius: 99px;
    font-size: 11px;
    color: var(--muted);
  }

  .chip--hand {
    border-color: rgba(198, 95, 60, 0.5);
    color: var(--ember);
  }

  .tr-quote {
    margin: 0;
    font: italic 13px/1.45 Georgia, serif;
    color: #5f4636;
  }

  .tr-span {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 11px;
    color: var(--muted);
  }

  .tr-span input {
    width: 140px;
    accent-color: var(--ember);
  }

  .tr-span b {
    min-width: 3ch;
    font-weight: 500;
    color: var(--ink);
  }

  /* ── просмотр и параметры ────────────────────────────────────────── */

  .se-side {
    position: sticky;
    top: 12px;
    padding: 18px 18px 20px;
    border: 1px solid var(--line);
    background: rgba(255, 255, 255, 0.4);
  }

  .preview {
    width: var(--w, 250px);
    max-width: 100%;
    margin: 0 auto;
  }

  .scrub {
    display: grid;
    gap: 6px;
    margin-top: 14px;
    font-size: 11px;
    color: var(--muted);
  }

  .scrub b {
    font-weight: 500;
    color: var(--ink);
  }

  .scrub input,
  .pos input {
    width: 100%;
    accent-color: var(--ember);
  }

  .side-block {
    margin-top: 20px;
    padding-top: 16px;
    border-top: 1px solid var(--line);
  }

  .seg {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    border: 1px solid rgba(52, 37, 28, 0.2);
  }

  .seg button {
    padding: 7px 0;
    border: 0;
    border-right: 1px solid rgba(52, 37, 28, 0.2);
    background: none;
    font: inherit;
    font-size: 12px;
    color: var(--muted);
    cursor: pointer;
  }

  .seg button:last-child {
    border-right: 0;
  }

  .seg button.on {
    background: var(--ink);
    color: #f8f1e7;
  }

  .pos {
    display: grid;
    grid-template-columns: 28px 1fr;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--muted);
  }
</style>
