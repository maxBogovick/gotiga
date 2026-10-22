<script lang="ts">
  // Док работы: плашка внизу по центру, которая рассказывает о работе, стоящей
  // сейчас на экране (`currentWork`). Только компьютер (шире 1024 px); на
  // телефоне внизу панель разделов, а работу показывает сама страница.
  //
  // В доке нет того, что уже есть на каждой карточке работы (сохранить,
  // поделиться, открыть). Он даёт то, чего на карточке нет, тремя кнопками:
  //   «Фото · N»  — все фотографии работы разом, нажатие открывает крупно;
  //   «Связи · N» — байка о работе (с отрывком), её карта в игре, остальные
  //                 работы той же серии;
  //   «Спросить»  — письмо автору, в котором уже стоит название работы.
  // Слева название не пишется: работа названа на самой карточке, а в доке она
  // узнаётся в лицо. Миниатюра и состояние — одна кнопка, она раскрывает паспорт
  // (материал, техника, размеры, серия, короткий текст, ссылка на следующую
  // работу в наличии). Рядом три похожие работы, выбранные автором: нажатие
  // всегда открывает страницу работы, название видно по наведению.
  //
  // Ступеней две. Пока листают быстро (`fast`), плашка показывает только место в
  // ленте: кнопки в летящем окне не нужны, а название не успеть прочесть. Как
  // только прокрутка замедлилась, всё стоит на месте сразу, без наведения.
  // Ширина плашки одна на обе ступени, чтобы она не прыгала под рукой.
  //
  // Плашка ничего не торопит: ни цены, ни «осталось N», ни предложений. Состояние
  // работы названо словом, цвет только дублирует его.
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { t, lang, localized, LANG_META, quote } from '$lib/i18n';
  import { imageAlt, localizeListItem, workName } from '$lib/workText';
  import { get } from 'svelte/store';
  import { api, resolveMediaUrl } from '$lib/api';
  import { figurineHref } from '$lib/figurineHref';
  import { interact } from '$lib/analytics';
  import AppImage from '$lib/components/AppImage.svelte';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import ContactMessageForm from '$lib/components/ContactMessageForm.svelte';
  import Lightbox from '$lib/components/Lightbox.svelte';
  import { cardCopy } from '$lib/battles';
  import { currentWork } from '$lib/stores/current-work.svelte';
  import {
    loadWorld,
    cardOfWork,
    talesOfWork,
    etudesWithCard,
    cardHref,
    etudeHref,
    type World,
  } from '$lib/world';
  import { leafHref } from '$lib/gazette';
  import { readingMinutes, minutesKey, renderTale } from '$lib/tales';
  import type { Figurine } from '$lib/types/api';

  type Level = 'fast' | 'look';
  type Panel = 'passport' | 'photos' | 'tale' | 'ask' | null;
  type Photo = { url: string; thumbUrl?: string; alt: string };

  // Быстрее — «летит» (пикселей в миллисекунду, сглажено); выход из быстрой
  // ступени ниже входа, иначе плашка мигала бы на пороге.
  const FAST_ENTER = 1.1;
  const FAST_LEAVE = 0.45;
  const SETTLE_MS = 200;
  // Прокрутка, начатая самой кнопкой, быстрой не считается, пока не остановится:
  // иначе кнопки исчезали бы из-под пальца в тот миг, когда ими пользуются.
  const STEP_GRACE_MS = 1100;
  // Линия чтения: та же доля окна, по которой хранилище выбирает работу.
  const LINE = 0.42;
  // Фотографии работы просятся, когда на неё смотрят спокойно, а не при каждом
  // проезде мимо: запрос на каждую из шестнадцати работ в быстрой прокрутке — лишний.
  const PHOTOS_AFTER_MS = 350;
  const EXCERPT_CHARS = 230;

  let root = $state<HTMLElement | null>(null);
  let whoBtn = $state<HTMLButtonElement | null>(null);
  let level = $state<Level>('look');
  let panel = $state<Panel>(null);
  let progress = $state(0);
  let reduceMotion = $state(false);
  let world = $state<World | null>(null);
  // Фотографии работ по их номеру; пустой список — запросили, а лишнего нет.
  let photos = $state.raw<Record<string, Photo[]>>({});
  let viewer = $state<{ images: Photo[]; index: number } | null>(null);
  // Работа, о которой пишут: снимок на миг открытия, иначе прокрутка под рукой
  // меняла бы тему письма, которое уже начали.
  let askAbout = $state<{ id: string; name: string } | null>(null);

  // Карта в полный размер по наведению: ждём долю секунды, чтобы она не вспыхивала
  // под указателем, который просто идёт мимо кнопки.
  let cardPeek = $state(false);
  let peekTimer: ReturnType<typeof setTimeout> | undefined;
  const PEEK_DELAY_MS = 250;
  function peekCard() {
    clearTimeout(peekTimer);
    peekTimer = setTimeout(() => {
      cardPeek = true;
      interact('work_dock', 'card_peek');
    }, PEEK_DELAY_MS);
  }
  function unpeekCard() {
    clearTimeout(peekTimer);
    cardPeek = false;
  }

  let work = $derived(currentWork.work ? localizeListItem(currentWork.work, $lang) : null);
  let index = $derived(currentWork.index);
  let total = $derived(currentWork.total);
  let position = $derived(
    $t('dockPosition').replace('{i}', String(index + 1)).replace('{n}', String(total)),
  );

  // ── Фото ─────────────────────────────────────────────────────────────────
  let workPhotos = $derived(work ? (photos[work.id] ?? null) : null);
  // Одна фотография — это та, что уже на карточке: кнопка нужна, когда есть из чего выбрать.
  let photoCount = $derived(workPhotos && workPhotos.length > 1 ? workPhotos.length : 0);

  const IMAGE_ORDER: Record<string, number> = { face: 0, detail: 1, full: 2 };
  function photosOf(f: Figurine): Photo[] {
    return [...f.images]
      .sort((a, b) => (IMAGE_ORDER[a.imageType] ?? 3) - (IMAGE_ORDER[b.imageType] ?? 3))
      .map((img) => ({
        url: resolveMediaUrl(img.originalUrl ?? img.url) ?? '',
        thumbUrl: resolveMediaUrl(img.thumbUrl ?? img.url) ?? undefined,
        alt: imageAlt(img, get(lang)) || workName(f, get(lang)),
      }))
      .filter((p) => p.url);
  }

  $effect(() => {
    const id = work?.id;
    if (!id || level === 'fast' || photos[id] !== undefined) return;
    const timer = setTimeout(() => {
      api
        .getFigurine(id)
        .then((f) => (photos = { ...photos, [id]: f ? photosOf(f) : [] }))
        .catch(() => (photos = { ...photos, [id]: [] }));
    }, PHOTOS_AFTER_MS);
    return () => clearTimeout(timer);
  });

  // ── Связи ────────────────────────────────────────────────────────────────
  let card = $derived(world && work ? cardOfWork(world, work.id) : null);
  let etudes = $derived(world && card ? etudesWithCard(world, card) : []);
  let tale = $derived(world && work ? (talesOfWork(world, work.id)[0] ?? null) : null);
  let taleBody = $derived(tale ? localized(tale, 'body', $lang) : '');
  let taleMinutes = $derived(readingMinutes(taleBody));
  let taleTitle = $derived(tale ? localized(tale, 'title', $lang) : '');
  let taleExcerpt = $derived.by(() => {
    if (!tale) return '';
    const first = renderTale(taleBody).find((b) => b.kind === 'p');
    const text = first && 'text' in first ? first.text.replace(/\*\*/g, '') : '';
    if (text.length <= EXCERPT_CHARS) return text;
    return `${text.slice(0, EXCERPT_CHARS).replace(/\s+\S*$/, '')}…`;
  });
  // Три похожие работы, выбранные автором (админка, «Реестр», форма работы).
  // Ищутся по всему собранию, а не по ленте. Выбранная, но скрытая или уже
  // удалённая работа в собрании не найдётся и просто не показывается.
  let poolById = $derived(new Map(currentWork.pool.map((w) => [w.id, w])));
  let similar = $derived(
    (work?.similarIds ?? [])
      .map((id) => poolById.get(id))
      .filter((w): w is NonNullable<typeof w> => Boolean(w))
      .map((w) => localizeListItem(w, $lang))
      .slice(0, 3),
  );

  // Полный паспорт: подписанные поля, только заполненные.
  let passport = $derived(
    work
      ? [
          { label: $t('archiveMaterialLabel'), value: work.material },
          { label: $t('archiveTechniqueLabel'), value: work.technique },
          { label: $t('figurineDimensions'), value: work.dimensions },
          { label: $t('archiveSeriesLabel'), value: work.series },
        ].filter((f): f is { label: string; value: string } => Boolean(f.value?.trim()))
      : [],
  );

  // Ближайшая впереди работа, которую можно взять. Впереди, а не по кругу: лента
  // идёт сверху вниз, и «вернуться к началу» человек не просил.
  let nextAvailable = $derived.by(() => {
    for (let i = index + 1; i < total; i++) {
      if (currentWork.works[i].status === 'available') return i;
    }
    return -1;
  });

  // ── Поведение: скорость прокрутки ────────────────────────────────────────
  let speed = 0;
  let lastY = 0;
  let lastT = 0;
  let graceUntil = 0;
  let settleTimer: ReturnType<typeof setTimeout> | undefined;
  let raf = 0;

  function measure() {
    raf = 0;
    const reel = currentWork.reel;
    if (!reel) return;
    const rect = reel.getBoundingClientRect();
    const line = window.innerHeight * LINE;
    progress = Math.min(1, Math.max(0, (line - rect.top) / Math.max(1, rect.height)));
  }

  function onScroll() {
    const now = performance.now();
    const y = window.scrollY;
    const dt = Math.max(1, now - lastT);
    speed = speed * 0.6 + (Math.abs(y - lastY) / dt) * 0.4;
    lastY = y;
    lastT = now;

    if (now < graceUntil) {
      // Идёт прокрутка, начатая кнопкой: пока она не остановится, быстрой не считается.
      level = 'look';
      graceUntil = Math.max(graceUntil, now + 350);
    } else if (speed > FAST_ENTER) level = 'fast';
    else if (level !== 'fast' || speed < FAST_LEAVE) level = 'look';
    if (!raf) raf = requestAnimationFrame(measure);

    clearTimeout(settleTimer);
    settleTimer = setTimeout(() => {
      speed = 0;
      level = 'look';
    }, SETTLE_MS);
  }

  // Быстрая прокрутка закрывает панели: они описывают работу, которая уже уходит.
  // Исключение — письмо: его начали писать, и терять набранное из-за прокрутки нельзя.
  $effect(() => {
    if ((level === 'fast' && panel !== 'ask') || !work) panel = null;
    if (level === 'fast' || !work || !card) cardPeek = false;
  });

  // Новая работа под линией — линейка пересчитывается сразу, а не при следующей прокрутке.
  $effect(() => {
    void currentWork.index;
    if (!raf) raf = requestAnimationFrame(measure);
  });

  // ── Ход по ленте и панели ────────────────────────────────────────────────
  function goTo(i: number, how: string) {
    const target = currentWork.works[i];
    if (!target) return;
    const el = document.getElementById(`work-${target.id}`);
    if (!el) return;
    graceUntil = performance.now() + STEP_GRACE_MS;
    level = 'look';
    panel = null;
    el.scrollIntoView({ behavior: reduceMotion ? 'auto' : 'smooth', block: 'center' });
    interact('work_dock', how);
  }

  function toggle(which: Exclude<Panel, null>) {
    if (panel === which) {
      panel = null;
      return;
    }
    if (which === 'ask' && work) askAbout = { id: work.id, name: work.name };
    panel = which;
    interact('work_dock', which);
  }

  function openPhoto(i: number) {
    if (!workPhotos) return;
    viewer = { images: workPhotos, index: i };
    panel = null;
    interact('work_dock', 'photo_open');
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && panel) {
      const was = panel;
      panel = null;
      if (was === 'passport') whoBtn?.focus();
    }
  }

  function onPointerDown(e: PointerEvent) {
    if (panel && root && !root.contains(e.target as Node)) panel = null;
  }

  let askPrefill = $derived.by(() => {
    if (!askAbout) return '';
    const [open, close] = LANG_META[$lang].quotes;
    return `${open}${askAbout.name}${close}: `;
  });

  onMount(() => {
    reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    lastY = window.scrollY;
    lastT = performance.now();
    loadWorld()
      .then((w) => (world = w))
      .catch(() => {});
    window.addEventListener('scroll', onScroll, { passive: true });
    document.addEventListener('keydown', onKey);
    document.addEventListener('pointerdown', onPointerDown);
    return () => {
      window.removeEventListener('scroll', onScroll);
      document.removeEventListener('keydown', onKey);
      document.removeEventListener('pointerdown', onPointerDown);
      clearTimeout(settleTimer);
      clearTimeout(peekTimer);
      if (raf) cancelAnimationFrame(raf);
    };
  });
</script>

<section
  bind:this={root}
  class="dock"
  class:is-up={Boolean(work)}
  class:is-fast={level === 'fast'}
  aria-label={$t('dockWorkLabel')}
  inert={!work}
>
  {#if panel === 'passport' && work}
    <div
      id="dock-passport"
      class="panel"
      transition:fly={{ y: 12, duration: reduceMotion ? 0 : 180 }}
    >
      {#if passport.length > 0}
        <dl class="grid">
          {#each passport as f (f.label)}
            <div>
              <dt>{f.label}</dt>
              <dd>{f.value}</dd>
            </div>
          {/each}
        </dl>
      {/if}
      {#if work.shortText}
        <p class="blurb">{work.shortText}</p>
      {/if}
      <p class="foot">
        <span>
          {position}{#if currentWork.remaining > 0} · {currentWork.remaining} {$t('homeMoreInArchive')}{/if}
        </span>
        {#if nextAvailable >= 0}
          <button type="button" class="textlink" onclick={() => goTo(nextAvailable, 'avail')}>
            {$t('dockNextAvailable')} →
          </button>
        {/if}
      </p>
    </div>
  {/if}

  {#if panel === 'photos' && workPhotos}
    <div
      id="dock-photos"
      class="panel"
      transition:fly={{ y: 12, duration: reduceMotion ? 0 : 180 }}
    >
      <ul class="photos">
        {#each workPhotos as p, i (p.url)}
          <li>
            <button type="button" class="photo-btn" aria-label={`${p.alt} ${i + 1}`} onclick={() => openPhoto(i)}>
              <img src={p.thumbUrl ?? p.url} alt="" loading="lazy" decoding="async" />
            </button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if panel === 'tale' && tale}
    <div
      id="dock-tale"
      class="panel tale"
      transition:fly={{ y: 12, duration: reduceMotion ? 0 : 180 }}
    >
      <h3>
        {$t('dockTale')}{#if taleMinutes} · {taleMinutes} {$t(minutesKey(taleMinutes))}{/if}
      </h3>
      <p class="sec-title">{taleTitle}</p>
      {#if taleExcerpt}<p class="excerpt">{taleExcerpt}</p>{/if}
      <a
        class="textlink"
        href={leafHref(tale, 'work_dock')}
        onclick={() => interact('world_link', 'dock_to_tale')}>{$t('worldTalesGo')} →</a
      >
    </div>
  {/if}

  {#if panel === 'ask' && askAbout}
    <div
      id="dock-ask"
      class="panel ask"
      transition:fly={{ y: 12, duration: reduceMotion ? 0 : 180 }}
    >
      <h3>{$t('dockAsk')}</h3>
      <p class="note">{$t('headerContactNote')}</p>
      <ContactMessageForm source="dock" compact prefill={askPrefill} />
    </div>
  {/if}

  {#if cardPeek && world && card}
    <div class="peek" aria-hidden="true" transition:fade={{ duration: reduceMotion ? 0 : 140 }}>
      <div class="peek-card">
        <BattleCard {card} frames={world.frames} owned={true} interactive={false} uniform={true} />
      </div>
      <p class="peek-cap">
        {cardCopy(card, $lang).title} · {$t('dockRank').replace('{n}', String(card.tier))}
        {#if etudes.length > 0}
          <span>{$t('worldInPlayEtudes')} {quote($lang, localized(etudes[0], 'title', $lang))}</span>
        {/if}
      </p>
    </div>
  {/if}

  <div class="pill">
    <span class="line" style="width:{(progress * 100).toFixed(1)}%" aria-hidden="true"></span>

    <div class="row">
      <div class="left">
        <button
          bind:this={whoBtn}
          type="button"
          class="who"
          aria-expanded={panel === 'passport'}
          aria-controls="dock-passport"
          aria-label={work?.name}
          onclick={() => toggle('passport')}
        >
          {#if work}
            {#key work.id}
              <span class="thumb" in:fade={{ duration: reduceMotion ? 0 : 160 }}>
                <AppImage src={work.faceImageUrl} thumbUrl={work.thumbUrl} alt="" class="thumb-img" sizes="48px" />
              </span>
            {/key}
          {/if}
        </button>

        {#if similar.length > 0}
          <div class="similar" role="group" aria-label={$t('dockSimilar')}>
            <span class="similar-label">{$t('dockSimilar')}</span>
            {#key work?.id}
              {#each similar as w (w.id)}
                <a
                  class="sim"
                  href={figurineHref(w, 'work_dock_similar')}
                  title={w.name}
                  aria-label={w.name}
                  onclick={() => interact('work_dock', 'similar')}
                  in:fade={{ duration: reduceMotion ? 0 : 160 }}
                >
                  <AppImage src={w.faceImageUrl} thumbUrl={w.thumbUrl} alt="" class="sim-img" sizes="42px" />
                </a>
              {/each}
            {/key}
          </div>
        {/if}
      </div>

      <div class="rail" aria-hidden="true">
        <span class="ticks">
          {#each currentWork.works as w, i (w.id)}
            <i class:is-past={i < index} class:is-now={i === index}></i>
          {/each}
        </span>
        <span class="count">{position}</span>
      </div>

      <div class="extra">
        {#if photoCount > 0}
          <button
            type="button"
            class="act"
            class:is-active={panel === 'photos'}
            aria-expanded={panel === 'photos'}
            aria-controls="dock-photos"
            onclick={() => toggle('photos')}
          >
            <svg width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true">
              <rect x="1.8" y="2.8" width="12.4" height="10.4" rx="1.3" stroke="currentColor" stroke-width="1.2" />
              <path d="M3.5 11.5l3-3.6 2.3 2.6 1.6-1.8 2.1 2.8" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" />
              <circle cx="5.4" cy="6" r="1" fill="currentColor" />
            </svg>
            {$t('dockPhotos')} · {photoCount}
          </button>
        {/if}
        {#if tale}
          <button
            type="button"
            class="act act-story"
            class:is-active={panel === 'tale'}
            aria-expanded={panel === 'tale'}
            aria-controls="dock-tale"
            onclick={() => toggle('tale')}
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
              <path d="M8 4C6.6 3 4.6 2.7 2.5 3v8.6c2.1-.3 4.1 0 5.5 1 1.4-1 3.4-1.3 5.5-1V3C11.4 2.7 9.4 3 8 4z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" />
              <path d="M8 4v8.6" stroke="currentColor" stroke-width="1.2" />
            </svg>
            {$t('dockTale')}
          </button>
        {/if}
        {#if card}
          <span class="peek-anchor">
            <a
              class="act act-story"
              href={cardHref(card)}
              aria-label={`${$t('dockCard')}: ${cardCopy(card, $lang).title}`}
              onpointerenter={peekCard}
              onpointerleave={unpeekCard}
              onfocus={peekCard}
              onblur={unpeekCard}
              onclick={() => interact('world_link', 'dock_to_card')}
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                <rect x="2.6" y="3.2" width="6.6" height="9.4" rx="1.1" transform="rotate(-9 5.9 7.9)" stroke="currentColor" stroke-width="1.2" />
                <rect x="6.8" y="2.8" width="6.6" height="9.4" rx="1.1" transform="rotate(8 10.1 7.5)" stroke="currentColor" stroke-width="1.2" />
              </svg>
              {$t('dockCard')}
            </a>
          </span>
        {/if}
        <button
          type="button"
          class="act"
          class:is-active={panel === 'ask'}
          aria-expanded={panel === 'ask'}
          aria-controls="dock-ask"
          onclick={() => toggle('ask')}
        >
          <svg width="15" height="15" viewBox="0 0 14 14" fill="none" aria-hidden="true">
            <path d="M12.2 1.8c-3.6 0-8.4 3.4-9.9 7-.3.7.4 1.4 1.1 1.1l1.9-.8" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round" />
            <path d="M12.2 1.8c0 3.6-3.4 8.4-7 9.9-.7.3-1.4-.4-1.1-1.1l.8-1.9" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round" />
            <path d="M5.4 8.6L1.8 12.2" stroke="currentColor" stroke-width="1" stroke-linecap="round" />
          </svg>
          {$t('dockAsk')}
        </button>
      </div>
    </div>
  </div>
</section>

{#if viewer}
  <Lightbox images={viewer.images} startIndex={viewer.index} onClose={() => (viewer = null)} />
{/if}

<style>
  .dock {
    --ink: var(--color-ink-primary, #34251c);
    --ember: var(--color-ember, #c65f3c);
    --ember-deep: var(--color-ember-deep, #6f3b24);
    --paper: var(--color-canvas-base, #f8f1e7);
    --line: var(--border, #d8c6b1);
    --glass-edge: color-mix(in srgb, #fff 70%, transparent);
    --lift: 0 1px 1px color-mix(in srgb, var(--ink) 8%, transparent),
      0 6px 14px -6px color-mix(in srgb, var(--ink) 22%, transparent);

    position: fixed;
    left: 50%;
    bottom: 18px;
    z-index: 190;
    width: min(880px, calc(100vw - 48px));
    transform: translate(-50%, 160%);
    transition: transform 0.45s cubic-bezier(0.2, 0.9, 0.25, 1.06);
    font-family: var(--font-body, system-ui, sans-serif);
    color: var(--ink);
  }

  .dock.is-up {
    transform: translate(-50%, 0);
  }

  /* Только компьютер: на телефоне и планшете внизу панель разделов. */
  @media (max-width: 1024px) {
    .dock {
      display: none;
    }
  }

  /* ── Плашка: матовое стекло на пергаменте ── */
  .pill {
    position: relative;
    overflow: hidden;
    border: 1px solid color-mix(in srgb, var(--line) 85%, transparent);
    border-radius: 22px;
    background:
      radial-gradient(120% 140% at 0% 0%, color-mix(in srgb, #fff 55%, transparent), transparent 55%),
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--paper) 90%, #fff),
        color-mix(in srgb, var(--paper) 86%, var(--line))
      );
    box-shadow:
      inset 0 1px 0 var(--glass-edge),
      inset 0 -1px 0 color-mix(in srgb, var(--ember-deep) 8%, transparent),
      0 2px 4px color-mix(in srgb, var(--ink) 6%, transparent),
      0 22px 44px -14px color-mix(in srgb, var(--ink) 34%, transparent);
  }

  @supports (backdrop-filter: blur(1px)) {
    .pill {
      background:
        radial-gradient(120% 140% at 0% 0%, color-mix(in srgb, #fff 50%, transparent), transparent 55%),
        linear-gradient(
          180deg,
          color-mix(in srgb, var(--paper) 78%, transparent),
          color-mix(in srgb, var(--paper) 68%, var(--line))
        );
      backdrop-filter: blur(18px) saturate(1.35);
    }
  }

  /* Место в ленте: нить вдоль верхней кромки с тёплым концом. */
  .line {
    position: absolute;
    top: 0;
    left: 0;
    height: 2px;
    background: linear-gradient(90deg, transparent, color-mix(in srgb, var(--ember) 55%, transparent) 40%, var(--ember));
    transition: width 0.12s linear;
  }

  .line::after {
    content: '';
    position: absolute;
    right: -3px;
    top: -2px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ember);
    box-shadow: 0 0 8px 1px color-mix(in srgb, var(--ember) 60%, transparent);
  }

  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    min-height: 68px;
    padding: 0 14px 0 10px;
    gap: 12px;
  }

  /* ── Кто: миниатюра, рядом три похожие ── */
  .left {
    grid-area: 1 / 1;
    display: flex;
    align-items: center;
    gap: 16px;
    min-width: 0;
    transition: opacity 0.18s ease;
  }

  .who {
    display: flex;
    align-items: center;
    padding: 6px;
    border: 0;
    background: none;
    font: inherit;
    color: inherit;
    cursor: pointer;
    border-radius: 16px;
  }

  .thumb {
    position: relative;
    flex: none;
    display: block;
    width: 48px;
    height: 48px;
    overflow: hidden;
    border-radius: 13px;
    background: var(--color-canvas-sunken, #efe4d3);
    box-shadow:
      0 0 0 2px var(--paper),
      0 0 0 3px color-mix(in srgb, var(--ember) 45%, transparent),
      var(--lift);
    transition: transform 0.25s cubic-bezier(0.2, 0.9, 0.25, 1.2), box-shadow 0.2s ease;
  }

  .thumb::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow: inset 0 0 0 1px color-mix(in srgb, #fff 25%, transparent);
    pointer-events: none;
  }

  .who:hover .thumb,
  .who[aria-expanded='true'] .thumb {
    transform: scale(1.06) rotate(-2deg);
    box-shadow:
      0 0 0 2px var(--paper),
      0 0 0 3px var(--ember),
      0 8px 18px -6px color-mix(in srgb, var(--ink) 40%, transparent);
  }

  .thumb :global(.thumb-img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .similar {
    display: flex;
    align-items: center;
    gap: 9px;
    padding-left: 18px;
    position: relative;
  }

  .similar::before {
    content: '';
    position: absolute;
    left: 0;
    top: 50%;
    height: 38px;
    width: 1px;
    transform: translateY(-50%);
    background: linear-gradient(180deg, transparent, var(--line), transparent);
  }

  .similar-label {
    margin-right: 4px;
    font-size: 10.5px;
    font-weight: 500;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary, #7a5a43);
  }

  .sim {
    display: block;
    width: 42px;
    height: 42px;
    overflow: hidden;
    border-radius: 12px;
    background: var(--color-canvas-sunken, #efe4d3);
    box-shadow: 0 0 0 1.5px var(--paper), 0 0 0 2.5px color-mix(in srgb, var(--line) 90%, transparent), var(--lift);
    transition: transform 0.22s cubic-bezier(0.2, 0.9, 0.25, 1.2), box-shadow 0.2s ease, opacity 0.2s ease,
      filter 0.2s ease;
  }

  .sim:hover {
    transform: translateY(-3px) scale(1.07);
    box-shadow:
      0 0 0 1.5px var(--paper),
      0 0 0 2.5px var(--ember),
      0 10px 16px -6px color-mix(in srgb, var(--ink) 40%, transparent);
  }

  /* Пока наведена одна, остальные отступают. */
  .similar:hover .sim:not(:hover) {
    opacity: 0.55;
    filter: saturate(0.6);
  }

  .sim :global(.sim-img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  /* ── Быстрая прокрутка: линейка вместо подробностей ── */
  .rail {
    grid-area: 1 / 1 / 2 / 3;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 14px;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.18s ease;
  }

  .ticks {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .ticks i {
    display: block;
    width: 5px;
    height: 10px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--line) 80%, transparent);
  }
  .ticks i.is-past { background: #b99f84; }
  .ticks i.is-now {
    height: 20px;
    background: var(--ember);
    box-shadow: 0 0 8px color-mix(in srgb, var(--ember) 50%, transparent);
  }

  .count {
    font-size: 12px;
    color: var(--color-ink-secondary, #5b4638);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  /* ── Кнопки: на виду всегда, кроме быстрой прокрутки ── */
  .extra {
    grid-area: 1 / 2;
    display: flex;
    align-items: center;
    gap: 8px;
    transition: opacity 0.18s ease;
  }

  .dock.is-fast .left,
  .dock.is-fast .extra {
    opacity: 0;
    pointer-events: none;
  }

  .dock.is-fast .rail {
    opacity: 1;
  }

  .act {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 9px 16px;
    border: 1px solid color-mix(in srgb, var(--line) 90%, transparent);
    border-radius: 999px;
    background: linear-gradient(180deg, color-mix(in srgb, #fff 60%, transparent), color-mix(in srgb, #fff 18%, transparent));
    box-shadow: inset 0 1px 0 var(--glass-edge), 0 1px 2px color-mix(in srgb, var(--ink) 8%, transparent);
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    text-decoration: none;
    color: var(--ember-deep);
    cursor: pointer;
    transition: transform 0.18s ease, box-shadow 0.18s ease, background 0.18s ease, color 0.18s ease,
      border-color 0.18s ease;
  }

  .act svg {
    flex: none;
    opacity: 0.85;
    transition: transform 0.25s cubic-bezier(0.2, 0.9, 0.25, 1.4);
  }

  .act:hover {
    transform: translateY(-1px);
    border-color: color-mix(in srgb, var(--ember) 70%, transparent);
    color: var(--ember);
    box-shadow: inset 0 1px 0 var(--glass-edge), 0 6px 14px -6px color-mix(in srgb, var(--ember) 45%, transparent);
  }

  .act:hover svg {
    transform: rotate(-8deg) scale(1.1);
  }

  .act:active {
    transform: translateY(0);
  }

  /* Открытая панель: кнопка заливается тёмным, как печать. */
  .act.is-active {
    border-color: var(--ember-deep);
    background: linear-gradient(180deg, color-mix(in srgb, var(--ember-deep) 88%, #fff), var(--ember-deep));
    color: var(--paper);
    box-shadow: inset 0 1px 0 color-mix(in srgb, #fff 22%, transparent), 0 6px 14px -6px color-mix(in srgb, var(--ember-deep) 60%, transparent);
  }

  .act.is-active svg {
    opacity: 1;
  }

  .who:focus-visible,
  .act:focus-visible,
  .photo-btn:focus-visible,
  .sim:focus-visible,
  .textlink:focus-visible {
    outline: 2px solid var(--ember);
    outline-offset: 2px;
  }

  /* История и карта: два самых «живых» перехода дока, поэтому чуть выразительнее
     остальных кнопок: засечки, тёплая подложка, акцентная кромка. */
  .act-story {
    font-family: var(--font-display, Georgia, serif);
    font-size: 15.5px;
    font-style: italic;
    font-weight: 400;
    border-color: color-mix(in srgb, var(--ember) 45%, transparent);
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--ember) 16%, #fff 30%),
      color-mix(in srgb, var(--ember) 9%, transparent)
    );
  }

  .act-story:hover {
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--ember) 22%, #fff 30%),
      color-mix(in srgb, var(--ember) 14%, transparent)
    );
  }

  .act-story.is-active {
    background: linear-gradient(180deg, color-mix(in srgb, var(--ember-deep) 88%, #fff), var(--ember-deep));
  }

  /* ── Панели над плашкой ── */
  .panel {
    position: absolute;
    left: 0;
    right: 0;
    bottom: calc(100% + 10px);
    padding: 16px 20px 14px;
    border: 1px solid color-mix(in srgb, var(--line) 85%, transparent);
    border-radius: 20px;
    background: color-mix(in srgb, var(--paper) 94%, #fff);
    box-shadow:
      inset 0 1px 0 var(--glass-edge),
      0 2px 4px color-mix(in srgb, var(--ink) 6%, transparent),
      0 22px 44px -14px color-mix(in srgb, var(--ink) 34%, transparent);
    max-height: calc(100vh - 150px);
    overflow-y: auto;
  }

  @supports (backdrop-filter: blur(1px)) {
    .panel {
      background: color-mix(in srgb, var(--paper) 84%, transparent);
      backdrop-filter: blur(18px) saturate(1.35);
    }
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 10px 18px;
    margin: 0 0 10px;
  }

  .grid dt,
  .panel h3 {
    margin: 0;
    font-size: 10.5px;
    font-weight: 500;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary, #7a5a43);
  }

  .grid dd {
    margin: 2px 0 0;
    font-size: 14px;
  }

  .blurb {
    margin: 0 0 8px;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    font-family: var(--font-display, Georgia, serif);
    font-size: 15px;
    line-height: 1.45;
    color: var(--color-ink-secondary, #5b4638);
  }

  .foot {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin: 0;
    font-size: 12px;
    color: var(--color-ink-tertiary, #7a5a43);
  }

  .textlink {
    display: inline-block;
    padding: 0;
    border: 0;
    background: none;
    font: inherit;
    font-size: 13px;
    color: var(--ember-deep);
    text-decoration: underline;
    text-decoration-color: color-mix(in srgb, var(--ember) 45%, transparent);
    text-underline-offset: 3px;
    cursor: pointer;
    transition: color 0.18s ease, text-decoration-color 0.18s ease;
  }

  .textlink:hover {
    color: var(--ember);
    text-decoration-color: var(--ember);
  }

  /* ── Фото ── */
  .photos {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .photo-btn {
    display: block;
    width: 100%;
    aspect-ratio: 1;
    padding: 0;
    overflow: hidden;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--color-canvas-sunken, #efe4d3);
    box-shadow: var(--lift);
    cursor: zoom-in;
    transition: transform 0.2s ease, box-shadow 0.2s ease;
  }

  .photo-btn:hover {
    transform: translateY(-2px);
  }

  .photo-btn img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 0.25s ease;
  }

  .photo-btn:hover img {
    transform: scale(1.05);
  }

  /* ── Связи ── */
  .tale {
    left: auto;
    right: 0;
    width: min(520px, 100%);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }

  .sec-title {
    margin: 0;
    font-family: var(--font-display, Georgia, serif);
    font-size: 21px;
    line-height: 1.2;
  }

  .excerpt {
    margin: 0;
    font-size: 14px;
    line-height: 1.5;
    color: var(--color-ink-secondary, #5b4638);
  }

  /* ── Карта в полный размер по наведению ── */
  .peek-anchor {
    position: relative;
    display: inline-flex;
  }

  /* Не внутри плашки: у неё `overflow: hidden` (нить места в ленте), и карта
     обрезалась бы по её кромке. Стоит над доком, у правого края. */
  .peek {
    position: absolute;
    right: 0;
    bottom: calc(100% + 14px);
    z-index: 3;
    padding: 14px 14px 12px;
    border: 1px solid color-mix(in srgb, var(--line) 85%, transparent);
    border-radius: 20px;
    background: color-mix(in srgb, var(--paper) 94%, #fff);
    box-shadow:
      inset 0 1px 0 var(--glass-edge),
      0 28px 56px -16px color-mix(in srgb, var(--ink) 45%, transparent);
    pointer-events: none;
  }

  /* Ширина такая, чтобы карта 5:7 помещалась над доком и в невысоком окне. */
  .peek-card {
    width: min(320px, calc((100vh - 230px) * 5 / 7));
    container-type: inline-size;
  }

  .peek-cap {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 10px 2px 0;
    font-size: 12px;
    color: var(--color-ink-secondary, #5b4638);
  }

  .peek-cap span {
    color: var(--color-ink-tertiary, #7a5a43);
  }

  /* ── Письмо ── */
  .ask {
    max-width: 440px;
    left: auto;
    right: 0;
  }

  .ask h3 {
    margin-bottom: 4px;
  }

  .note {
    margin: 0 0 10px;
    font-size: 13px;
    line-height: 1.4;
    color: var(--color-ink-secondary, #5b4638);
  }

  @media (prefers-reduced-motion: reduce) {
    .dock,
    .thumb,
    .left,
    .sim,
    .rail,
    .extra,
    .line,
    .act,
    .act svg,
    .photo-btn,
    .photo-btn img {
      transition: none;
    }

    .act:hover svg,
    .who:hover .thumb,
    .sim:hover {
      transform: none;
    }
  }
</style>
