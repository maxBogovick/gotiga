<script lang="ts">
  import { onMount } from 'svelte';
  import { api, resolveMediaUrl } from '$lib/api';
  import { t } from '$lib/i18n';
  import { edgeMask, NO_EDGE } from '$lib/headerPortrait';
  import type {
    HeaderPortrait,
    HeaderPortraitAsset,
    HeaderPortraitCrop,
    HeaderPortraitPicture,
    HeaderPortraitPlace,
  } from '$lib/types/api';

  // Высота свёрнутой шапки на компьютере; по ней строится превью.
  const HEADER_H = 58;
  // Готовая картинка не выше этого: шапке хватает 3× от 58 px, остальное — лишние байты.
  const OUT_MAX_H = 300;
  const STAGE_MAX_W = 1100;

  type Kind = 'compositions' | 'photos';
  type Mode = 'photo' | 'frame';

  let data = $state<HeaderPortrait>({ compositions: [], photos: [], assets: [], activeId: null, edge: { ...NO_EDGE } });
  let loading = $state(true);
  let saving = $state(false);
  let uploading = $state<Kind | null>(null);
  let message = $state('');
  let messageType = $state<'ok' | 'err'>('ok');

  let compId = $state<string | null>(null);
  let photoId = $state<string | null>(null);
  let compBmp = $state.raw<ImageBitmap | null>(null);
  let photoBmp = $state.raw<ImageBitmap | null>(null);

  let place = $state<HeaderPortraitPlace>(defaultPlace());
  let crop = $state<HeaderPortraitCrop>({ x: 0, y: 0, w: 1, h: 1 });
  // null — кадр той же формы, что композиция.
  let frameRatio = $state<number | null>(null);
  let frameScale = $state(1);
  let mode = $state<Mode>('photo');
  let name = $state('');
  let editingId = $state<string | null>(null);

  let stageEl = $state<HTMLDivElement | null>(null);
  let stageCanvas = $state<HTMLCanvasElement | null>(null);
  let stripCanvas = $state<HTMLCanvasElement | null>(null);

  const bitmaps = new Map<string, Promise<ImageBitmap | null>>();

  const ASPECTS: { label: string; ratio: number | null }[] = [
    { label: '', ratio: null },
    { label: '1:1', ratio: 1 },
    { label: '3:2', ratio: 3 / 2 },
    { label: '2:1', ratio: 2 },
    { label: '3:1', ratio: 3 },
  ];

  let comp = $derived(data.compositions.find((c) => c.id === compId) ?? null);
  let photo = $derived(data.photos.find((p) => p.id === photoId) ?? null);
  let ready = $derived(!!compBmp && !!photoBmp);

  function defaultPlace(): HeaderPortraitPlace {
    return { x: 0.5, y: 1, size: 0.3, flip: false, fadeSides: 0, fadeBottom: 0, shadow: 0 };
  }

  const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

  function say(text: string, type: 'ok' | 'err' = 'ok') {
    message = text;
    messageType = type;
  }

  onMount(async () => {
    try {
      const d = await api.adminGetHeaderPortrait();
      data = {
        compositions: d.compositions ?? [],
        photos: d.photos ?? [],
        assets: d.assets ?? [],
        activeId: d.activeId ?? null,
        edge: { ...NO_EDGE, ...d.edge },
      };
    } catch {
      // пустой набор
    }
    loading = false;
  });

  // ── картинки ──────────────────────────────────────────────────────────

  // Адрес `/static/…` просится у своего origin: чужой сделал бы холст «грязным»,
  // и toBlob отказал бы (тот же приём, что у открытки байки).
  function local(u: string): string {
    try {
      const x = new URL(u, location.href);
      if (x.origin === location.origin) return x.href;
      return x.pathname.startsWith('/static/') ? x.pathname : x.href;
    } catch {
      return u;
    }
  }

  function loadBitmap(path: string): Promise<ImageBitmap | null> {
    let p = bitmaps.get(path);
    if (!p) {
      const url = resolveMediaUrl(path);
      p = url
        ? fetch(local(url), { credentials: 'omit' })
            .then((r) => (r.ok ? r.blob() : Promise.reject(new Error('load'))))
            .then((b) => createImageBitmap(b))
            .catch(() => null)
        : Promise.resolve(null);
      bitmaps.set(path, p);
    }
    return p;
  }

  async function pickComposition(id: string | null, keepFrame = false) {
    compId = id;
    compBmp = null;
    const c = data.compositions.find((x) => x.id === id);
    if (!c) return;
    if (!keepFrame) setFrame(null, 1);
    const bmp = await loadBitmap(c.image);
    if (compId !== id) return;
    compBmp = bmp;
    if (!bmp) say($t('adminPortraitLoadFailed'), 'err');
  }

  async function pickPhoto(id: string | null, keepPlace = false) {
    photoId = id;
    photoBmp = null;
    const p = data.photos.find((x) => x.id === id);
    if (!p) return;
    if (!keepPlace && comp) {
      // Фото почти во всю высоту композиции, стоит внизу по центру.
      const size = (0.85 * comp.height * (p.width / p.height)) / comp.width;
      place = { ...defaultPlace(), size: clamp(size, 0.05, 1) };
    }
    const bmp = await loadBitmap(p.image);
    if (photoId !== id) return;
    photoBmp = bmp;
    if (!bmp) say($t('adminPortraitLoadFailed'), 'err');
  }

  // ── кадр ──────────────────────────────────────────────────────────────

  function frameMax(ratio: number): { w: number; h: number } {
    if (!comp) return { w: 1, h: 1 };
    let w = (ratio * comp.height) / comp.width;
    let h = 1;
    if (w > 1) {
      w = 1;
      h = comp.width / ratio / comp.height;
    }
    return { w, h };
  }

  function compRatio(): number {
    return comp ? comp.width / comp.height : 1;
  }

  function setFrame(ratio: number | null, scale: number, cx?: number, cy?: number) {
    frameRatio = ratio;
    frameScale = scale;
    const m = frameMax(ratio ?? compRatio());
    const w = m.w * scale;
    const h = m.h * scale;
    const ccx = cx ?? crop.x + crop.w / 2;
    const ccy = cy ?? crop.y + crop.h / 2;
    crop = {
      x: clamp(ccx - w / 2, 0, 1 - w),
      y: clamp(ccy - h / 2, 0, 1 - h),
      w,
      h,
    };
  }

  // ── рисование ─────────────────────────────────────────────────────────

  /**
   * Рисует часть композиции `cr` в холст `ctx` размером outW × outH и ставит
   * в неё фото. Цвета фото не меняются: края растворяются только по альфе.
   */
  function drawScene(
    ctx: CanvasRenderingContext2D,
    outW: number,
    outH: number,
    cr: HeaderPortraitCrop,
    c: ImageBitmap,
    p: ImageBitmap | null,
    pl: HeaderPortraitPlace,
  ) {
    ctx.clearRect(0, 0, outW, outH);
    ctx.imageSmoothingQuality = 'high';
    ctx.drawImage(c, cr.x * c.width, cr.y * c.height, cr.w * c.width, cr.h * c.height, 0, 0, outW, outH);
    if (!p) return;

    const k = outW / (cr.w * c.width);
    const pw = Math.max(1, Math.round(pl.size * c.width * k));
    const ph = Math.max(1, Math.round((pw * p.height) / p.width));
    if (pw > 6000 || ph > 6000) return;
    const left = (pl.x * c.width - cr.x * c.width) * k - pw / 2;
    const top = (pl.y * c.height - cr.y * c.height) * k - ph;

    const off = document.createElement('canvas');
    off.width = pw;
    off.height = ph;
    const o = off.getContext('2d');
    if (!o) return;
    o.imageSmoothingQuality = 'high';
    if (pl.flip) {
      o.translate(pw, 0);
      o.scale(-1, 1);
    }
    o.drawImage(p, 0, 0, pw, ph);
    o.setTransform(1, 0, 0, 1, 0, 0);
    o.globalCompositeOperation = 'destination-in';
    if (pl.fadeSides > 0) {
      const f = Math.min(0.5, pl.fadeSides / 100);
      const g = o.createLinearGradient(0, 0, pw, 0);
      g.addColorStop(0, 'rgba(0,0,0,0)');
      g.addColorStop(f, 'rgba(0,0,0,1)');
      g.addColorStop(1 - f, 'rgba(0,0,0,1)');
      g.addColorStop(1, 'rgba(0,0,0,0)');
      o.fillStyle = g;
      o.fillRect(0, 0, pw, ph);
    }
    if (pl.fadeBottom > 0) {
      const f = Math.min(1, pl.fadeBottom / 100);
      const g = o.createLinearGradient(0, 0, 0, ph);
      g.addColorStop(0, 'rgba(0,0,0,1)');
      g.addColorStop(1 - f, 'rgba(0,0,0,1)');
      g.addColorStop(1, 'rgba(0,0,0,0)');
      o.fillStyle = g;
      o.fillRect(0, 0, pw, ph);
    }

    ctx.save();
    if (pl.shadow > 0) {
      ctx.shadowColor = `rgba(24, 12, 4, ${(pl.shadow / 100) * 0.75})`;
      ctx.shadowBlur = pw * 0.06;
      ctx.shadowOffsetY = pw * 0.012;
    }
    ctx.drawImage(off, left, top);
    ctx.restore();
  }

  $effect(() => {
    const canvas = stageCanvas;
    const c = compBmp;
    if (!canvas || !c) return;
    const w = Math.min(c.width, STAGE_MAX_W);
    const h = Math.round((w * c.height) / c.width);
    if (canvas.width !== w) canvas.width = w;
    if (canvas.height !== h) canvas.height = h;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    drawScene(ctx, w, h, { x: 0, y: 0, w: 1, h: 1 }, c, photoBmp, place);
  });

  $effect(() => {
    const canvas = stripCanvas;
    const c = compBmp;
    if (!canvas || !c) return;
    const ratio = (crop.w * c.width) / (crop.h * c.height);
    const h = HEADER_H * 2;
    const w = Math.max(1, Math.round(h * ratio));
    if (canvas.width !== w) canvas.width = w;
    if (canvas.height !== h) canvas.height = h;
    canvas.style.width = `${w / 2}px`;
    canvas.style.height = `${HEADER_H}px`;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    drawScene(ctx, w, h, crop, c, photoBmp, place);
  });

  // ── мышь ──────────────────────────────────────────────────────────────

  let drag: { x: number; y: number } | null = null;

  function down(e: PointerEvent) {
    if (!ready) return;
    drag = { x: e.clientX, y: e.clientY };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function move(e: PointerEvent) {
    if (!drag || !stageEl) return;
    const r = stageEl.getBoundingClientRect();
    const dx = (e.clientX - drag.x) / r.width;
    const dy = (e.clientY - drag.y) / r.height;
    drag = { x: e.clientX, y: e.clientY };
    if (mode === 'photo') {
      place.x = clamp(place.x + dx, -0.5, 1.5);
      place.y = clamp(place.y + dy, -0.5, 1.5);
    } else {
      setFrame(frameRatio, frameScale, crop.x + crop.w / 2 + dx, crop.y + crop.h / 2 + dy);
    }
  }

  function up(e: PointerEvent) {
    drag = null;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // захвата уже нет
    }
  }

  function wheelAction(node: HTMLElement) {
    const onWheel = (e: WheelEvent) => {
      if (!ready) return;
      e.preventDefault();
      const f = Math.exp(-e.deltaY * 0.0015);
      if (mode === 'photo') place.size = clamp(place.size * f, 0.02, 4);
      else setFrame(frameRatio, clamp(frameScale * f, 0.1, 1));
    };
    node.addEventListener('wheel', onWheel, { passive: false });
    return { destroy: () => node.removeEventListener('wheel', onWheel) };
  }

  // ── набор ─────────────────────────────────────────────────────────────

  async function persist(next: HeaderPortrait): Promise<boolean> {
    saving = true;
    try {
      const saved = await api.saveHeaderPortrait($state.snapshot(next) as HeaderPortrait);
      data = {
        compositions: saved.compositions ?? [],
        photos: saved.photos ?? [],
        assets: saved.assets ?? [],
        activeId: saved.activeId ?? null,
        edge: { ...NO_EDGE, ...saved.edge },
      };
      return true;
    } catch {
      say($t('adminPortraitFailed'), 'err');
      return false;
    } finally {
      saving = false;
    }
  }

  function snapshot(): HeaderPortrait {
    return $state.snapshot(data) as HeaderPortrait;
  }

  function upload(kind: Kind) {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'image/png,image/webp,image/jpeg';
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      uploading = kind;
      message = '';
      try {
        const art = await api.adminUploadBattleFrameArt(file);
        const pic: HeaderPortraitPicture = {
          id: crypto.randomUUID(),
          name: file.name.replace(/\.[^.]+$/, '').slice(0, 80),
          image: art.path,
          width: art.width,
          height: art.height,
        };
        const next = snapshot();
        next[kind] = [...next[kind], pic];
        if (await persist(next)) {
          if (kind === 'compositions') await pickComposition(pic.id);
          else await pickPhoto(pic.id);
        }
      } catch {
        say($t('adminPortraitUploadFailed'), 'err');
      } finally {
        uploading = null;
      }
    };
    input.click();
  }

  async function removePicture(kind: Kind, id: string) {
    if (!confirm($t('adminPortraitConfirm'))) return;
    const next = snapshot();
    next[kind] = next[kind].filter((p) => p.id !== id);
    if (await persist(next)) {
      if (kind === 'compositions' && compId === id) {
        compId = null;
        compBmp = null;
      }
      if (kind === 'photos' && photoId === id) {
        photoId = null;
        photoBmp = null;
      }
    }
  }

  // ── готовые картинки ──────────────────────────────────────────────────

  function render(): HTMLCanvasElement | null {
    if (!compBmp || !photoBmp) return null;
    const pxW = crop.w * compBmp.width;
    const pxH = crop.h * compBmp.height;
    const sc = Math.min(1, OUT_MAX_H / pxH);
    const w = Math.max(1, Math.round(pxW * sc));
    const h = Math.max(1, Math.round(pxH * sc));
    const canvas = document.createElement('canvas');
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;
    drawScene(ctx, w, h, crop, compBmp, photoBmp, place);
    return canvas;
  }

  async function saveAsset() {
    if (!ready || !compId || !photoId) return;
    const canvas = render();
    if (!canvas) return;
    saving = true;
    message = '';
    try {
      const blob = await new Promise<Blob | null>((res) => canvas.toBlob(res, 'image/png'));
      if (!blob) throw new Error('blob');
      const art = await api.adminUploadBattleFrameArt(
        new File([blob], 'header-portrait.png', { type: 'image/png' }),
      );
      const asset: HeaderPortraitAsset = {
        id: editingId ?? crypto.randomUUID(),
        name: name.trim() || `${$t('adminPortraitNameDefault')} ${data.assets.length + 1}`,
        image: art.path,
        width: art.width,
        height: art.height,
        compositionId: compId,
        photoId,
        place: $state.snapshot(place) as HeaderPortraitPlace,
        crop: $state.snapshot(crop) as HeaderPortraitCrop,
      };
      const next = snapshot();
      const i = next.assets.findIndex((a) => a.id === asset.id);
      if (i >= 0) next.assets[i] = asset;
      else next.assets = [...next.assets, asset];
      if (await persist(next)) {
        editingId = asset.id;
        name = asset.name;
        say($t('adminPortraitSaved'));
      }
    } catch {
      say($t('adminPortraitFailed'), 'err');
    } finally {
      saving = false;
    }
  }

  async function setActive(id: string | null) {
    const next = snapshot();
    next.activeId = id;
    if (await persist(next)) say($t('adminPortraitSaved'));
  }

  async function removeAsset(id: string) {
    if (!confirm($t('adminPortraitConfirm'))) return;
    const next = snapshot();
    next.assets = next.assets.filter((a) => a.id !== id);
    if (next.activeId === id) next.activeId = null;
    if (await persist(next) && editingId === id) editingId = null;
  }

  async function editAsset(a: HeaderPortraitAsset) {
    editingId = a.id;
    name = a.name;
    if (a.place) place = { ...a.place };
    await pickComposition(a.compositionId ?? null, true);
    await pickPhoto(a.photoId ?? null, true);
    if (a.crop && comp) {
      const r = (a.crop.w * comp.width) / (a.crop.h * comp.height);
      const whole = Math.abs(r - compRatio()) < 0.01;
      const preset = ASPECTS.find((p) => p.ratio !== null && Math.abs(p.ratio - r) < 0.01);
      frameRatio = whole ? null : preset ? preset.ratio : r;
      const m = frameMax(frameRatio ?? compRatio());
      frameScale = clamp(a.crop.w / m.w, 0.1, 1);
      crop = { ...a.crop };
    }
  }

  function startNew() {
    editingId = null;
    name = '';
    if (comp) setFrame(null, 1);
    if (comp && photo) {
      const size = (0.85 * comp.height * (photo.width / photo.height)) / comp.width;
      place = { ...defaultPlace(), size: clamp(size, 0.05, 1) };
    }
  }

  const url = (p: string) => resolveMediaUrl(p) ?? '';

  let activeAsset = $derived(data.assets.find((a) => a.id === data.activeId) ?? null);
  let edge = $derived({ ...NO_EDGE, ...data.edge });

  function setEdge(patch: Partial<typeof NO_EDGE>) {
    data.edge = { ...NO_EDGE, ...data.edge, ...patch };
    message = '';
  }

  async function saveEdge() {
    if (await persist(snapshot())) say($t('adminPortraitSaved'));
  }
</script>

<div class="h-full overflow-y-auto p-6">
  <div class="max-w-4xl mx-auto space-y-8">
    <div>
      <h2 class="font-display text-lg text-[#34251c]">{$t('adminPortraitTitle')}</h2>
      <p class="text-[11px] text-[#5f4636] mt-1">{$t('adminPortraitDesc')}</p>
    </div>

    {#if loading}
      <p class="text-[11px] text-[#5f4636]">…</p>
    {:else}
      <!-- Наборы -->
      {#each [['compositions', $t('adminPortraitCompositions'), $t('adminPortraitCompositionsHint')], ['photos', $t('adminPortraitPhotos'), $t('adminPortraitPhotosHint')]] as [kind, title, hint] (kind)}
        {@const k = kind as Kind}
        {@const list = data[k]}
        {@const picked = k === 'compositions' ? compId : photoId}
        <section class="border border-[#34251c]/10 p-4 space-y-3">
          <div class="flex flex-wrap items-baseline justify-between gap-2">
            <div>
              <p class="text-[10px] tracking-[0.08em] uppercase text-[#5f4636]">{title}</p>
              <p class="text-[10px] text-[#5f4636]/80">{hint}</p>
            </div>
            <button
              type="button"
              onclick={() => upload(k)}
              disabled={uploading !== null}
              class="btn-gothic text-[10px] px-3 py-1.5 disabled:opacity-60"
            >{uploading === k ? $t('adminPortraitUploading') : $t('adminPortraitUpload')}</button>
          </div>
          {#if list.length === 0}
            <p class="text-[10px] text-[#5f4636]/60">{$t('adminPortraitEmpty')}</p>
          {:else}
            <div class="flex flex-wrap gap-3">
              {#each list as pic (pic.id)}
                <div class="w-28">
                  <button
                    type="button"
                    onclick={() => (k === 'compositions' ? pickComposition(pic.id) : pickPhoto(pic.id))}
                    class="block w-full h-20 border overflow-hidden relative {picked === pic.id ? 'border-[#c65f3c] ring-2 ring-[#c65f3c]/40' : 'border-[#34251c]/20'}"
                    style={k === 'photos'
                      ? 'background: repeating-conic-gradient(#e7dccb 0% 25%, #f8f1e7 0% 50%) 50% / 12px 12px;'
                      : 'background: #f8f1e7;'}
                    title={pic.name}
                  >
                    <img src={url(pic.image)} alt={pic.name} class="w-full h-full object-contain" loading="lazy" />
                  </button>
                  <div class="flex items-center justify-between gap-1 mt-1">
                    <span class="text-[10px] text-[#5f4636] truncate" title={pic.name}>{pic.name}</span>
                    <button
                      type="button"
                      onclick={() => removePicture(k, pic.id)}
                      class="text-[10px] text-[#9b2c1c] hover:underline shrink-0"
                    >{$t('adminPortraitDelete')}</button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </section>
      {/each}

      <!-- Сборка -->
      <section class="border border-[#34251c]/10 p-4 space-y-4">
        <p class="text-[10px] tracking-[0.08em] uppercase text-[#5f4636]">{$t('adminPortraitBuilder')}</p>

        {#if !comp || !photo}
          <p class="text-[11px] text-[#5f4636]/70">{$t('adminPortraitBuilderPick')}</p>
        {:else}
          <div class="flex flex-wrap gap-2">
            {#each [['photo', $t('adminPortraitModePhoto')], ['frame', $t('adminPortraitModeFrame')]] as [m, label] (m)}
              <button
                type="button"
                onclick={() => (mode = m as Mode)}
                class="text-[10px] uppercase tracking-wide px-3 py-1.5 border {mode === m ? 'border-[#34251c]/60 text-[#34251c] bg-[#34251c]/5' : 'border-[#34251c]/15 text-[#5f4636]'}"
              >{label}</button>
            {/each}
          </div>

          <!-- Холст: тянуть — двигает то, что выбрано режимом; колесо — размер -->
          <div
            bind:this={stageEl}
            use:wheelAction
            class="relative w-full overflow-hidden select-none touch-none border border-[#34251c]/20 bg-[#f8f1e7] {ready ? (mode === 'photo' ? 'cursor-grab active:cursor-grabbing' : 'cursor-move') : ''}"
            style="max-width: {STAGE_MAX_W}px; aspect-ratio: {comp.width} / {comp.height};"
            onpointerdown={down}
            onpointermove={move}
            onpointerup={up}
            onpointercancel={up}
            role="presentation"
          >
            <canvas bind:this={stageCanvas} class="absolute inset-0 w-full h-full block pointer-events-none"></canvas>
            {#if ready}
              <div
                class="absolute pointer-events-none"
                style="
                  left: {crop.x * 100}%; top: {crop.y * 100}%;
                  width: {crop.w * 100}%; height: {crop.h * 100}%;
                  outline: 2px dashed rgba(255,255,255,0.95);
                  box-shadow: 0 0 0 1px rgba(52,37,28,0.7), 0 0 0 9999px rgba(20,12,6,0.45);
                "
              ></div>
            {/if}
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label for="pt-size" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">
                {$t('adminPortraitSize')}: {Math.round(place.size * 100)}%
              </label>
              <input id="pt-size" type="range" min="2" max="150" step="1" class="w-full"
                value={Math.round(place.size * 100)}
                oninput={(e) => (place.size = Number(e.currentTarget.value) / 100)} />
            </div>
            <div>
              <label for="pt-shadow" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">
                {$t('adminPortraitShadow')}: {place.shadow}%
              </label>
              <input id="pt-shadow" type="range" min="0" max="100" step="5" class="w-full"
                value={place.shadow}
                oninput={(e) => (place.shadow = Number(e.currentTarget.value))} />
            </div>
            <div>
              <label for="pt-sides" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">
                {$t('adminPortraitFadeSides')}: {place.fadeSides}%
              </label>
              <input id="pt-sides" type="range" min="0" max="50" step="1" class="w-full"
                value={place.fadeSides}
                oninput={(e) => (place.fadeSides = Number(e.currentTarget.value))} />
            </div>
            <div>
              <label for="pt-bottom" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">
                {$t('adminPortraitFadeBottom')}: {place.fadeBottom}%
              </label>
              <input id="pt-bottom" type="range" min="0" max="100" step="1" class="w-full"
                value={place.fadeBottom}
                oninput={(e) => (place.fadeBottom = Number(e.currentTarget.value))} />
            </div>
            <label class="flex items-center gap-2 text-[11px] text-[#34251c]">
              <input type="checkbox" checked={place.flip} onchange={(e) => (place.flip = e.currentTarget.checked)} />
              {$t('adminPortraitFlip')}
            </label>
          </div>

          <div class="space-y-2">
            <p class="text-[10px] tracking-wide text-[#5f4636]">{$t('adminPortraitFrame')}</p>
            <div class="flex flex-wrap items-center gap-2">
              {#each ASPECTS as a (a.label)}
                <button
                  type="button"
                  onclick={() => setFrame(a.ratio, 1)}
                  class="text-[10px] uppercase tracking-wide px-3 py-1.5 border {frameRatio === a.ratio ? 'border-[#34251c]/60 text-[#34251c] bg-[#34251c]/5' : 'border-[#34251c]/15 text-[#5f4636]'}"
                >{a.label || $t('adminPortraitFrameWhole')}</button>
              {/each}
            </div>
            <div class="max-w-sm">
              <label for="pt-frame" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">
                {$t('adminPortraitFrameSize')}: {Math.round(frameScale * 100)}%
              </label>
              <input id="pt-frame" type="range" min="10" max="100" step="1" class="w-full"
                value={Math.round(frameScale * 100)}
                oninput={(e) => setFrame(frameRatio, Number(e.currentTarget.value) / 100)} />
            </div>
          </div>

          <!-- Как это выглядит в свёрнутой шапке -->
          <div>
            <p class="text-[10px] tracking-wide text-[#5f4636] mb-1">{$t('adminPortraitPreview')}</p>
            <div
              class="flex items-center border border-[#34251c]/10 overflow-hidden"
              style="height: {HEADER_H}px; background: rgba(248, 241, 231, 0.97);"
            >
              <canvas bind:this={stripCanvas} class="block shrink-0"></canvas>
              <span class="ml-5 pl-5 border-l border-[#34251c]/20 font-display text-[#34251c] text-lg leading-none">Ritunia</span>
            </div>
          </div>

          <div class="flex flex-wrap items-end gap-3">
            <div>
              <label for="pt-name" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">{$t('adminPortraitName')}</label>
              <input id="pt-name" type="text" maxlength="80" bind:value={name}
                class="border border-[#34251c]/20 bg-white/60 px-2 py-1.5 text-[12px] text-[#34251c] w-56" />
            </div>
            <button
              type="button"
              onclick={saveAsset}
              disabled={saving || !ready}
              class="btn-gothic text-[10px] px-4 py-2 disabled:opacity-50"
            >{saving ? $t('adminPortraitSaving') : editingId ? $t('adminPortraitUpdateAsset') : $t('adminPortraitSaveAsset')}</button>
            {#if editingId}
              <button
                type="button"
                onclick={startNew}
                class="text-[10px] uppercase tracking-wide text-[#5f4636] hover:text-[#34251c] px-2 py-2"
              >{$t('adminPortraitNewAsset')}</button>
            {/if}
            {#if message}
              <span class="text-[11px] {messageType === 'ok' ? 'text-[#5f4636]' : 'text-[#9b2c1c]'}">{message}</span>
            {/if}
          </div>
          <p class="text-[10px] text-[#5f4636]/80">{$t('adminPortraitHint')}</p>
        {/if}
      </section>

      <!-- Готовые картинки -->
      <section class="border border-[#34251c]/10 p-4 space-y-3">
        <p class="text-[10px] tracking-[0.08em] uppercase text-[#5f4636]">{$t('adminPortraitAssets')}</p>
        {#if data.assets.length === 0}
          <p class="text-[10px] text-[#5f4636]/60">{$t('adminPortraitAssetsEmpty')}</p>
        {:else}
          <div class="space-y-3">
            {#each data.assets as a (a.id)}
              <div class="flex flex-wrap items-center gap-4 border border-[#34251c]/10 p-2 {data.activeId === a.id ? 'bg-[#c65f3c]/5' : ''}">
                <img
                  src={url(a.image)}
                  alt={a.name}
                  class="block border border-[#34251c]/10"
                  style="height: {HEADER_H}px; width: auto; background: rgba(248,241,231,0.97);"
                  loading="lazy"
                />
                <div class="min-w-0 flex-1">
                  <p class="text-[12px] text-[#34251c] truncate">{a.name}</p>
                  {#if data.activeId === a.id}
                    <p class="text-[10px] uppercase tracking-wide text-[#a4492a]">{$t('adminPortraitInHeader')}</p>
                  {/if}
                </div>
                <div class="flex flex-wrap items-center gap-2">
                  {#if data.activeId === a.id}
                    <button type="button" onclick={() => setActive(null)} disabled={saving}
                      class="text-[10px] uppercase tracking-wide border border-[#34251c]/20 px-3 py-1.5 text-[#5f4636] hover:text-[#34251c]">
                      {$t('adminPortraitClearActive')}
                    </button>
                  {:else}
                    <button type="button" onclick={() => setActive(a.id)} disabled={saving} class="btn-gothic text-[10px] px-3 py-1.5">
                      {$t('adminPortraitSetActive')}
                    </button>
                  {/if}
                  {#if a.compositionId && a.photoId}
                    <button type="button" onclick={() => editAsset(a)}
                      class="text-[10px] uppercase tracking-wide text-[#5f4636] hover:text-[#34251c] px-2 py-1.5">
                      {$t('adminPortraitEdit')}
                    </button>
                  {/if}
                  <button type="button" onclick={() => removeAsset(a.id)}
                    class="text-[10px] uppercase tracking-wide text-[#9b2c1c] hover:underline px-2 py-1.5">
                    {$t('adminPortraitDelete')}
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </section>

      <!-- Края картинки в шапке -->
      <section class="border border-[#34251c]/10 p-4 space-y-4">
        <div>
          <p class="text-[10px] tracking-[0.08em] uppercase text-[#5f4636]">{$t('adminPortraitEdgeTitle')}</p>
          <p class="text-[10px] text-[#5f4636]/80">{$t('adminPortraitEdgeDesc')}</p>
        </div>

        {#if activeAsset}
          <div
            class="flex items-center border border-[#34251c]/10 overflow-hidden"
            style="height: {HEADER_H}px; background: rgba(248, 241, 231, 0.97);"
          >
            <img
              src={url(activeAsset.image)}
              alt=""
              class="block shrink-0"
              style="
                height: {HEADER_H}px; width: auto;
                margin-left: {edge.gapLeft}px; margin-right: {edge.gapRight}px;
                {edgeMask(edge) ? `-webkit-mask-image: ${edgeMask(edge)}; mask-image: ${edgeMask(edge)};` : ''}
              "
            />
            <span class="pl-5 border-l border-[#34251c]/20 font-display text-[#34251c] text-lg leading-none">Ritunia</span>
          </div>
        {:else}
          <p class="text-[11px] text-[#5f4636]/70">{$t('adminPortraitEdgeNone')}</p>
        {/if}

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div>
            <label for="pt-gap-l" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">
              {$t('adminPortraitEdgeGapLeft')}: {edge.gapLeft}px
            </label>
            <input id="pt-gap-l" type="range" min="0" max="200" step="2" class="w-full"
              value={edge.gapLeft}
              oninput={(e) => setEdge({ gapLeft: Number(e.currentTarget.value) })} />
          </div>
          <div>
            <label for="pt-gap-r" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">
              {$t('adminPortraitEdgeGapRight')}: {edge.gapRight}px
            </label>
            <input id="pt-gap-r" type="range" min="0" max="200" step="2" class="w-full"
              value={edge.gapRight}
              oninput={(e) => setEdge({ gapRight: Number(e.currentTarget.value) })} />
          </div>
          <div>
            <label for="pt-fade-l" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">
              {$t('adminPortraitEdgeFadeLeft')}: {edge.fadeLeft}%
            </label>
            <input id="pt-fade-l" type="range" min="0" max="50" step="1" class="w-full"
              value={edge.fadeLeft}
              oninput={(e) => setEdge({ fadeLeft: Number(e.currentTarget.value) })} />
          </div>
          <div>
            <label for="pt-fade-r" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">
              {$t('adminPortraitEdgeFadeRight')}: {edge.fadeRight}%
            </label>
            <input id="pt-fade-r" type="range" min="0" max="50" step="1" class="w-full"
              value={edge.fadeRight}
              oninput={(e) => setEdge({ fadeRight: Number(e.currentTarget.value) })} />
          </div>
        </div>

        <div class="flex items-center gap-3">
          <button type="button" onclick={saveEdge} disabled={saving} class="btn-gothic text-[10px] px-4 py-2 disabled:opacity-50">
            {saving ? $t('adminPortraitSaving') : $t('adminPortraitEdgeSave')}
          </button>
          {#if message}
            <span class="text-[11px] {messageType === 'ok' ? 'text-[#5f4636]' : 'text-[#9b2c1c]'}">{message}</span>
          {/if}
        </div>
      </section>
    {/if}
  </div>
</div>
