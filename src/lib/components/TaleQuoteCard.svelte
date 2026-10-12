<script lang="ts">
  // Открытка из байки: читатель выделяет фразу, под выделением появляется
  // «Открытка», и фраза становится картинкой — фото работы, а на нём панель
  // матового стекла с фразой, заглавием байки и адресом. Без фото (не
  // загрузилось, у байки его нет) — прежняя открытка на пергаменте. Её посылают в Telegram и кладут на доску Pinterest; по адресу
  // на ней и по ссылке (`?src=tale_card`) видно, сколько людей она привела.
  //
  // Рисуется на canvas, а не снимается с экрана: открытка не зависит от того,
  // какая у читателя ширина окна, тема и сколько абзацев сейчас приглушено
  // свечой. Шрифт тот же, что у прозы, и ждём мы его явно — canvas, в отличие
  // от разметки, не перерисуется, когда шрифт доедет.
  import { t, lang } from '$lib/i18n';
  import { portal } from '$lib/actions/portal';
  import { interact } from '$lib/analytics';

  let {
    root,
    title,
    address,
    link,
    slug,
    photos = [],
  }: {
    /** Текст байки: выделение за его пределами открытки не предлагает. */
    root: HTMLElement | undefined;
    title: string;
    /** Адрес, как он напечатан на открытке: без схемы и без метки. */
    address: string;
    /** Адрес, который уходит в «Поделиться» и «Скопировать»: с `?src=`. */
    link: string;
    slug: string;
    /** Адреса фото работы от крупного к мелкому: берётся первое, что отдалось. */
    photos?: string[];
  } = $props();

  /** Короче — это слово, не фраза; длиннее — шрифт стал бы мельче подписи. */
  const MIN = 12;
  const MAX = 320;

  type Pick = { text: string; x: number; y: number; below: boolean; tooLong: boolean };
  let pick = $state<Pick | null>(null);

  let box = $state<HTMLDialogElement>();
  type Look = 'glass' | 'paper';
  let made = $state<{ blob: Blob; src: string; name: string; quote: string } | null>(null);
  let style = $state<Look>('glass');
  let photo = $state<ImageBitmap | null>(null);
  let photoAsked = false;
  let busy = $state(false);
  let failed = $state(false);
  let copied = $state(false);

  // The text of a selection, without whatever sits between two paragraphs
  // (ornament, heading, the plate in the margin) and without the glue that
  // `textContent` leaves between them.
  function textOf(range: Range): string {
    const part = range.cloneContents();
    part.querySelectorAll('.ornament, .subhead, .tipped, .margin, .tale-stage, script, style').forEach((n) => n.remove());
    const paras = [...part.querySelectorAll('p')];
    const raw = paras.length ? paras.map((p) => p.textContent ?? '').join(' ') : (part.textContent ?? '');
    return raw.replace(/\s+/g, ' ').trim();
  }

  function inProse(node: Node | null): boolean {
    const el = node instanceof Element ? node : node?.parentElement;
    return !!el && !!root && root.contains(el) && !!el.closest('.para, .quote');
  }

  let frame = 0;
  function look() {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
      const sel = getSelection();
      if (!root || !sel || sel.isCollapsed || sel.rangeCount === 0 || busy || made) {
        pick = null;
        return;
      }
      if (!inProse(sel.anchorNode) || !inProse(sel.focusNode)) {
        pick = null;
        return;
      }
      const range = sel.getRangeAt(0);
      const text = textOf(range);
      if (text.length < MIN) {
        pick = null;
        return;
      }
      const r = range.getBoundingClientRect();
      // A finger's selection comes with the system's own bar above it, and a
      // mouse's does not; under the text on touch, above it everywhere else.
      const touch = matchMedia('(pointer: coarse)').matches;
      const below = touch || r.top < 56;
      pick = {
        text,
        x: Math.min(Math.max(r.left + r.width / 2, 96), innerWidth - 96),
        y: below ? r.bottom + (touch ? 26 : 10) : r.top - 10,
        below,
        tooLong: text.length > MAX,
      };
    });
  }

  $effect(() => {
    document.addEventListener('selectionchange', look);
    // A scroll moves the selection but fires no event of its own.
    addEventListener('scroll', look, { passive: true });
    return () => {
      document.removeEventListener('selectionchange', look);
      removeEventListener('scroll', look);
      cancelAnimationFrame(frame);
    };
  });

  const INK = '#34251c';
  const DEEP = '#6f3b24';
  const COPPER = '#c65f3c';
  const EDGE = '#d8c6b1';
  const PAPER = '#f8f1e7';
  const FACE = 'Georgia, serif';
  const SERIF = '"Cormorant Garamond", Georgia, serif';
  const W = 1080;
  const H = 1350;

  // The photograph is read through `fetch` and drawn as a bitmap: an <img>
  // from another origin would taint the canvas and `toBlob` would refuse. On
  // the site the picture is same-origin; in development it sits on the
  // production host, so its `/static/` path is asked from our own origin,
  // which the dev server forwards.
  function local(u: string): string {
    try {
      const x = new URL(u, location.href);
      if (x.origin === location.origin) return x.href;
      return x.pathname.startsWith('/static/') ? x.pathname : x.href;
    } catch {
      return u;
    }
  }

  async function ensurePhoto() {
    if (photoAsked || !photos.length) return;
    photoAsked = true;
    for (const u of photos) {
      try {
        const r = await fetch(local(u), { credentials: 'omit' });
        if (!r.ok) continue;
        photo = await createImageBitmap(await r.blob());
        return;
      } catch {
        // Next rendition; with none left the card is paper.
      }
    }
  }

  // Warm the photograph as soon as a selection first appears: the button is
  // pressed a second later, and the picture should already be here.
  $effect(() => {
    if (pick) void ensurePhoto();
  });

  /** Break `text` into lines no wider than `width` at the context's font. */
  function wrap(ctx: CanvasRenderingContext2D, text: string, width: number): string[] {
    const lines: string[] = [];
    let line = '';
    for (const word of text.split(' ')) {
      const next = line ? `${line} ${word}` : word;
      if (line && ctx.measureText(next).width > width) {
        lines.push(line);
        line = word;
      } else {
        line = next;
      }
    }
    if (line) lines.push(line);
    return lines;
  }

  /** The largest size at which the line fits the box. */
  function fit(ctx: CanvasRenderingContext2D, text: string, width: number, height: number, from: number) {
    let size = from;
    let lines: string[] = [];
    for (; size >= 34; size -= 2) {
      ctx.font = `italic 400 ${size}px ${SERIF}`;
      lines = wrap(ctx, text, width);
      if (lines.length * size * 1.3 <= height) break;
    }
    return { size, lines, lead: size * 1.3 };
  }

  /** The tale's name, shrunk until it stands on one line. */
  function fitTitle(ctx: CanvasRenderingContext2D, width: number): number {
    let size = 40;
    ctx.font = `500 ${size}px ${SERIF}`;
    while (ctx.measureText(title).width > width && size > 24) {
      size -= 2;
      ctx.font = `500 ${size}px ${SERIF}`;
    }
    return size;
  }

  /** A four-pointed star, the same ✦ the tale uses between its parts. */
  function star(ctx: CanvasRenderingContext2D, cx: number, cy: number, r: number) {
    const k = r * 0.16;
    ctx.beginPath();
    ctx.moveTo(cx, cy - r);
    ctx.quadraticCurveTo(cx + k, cy - k, cx + r, cy);
    ctx.quadraticCurveTo(cx + k, cy + k, cx, cy + r);
    ctx.quadraticCurveTo(cx - k, cy + k, cx - r, cy);
    ctx.quadraticCurveTo(cx - k, cy - k, cx, cy - r);
    ctx.closePath();
    ctx.fill();
  }

  function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.arcTo(x + w, y, x + w, y + h, r);
    ctx.arcTo(x + w, y + h, x, y + h, r);
    ctx.arcTo(x, y + h, x, y, r);
    ctx.arcTo(x, y, x + w, y, r);
    ctx.closePath();
  }

  function canvasOf(): [HTMLCanvasElement, CanvasRenderingContext2D] {
    const canvas = document.createElement('canvas');
    canvas.width = W;
    canvas.height = H;
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('no canvas');
    return [canvas, ctx];
  }

  /** Blur of a whole canvas. Safari draws no `filter`, so there it is a scale down and back up. */
  function blurred(src: HTMLCanvasElement): HTMLCanvasElement {
    const [out, o] = canvasOf();
    if (typeof o.filter === 'string') {
      o.filter = 'blur(36px)';
      // Drawn a little oversize: a blur fades to nothing at the canvas edge.
      o.drawImage(src, -60, -60, W + 120, H + 120);
      return out;
    }
    o.imageSmoothingEnabled = true;
    o.imageSmoothingQuality = 'high';
    let from: CanvasImageSource = src;
    for (const k of [4, 12, 28]) {
      const step = document.createElement('canvas');
      step.width = Math.ceil(W / k);
      step.height = Math.ceil(H / k);
      const c = step.getContext('2d');
      if (!c) break;
      c.imageSmoothingQuality = 'high';
      c.drawImage(from, 0, 0, step.width, step.height);
      from = step;
    }
    o.drawImage(from, 0, 0, W, H);
    return out;
  }

  /** Mean brightness 0..1 of a rectangle of a canvas. */
  function brightness(src: HTMLCanvasElement, x: number, y: number, w: number, h: number): number {
    const tiny = document.createElement('canvas');
    tiny.width = tiny.height = 12;
    const c = tiny.getContext('2d');
    if (!c) return 0.3;
    c.drawImage(src, x, y, w, h, 0, 0, 12, 12);
    const d = c.getImageData(0, 0, 12, 12).data;
    let sum = 0;
    for (let i = 0; i < d.length; i += 4) sum += (0.2126 * d[i] + 0.7152 * d[i + 1] + 0.0722 * d[i + 2]) / 255;
    return sum / (d.length / 4);
  }

  /** Fine grain, so the glass reads as a surface and not as a flat tint. */
  function grain(): CanvasPattern | null {
    const n = document.createElement('canvas');
    n.width = n.height = 160;
    const c = n.getContext('2d');
    if (!c) return null;
    const img = c.createImageData(160, 160);
    for (let i = 0; i < img.data.length; i += 4) {
      const v = Math.random() * 255;
      img.data[i] = img.data[i + 1] = img.data[i + 2] = v;
      img.data[i + 3] = 20;
    }
    c.putImageData(img, 0, 0);
    return c.createPattern(n, 'repeat');
  }

  function say(quote: string): string {
    const [open, close] = $lang === 'ru' ? ['«', '»'] : ['“', '”'];
    return `${open}${quote}${close}`;
  }

  function paintGlass(quote: string, img: ImageBitmap): HTMLCanvasElement {
    const [canvas, ctx] = canvasOf();

    // The photograph, filling the card and weighted to its upper part, where
    // the figure usually stands; the panel takes the lower part.
    const k = Math.max(W / img.width, H / img.height);
    const dw = img.width * k;
    const dh = img.height * k;
    ctx.drawImage(img, (W - dw) / 2, (H - dh) * 0.3, dw, dh);

    // Grade: a little darker towards the bottom and at the corners.
    const fall = ctx.createLinearGradient(0, 0, 0, H);
    fall.addColorStop(0, 'rgba(20,12,8,0.08)');
    fall.addColorStop(0.5, 'rgba(20,12,8,0.12)');
    fall.addColorStop(1, 'rgba(20,12,8,0.55)');
    ctx.fillStyle = fall;
    ctx.fillRect(0, 0, W, H);
    const vig = ctx.createRadialGradient(W / 2, H * 0.45, H * 0.35, W / 2, H * 0.5, H * 0.85);
    vig.addColorStop(0, 'rgba(0,0,0,0)');
    vig.addColorStop(1, 'rgba(0,0,0,0.38)');
    ctx.fillStyle = vig;
    ctx.fillRect(0, 0, W, H);

    // The panel is laid out first, from the text, so that its height follows the line.
    const px = 80;
    const pw = W - 2 * px;
    const padX = 84;
    const padT = 66;
    const padB = 60;
    const said = say(quote);
    // Compact on purpose: the figure stands in the upper half of most
    // photographs, and a tall pane would cover its face.
    const q = fit(ctx, said, pw - 2 * padX, 400, 60);
    const tsize = fitTitle(ctx, pw - 2 * padX);
    const ph = padT + 40 + 34 + q.lines.length * q.lead + 46 + 2 + 44 + tsize + 18 + 28 + padB;
    const py = H - 84 - ph;

    // Light or dark glass, by what lies under it.
    const under = blurred(canvas);
    const light = brightness(under, px, py, pw, ph) > 0.56;
    const ink = light ? '#2a1c14' : '#f8f1e7';
    const accent = light ? COPPER : '#ee9a70';
    const quiet = light ? '#6a533d' : 'rgba(248,241,231,0.72)';
    const name = light ? DEEP : '#ecd2b4';

    // Shadow, then the frosted pane: the blurred photograph, a tint, a
    // sheen, grain, and a rim that catches light at the top left.
    ctx.save();
    ctx.shadowColor = 'rgba(0,0,0,0.5)';
    ctx.shadowBlur = 70;
    ctx.shadowOffsetY = 26;
    ctx.fillStyle = '#000';
    roundRect(ctx, px, py, pw, ph, 44);
    ctx.fill();
    ctx.restore();

    ctx.save();
    roundRect(ctx, px, py, pw, ph, 44);
    ctx.clip();
    ctx.drawImage(under, 0, 0);
    ctx.fillStyle = light ? 'rgba(250,244,234,0.56)' : 'rgba(22,14,10,0.46)';
    ctx.fillRect(px, py, pw, ph);
    const sheen = ctx.createLinearGradient(px, py, px + pw * 0.8, py + ph);
    sheen.addColorStop(0, 'rgba(255,255,255,0.22)');
    sheen.addColorStop(0.45, 'rgba(255,255,255,0.03)');
    sheen.addColorStop(1, 'rgba(255,255,255,0.09)');
    ctx.fillStyle = sheen;
    ctx.fillRect(px, py, pw, ph);
    const g = grain();
    if (g) {
      ctx.fillStyle = g;
      ctx.fillRect(px, py, pw, ph);
    }
    ctx.restore();

    const rim = ctx.createLinearGradient(px, py, px + pw, py + ph);
    rim.addColorStop(0, 'rgba(255,255,255,0.7)');
    rim.addColorStop(0.4, 'rgba(255,255,255,0.14)');
    rim.addColorStop(1, 'rgba(255,255,255,0.3)');
    ctx.strokeStyle = rim;
    ctx.lineWidth = 2;
    roundRect(ctx, px + 1, py + 1, pw - 2, ph - 2, 43);
    ctx.stroke();

    // Text on the pane.
    ctx.textAlign = 'center';
    ctx.textBaseline = 'alphabetic';
    let y = py + padT;
    ctx.fillStyle = accent;
    star(ctx, W / 2, y + 18, 22);
    y += 40 + 34;

    ctx.fillStyle = ink;
    ctx.font = `italic 400 ${q.size}px ${SERIF}`;
    ctx.shadowColor = light ? 'rgba(255,255,255,0.35)' : 'rgba(0,0,0,0.4)';
    ctx.shadowBlur = 10;
    ctx.shadowOffsetY = 2;
    y += q.size * 0.95;
    for (const l of q.lines) {
      ctx.fillText(l, W / 2, y);
      y += q.lead;
    }
    ctx.shadowColor = 'transparent';
    ctx.shadowBlur = 0;
    ctx.shadowOffsetY = 0;
    y += 46 - q.size * 0.95 + q.size * 0.3;

    ctx.fillStyle = accent;
    ctx.globalAlpha = 0.7;
    ctx.fillRect(W / 2 - 36, y, 72, 2);
    ctx.globalAlpha = 1;
    y += 2 + 44 + tsize * 0.8;

    ctx.fillStyle = name;
    ctx.font = `500 ${tsize}px ${SERIF}`;
    ctx.fillText(title, W / 2, y);
    y += 18 + 22;
    ctx.fillStyle = quiet;
    ctx.font = `400 26px ${FACE}`;
    ctx.fillText(address, W / 2, y);
    return canvas;
  }

  function paintPaper(quote: string): HTMLCanvasElement {
    const [canvas, ctx] = canvasOf();
    const said = say(quote);

    // Paper: flat colour with a little warmth gathered towards the edges.
    ctx.fillStyle = PAPER;
    ctx.fillRect(0, 0, W, H);
    const warm = ctx.createRadialGradient(W / 2, H / 2, H * 0.3, W / 2, H / 2, H * 0.78);
    warm.addColorStop(0, 'rgba(216,198,177,0)');
    warm.addColorStop(1, 'rgba(216,198,177,0.55)');
    ctx.fillStyle = warm;
    ctx.fillRect(0, 0, W, H);

    // Double border, as on the house's windows.
    ctx.strokeStyle = EDGE;
    ctx.lineWidth = 3;
    ctx.strokeRect(48, 48, W - 96, H - 96);
    ctx.lineWidth = 1.5;
    ctx.strokeRect(64, 64, W - 128, H - 128);

    ctx.fillStyle = COPPER;
    star(ctx, W / 2, 190, 24);

    // The line itself: the biggest size at which it fits the field.
    const top = 270;
    const bottom = 1000;
    const q = fit(ctx, said, W - 2 * 170, bottom - top, 76);
    ctx.fillStyle = INK;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'alphabetic';
    let y = top + (bottom - top - q.lines.length * q.lead) / 2 + q.size * 0.95;
    ctx.font = `italic 400 ${q.size}px ${SERIF}`;
    for (const l of q.lines) {
      ctx.fillText(l, W / 2, y);
      y += q.lead;
    }

    // Under it: a short rule, the tale's name, the address.
    ctx.fillStyle = COPPER;
    ctx.globalAlpha = 0.6;
    ctx.fillRect(W / 2 - 36, 1060, 72, 2);
    ctx.globalAlpha = 1;

    ctx.fillStyle = DEEP;
    const tsize = fitTitle(ctx, W - 2 * 150);
    ctx.font = `500 ${tsize}px ${SERIF}`;
    ctx.fillText(title, W / 2, 1130);

    ctx.fillStyle = '#7b6247';
    ctx.font = `400 28px ${FACE}`;
    ctx.fillText(address, W / 2, 1186);
    return canvas;
  }

  async function render(how: Look, quote: string): Promise<Blob> {
    // The font must be here before the first measurement, or the lines are
    // wrapped for Georgia and drawn in Cormorant.
    await Promise.all([
      document.fonts.load(`italic 400 56px ${SERIF}`, say(quote)),
      document.fonts.load(`500 32px ${SERIF}`, title),
    ]);
    const canvas = how === 'glass' && photo ? paintGlass(quote, photo) : paintPaper(quote);
    return new Promise((resolve, reject) =>
      canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('no blob'))), 'image/jpeg', 0.92),
    );
  }

  function keep(blob: Blob, quote: string) {
    if (made) URL.revokeObjectURL(made.src);
    made = { blob, src: URL.createObjectURL(blob), name: `${slug}-card.jpg`, quote };
  }

  async function make() {
    if (!pick || pick.tooLong || busy) return;
    const quote = pick.text;
    busy = true;
    failed = false;
    copied = false;
    try {
      await ensurePhoto();
      keep(await render(photo ? style : 'paper', quote), quote);
      pick = null;
      getSelection()?.removeAllRanges();
      interact('tale_card_made', photo ? style : 'paper', quote.length);
      box?.showModal();
    } catch {
      failed = true;
    } finally {
      busy = false;
    }
  }

  async function restyle(how: Look) {
    if (!made || busy || how === style) return;
    busy = true;
    style = how;
    try {
      keep(await render(how, made.quote), made.quote);
    } catch {
      failed = true;
    } finally {
      busy = false;
    }
  }

  // A click on the backdrop lands on the dialog itself, outside its rectangle;
  // a click on its own padding lands there too, and must not close it.
  function away(e: MouseEvent) {
    if (e.target !== box || !box) return;
    const r = box.getBoundingClientRect();
    const inside = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
    if (!inside) box.close();
  }

  function shut() {
    if (made) URL.revokeObjectURL(made.src);
    made = null;
    copied = false;
  }

  let file = $derived(made ? new File([made.blob], made.name, { type: 'image/jpeg' }) : null);
  let canShare = $derived(
    !!file && typeof navigator !== 'undefined' && !!navigator.canShare && navigator.canShare({ files: [file] }),
  );

  async function share() {
    if (!file) return;
    interact('tale_card_out', 'share', null, false);
    try {
      await navigator.share({ files: [file], title, text: link });
    } catch {
      // Closed by the reader: not an error, nothing to say.
    }
  }

  async function copy() {
    interact('tale_card_out', 'link', null, false);
    try {
      await navigator.clipboard.writeText(link);
      copied = true;
    } catch {
      copied = false;
    }
  }
</script>

{#if pick}
  <!-- Нажатие не должно снимать выделение, из которого берётся фраза: на
       mousedown браузер его сбрасывает, поэтому событие гасится. -->
  <div
    class="pick"
    class:pick--below={pick.below}
    style="left: {pick.x}px; top: {pick.y}px"
    use:portal
    onmousedown={(e) => e.preventDefault()}
    role="presentation"
  >
    {#if pick.tooLong}
      <span class="pick-note">{$t('talesCardTooLong')}</span>
    {:else}
      <button class="pick-btn" type="button" onclick={make} disabled={busy}>
        <span class="pick-star" aria-hidden="true">✦</span>{$t('talesCardMake')}
      </button>
    {/if}
  </div>
{/if}

{#if failed}
  <p class="fail" role="status" use:portal>{$t('talesCardFailed')}</p>
{/if}

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog bind:this={box} class="box" onclose={shut} onclick={away} aria-label={$t('talesCardTitle')}>
  {#if made}
    <img class="card" src={made.src} alt={$t('talesCardAlt')} width="1080" height="1350" />
    {#if photo}
      <div class="looks" role="group" aria-label={$t('talesCardLook')}>
        <button type="button" class="look" aria-pressed={style === 'glass'} disabled={busy} onclick={() => restyle('glass')}>{$t('talesCardLookGlass')}</button>
        <button type="button" class="look" aria-pressed={style === 'paper'} disabled={busy} onclick={() => restyle('paper')}>{$t('talesCardLookPaper')}</button>
      </div>
    {/if}
    <div class="acts">
      {#if canShare}
        <button class="act act--main" type="button" onclick={share}>{$t('talesCardShare')}</button>
      {/if}
      <a
        class="act"
        class:act--main={!canShare}
        href={made.src}
        download={made.name}
        onclick={() => interact('tale_card_out', 'save', null, false)}
      >{$t('talesCardSave')}</a>
      <button class="act" type="button" onclick={copy}>{copied ? $t('talesCardCopied') : $t('talesCardCopy')}</button>
      <button class="act act--quiet" type="button" onclick={() => box?.close()}>{$t('talesCardClose')}</button>
    </div>
    <p class="live" aria-live="polite">{copied ? $t('talesCardCopied') : ''}</p>
  {/if}
</dialog>

<style>
  .pick {
    position: fixed;
    z-index: 60;
    transform: translate(-50%, -100%);
  }
  .pick--below { transform: translate(-50%, 0); }

  .pick-btn,
  .pick-note {
    display: inline-flex;
    align-items: center;
    gap: 0.5em;
    padding: 0.5em 0.95em;
    font: 500 13px/1 'Inter', system-ui, sans-serif;
    letter-spacing: 0.04em;
    color: var(--ink, #34251c);
    background: var(--paper, #f8f1e7);
    border: 1px solid var(--edge, #d8c6b1);
    border-radius: 2px;
    box-shadow: 0 0 0 3px var(--paper, #f8f1e7), 0 0 0 4px var(--edge, #d8c6b1), 0 8px 22px rgba(52, 37, 28, 0.18);
  }
  .pick-btn { cursor: pointer; }
  .pick-btn:hover:not(:disabled) { color: var(--deep, #6f3b24); }
  .pick-btn:focus-visible { outline: 2px solid var(--copper, #c65f3c); outline-offset: 4px; }
  .pick-btn:disabled { opacity: 0.6; cursor: progress; }
  .pick-note { color: var(--ink-tertiary, #6f5a47); }
  .pick-star { color: var(--copper, #c65f3c); font-size: 12px; }

  .fail {
    position: fixed;
    z-index: 60;
    left: 50%;
    bottom: 24px;
    transform: translateX(-50%);
    margin: 0;
    padding: 0.6em 1em;
    font: 500 13px/1.3 'Inter', system-ui, sans-serif;
    color: var(--ink, #34251c);
    background: var(--paper, #f8f1e7);
    border: 1px solid var(--edge, #d8c6b1);
  }

  .box {
    margin: auto;
    padding: 18px;
    width: min(92vw, 420px);
    max-height: 94vh;
    overflow: auto;
    color: var(--ink, #34251c);
    background: var(--paper, #f8f1e7);
    border: 1px solid var(--edge, #d8c6b1);
    border-radius: 2px;
    /* The house's window: turned a degree, a second border outside the first. */
    transform: rotate(1deg);
    box-shadow: 0 0 0 5px var(--paper, #f8f1e7), 0 0 0 6px var(--edge, #d8c6b1), 0 24px 60px rgba(52, 37, 28, 0.3);
  }
  .box::backdrop { background: rgba(52, 37, 28, 0.55); }

  .card {
    display: block;
    width: auto;
    max-width: 100%;
    /* 4:5 is tall: on a low window the picture shrinks rather than the whole
       dialog scrolling away from its buttons. */
    max-height: calc(100vh - 230px);
    margin: 0 auto;
    height: auto;
    box-shadow: 0 2px 14px rgba(52, 37, 28, 0.2);
  }

  .looks {
    display: flex;
    gap: 6px;
    margin-top: 14px;
  }
  .look {
    flex: 1;
    padding: 0.55em 0.8em;
    font: 500 12px/1.1 'Inter', system-ui, sans-serif;
    letter-spacing: 0.04em;
    color: var(--ink-tertiary, #6f5a47);
    background: transparent;
    border: 1px solid var(--edge, #d8c6b1);
    border-radius: 2px;
    cursor: pointer;
  }
  .look[aria-pressed='true'] { color: var(--ink, #34251c); border-color: var(--copper, #c65f3c); }
  .look:disabled { cursor: progress; }
  .look:focus-visible { outline: 2px solid var(--copper, #c65f3c); outline-offset: 2px; }

  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 16px;
  }
  .act {
    flex: 1 1 auto;
    padding: 0.7em 1em;
    text-align: center;
    text-decoration: none;
    font: 500 13px/1.1 'Inter', system-ui, sans-serif;
    letter-spacing: 0.03em;
    color: var(--ink, #34251c);
    background: transparent;
    border: 1px solid var(--edge, #d8c6b1);
    border-radius: 2px;
    cursor: pointer;
  }
  .act:hover { border-color: var(--copper, #c65f3c); }
  .act:focus-visible { outline: 2px solid var(--copper, #c65f3c); outline-offset: 2px; }
  .act--main {
    color: #fff;
    background: var(--deep, #6f3b24);
    border-color: var(--deep, #6f3b24);
  }
  .act--quiet { flex-basis: 100%; border-color: transparent; color: var(--ink-tertiary, #6f5a47); }

  .live {
    margin: 0;
    height: 0;
    overflow: hidden;
  }
</style>
