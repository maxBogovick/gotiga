<script lang="ts">
  // Открытка с картой: картинка карты на бумаге, имя под ней и адрес сайта.
  // Её сохраняют или отправляют, и по ссылке на ней приходят новые люди —
  // то же устройство, что у открытки из фразы байки (`TaleQuoteCard`).
  //
  // Рисуется на canvas, а не снимком `BattleCard`: снимок DOM без сторонней
  // библиотеки не сделать, а карта на открытке крупнее любой на сайте.
  // Готовая карта (`isBaked`) — это и есть вся карта одной картинкой, поэтому
  // она кладётся целиком; у прочих кладётся фотография работы в рамке.
  import { t, lang } from '$lib/i18n';
  import { interact } from '$lib/analytics';
  import { cardCopy, frameForCard, frameName, isBaked } from '$lib/battles';
  import type { BattleCard, BattleFrame } from '$lib/types/api';

  let { card, frames }: { card: BattleCard; frames: BattleFrame[] } = $props();

  const W = 1080;
  const H = 1350;
  const SERIF = "'Cormorant Garamond', Georgia, serif";
  const SANS = 'Inter, system-ui, sans-serif';

  let box = $state<HTMLDialogElement | null>(null);
  let busy = $state(false);
  let failed = $state(false);
  let copied = $state(false);
  let made = $state<{ blob: Blob; src: string } | null>(null);

  let frame = $derived(frameForCard(card, frames));
  let baked = $derived(isBaked(frame));
  let title = $derived(cardCopy(card, $lang).title || card.figurineName || '');
  let rank = $derived(frameName(frame, $lang));
  let link = $derived(
    typeof location === 'undefined'
      ? ''
      : `${location.origin}/battles?card=${card.id}&src=card_postcard`,
  );

  // Чужой адрес сделал бы canvas «грязным», и `toBlob` отказал бы: путь
  // `/static/…` просится у своего origin (в dev его проксирует сервер).
  function local(u: string): string {
    try {
      const x = new URL(u, location.href);
      if (x.origin === location.origin) return x.href;
      return x.pathname.startsWith('/static/') ? x.pathname : x.href;
    } catch {
      return u;
    }
  }

  async function picture(): Promise<ImageBitmap | null> {
    if (!card.artUrl) return null;
    try {
      const r = await fetch(local(card.artUrl), { credentials: 'omit' });
      if (!r.ok) return null;
      return await createImageBitmap(await r.blob());
    } catch {
      return null;
    }
  }

  /** Положить картинку в коробку «cover»: заполнить, лишнее обрезать поровну. */
  function cover(ctx: CanvasRenderingContext2D, img: ImageBitmap, x: number, y: number, w: number, h: number) {
    const k = Math.max(w / img.width, h / img.height);
    const sw = w / k;
    const sh = h / k;
    ctx.drawImage(img, (img.width - sw) / 2, (img.height - sh) / 2, sw, sh, x, y, w, h);
  }

  function paint(img: ImageBitmap | null): HTMLCanvasElement {
    const canvas = document.createElement('canvas');
    canvas.width = W;
    canvas.height = H;
    const ctx = canvas.getContext('2d')!;

    // Бумага.
    ctx.fillStyle = '#f8f1e7';
    ctx.fillRect(0, 0, W, H);
    ctx.strokeStyle = '#d8c6b1';
    ctx.lineWidth = 2;
    ctx.strokeRect(36, 36, W - 72, H - 72);
    ctx.strokeRect(46, 46, W - 92, H - 92);

    // Карта 5 : 7.
    const cw = 600;
    const ch = 840;
    const cx = (W - cw) / 2;
    const cy = 110;
    ctx.save();
    ctx.shadowColor = 'rgba(52, 37, 28, 0.35)';
    ctx.shadowBlur = 40;
    ctx.shadowOffsetY = 14;
    ctx.fillStyle = '#efe4d3';
    ctx.fillRect(cx, cy, cw, ch);
    ctx.restore();
    if (img && baked) {
      cover(ctx, img, cx, cy, cw, ch);
    } else {
      // Рамка из двух линий и фотография работы в окне.
      ctx.fillStyle = '#6f3b24';
      ctx.fillRect(cx, cy, cw, ch);
      ctx.fillStyle = '#efe4d3';
      ctx.fillRect(cx + 18, cy + 18, cw - 36, ch - 36);
      if (img) cover(ctx, img, cx + 34, cy + 34, cw - 68, ch - 68);
      ctx.strokeStyle = '#c65f3c';
      ctx.lineWidth = 3;
      ctx.strokeRect(cx + 34, cy + 34, cw - 68, ch - 68);
    }

    // Подписи.
    ctx.textAlign = 'center';
    ctx.fillStyle = '#34251c';
    let size = 64;
    ctx.font = `italic 500 ${size}px ${SERIF}`;
    while (ctx.measureText(title).width > W - 180 && size > 36) {
      size -= 2;
      ctx.font = `italic 500 ${size}px ${SERIF}`;
    }
    ctx.fillText(title, W / 2, cy + ch + 110);

    ctx.fillStyle = '#6f3b24';
    ctx.font = `500 26px ${SANS}`;
    const caption = [$t('worldPostcardCaption'), rank].filter(Boolean).join(' · ').toUpperCase();
    ctx.fillText(caption, W / 2, cy + ch + 165);
    ctx.fillStyle = '#8a6a55';
    ctx.font = `400 26px ${SANS}`;
    ctx.fillText(location.host, W / 2, H - 80);
    return canvas;
  }

  async function make() {
    if (busy) return;
    busy = true;
    failed = false;
    copied = false;
    try {
      await document.fonts.load(`italic 500 64px ${SERIF}`, title);
      const canvas = paint(await picture());
      const blob = await new Promise<Blob>((resolve, reject) =>
        canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('no blob'))), 'image/jpeg', 0.92),
      );
      if (made) URL.revokeObjectURL(made.src);
      made = { blob, src: URL.createObjectURL(blob) };
      interact('card_postcard_made', card.slug);
      box?.showModal();
    } catch {
      failed = true;
    } finally {
      busy = false;
    }
  }

  let file = $derived(made ? new File([made.blob], `${card.slug}-card.jpg`, { type: 'image/jpeg' }) : null);
  let canShare = $derived(
    !!file && typeof navigator !== 'undefined' && !!navigator.canShare && navigator.canShare({ files: [file] }),
  );

  async function share() {
    if (!file) return;
    interact('card_postcard_out', 'share', null, false);
    try {
      await navigator.share({ files: [file], title, text: link });
    } catch {
      // Окно закрыли — это не ошибка.
    }
  }

  async function copy() {
    interact('card_postcard_out', 'link', null, false);
    try {
      await navigator.clipboard.writeText(link);
      copied = true;
    } catch {
      copied = false;
    }
  }

  function away(e: MouseEvent) {
    if (e.target !== box || !box) return;
    const r = box.getBoundingClientRect();
    const inside = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
    if (!inside) box.close();
  }
</script>

<button type="button" class="pc-open" onclick={make} disabled={busy}>
  {busy ? $t('worldPostcardBusy') : $t('worldPostcard')}
</button>
{#if failed}<span class="pc-failed">{$t('worldPostcardFailed')}</span>{/if}

<dialog bind:this={box} class="pc" onclick={away} aria-label={$t('worldPostcard')}>
  {#if made}
    <img class="pc-img" src={made.src} alt={title} />
    <div class="pc-actions">
      {#if canShare}
        <button type="button" onclick={share}>{$t('worldPostcardShare')}</button>
      {/if}
      <a
        href={made.src}
        download={`${card.slug}-card.jpg`}
        onclick={() => interact('card_postcard_out', 'save', null, false)}
      >{$t('worldPostcardSave')}</a>
      <button type="button" onclick={copy}>{copied ? $t('worldPostcardCopied') : $t('worldPostcardLink')}</button>
      <button type="button" class="pc-close" onclick={() => box?.close()}>{$t('worldPostcardClose')}</button>
    </div>
  {/if}
</dialog>

<style>
  .pc-open {
    font: inherit;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 17px;
    background: none;
    border: 0;
    padding: 0;
    cursor: pointer;
    color: var(--color-ember-deep, #6f3b24);
    border-bottom: 1px dashed color-mix(in srgb, var(--color-ember, #c65f3c) 50%, transparent);
  }
  .pc-open:hover {
    color: var(--color-ink-primary, #34251c);
  }
  .pc-open:disabled {
    opacity: 0.6;
    cursor: progress;
  }
  .pc-failed {
    margin-left: 0.6rem;
    font-size: 12px;
    color: var(--color-ink-tertiary, #8a6a55);
  }

  .pc {
    /* Общие стили сайта обнуляют поля у <dialog>; без этого окно стоит в углу. */
    position: fixed;
    inset: 0;
    margin: auto;
    height: fit-content;
    max-height: 92vh;
    overflow: auto;
    max-width: min(92vw, 460px);
    padding: 18px;
    border: 1px solid #d8c6b1;
    outline: 1px solid #d8c6b1;
    outline-offset: 4px;
    background: #f8f1e7;
    transform: rotate(1deg);
  }
  .pc::backdrop {
    background: rgba(52, 37, 28, 0.45);
  }
  .pc-img {
    display: block;
    width: 100%;
    height: auto;
  }
  .pc-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1.1rem;
    margin-top: 14px;
    justify-content: center;
  }
  .pc-actions button,
  .pc-actions a {
    font: inherit;
    font-size: 14px;
    background: none;
    border: 0;
    padding: 0;
    cursor: pointer;
    color: var(--color-ember-deep, #6f3b24);
    text-decoration: none;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ember, #c65f3c) 40%, transparent);
  }
  .pc-actions .pc-close {
    color: var(--color-ink-tertiary, #8a6a55);
    border-bottom-color: transparent;
  }
</style>
