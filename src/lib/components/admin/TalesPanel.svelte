<script lang="ts">
  // The writing desk.
  //
  // Not a form. The gazette composer next door is a newsroom — kinds, feeds,
  // cuttings, templates — and prose written inside it comes out sounding like
  // a form filled in. Here the desk IS the paper: the title is typed in the
  // face it will be read in, the body in the measure it will be read at, and
  // the work stands in the margin exactly where the visitor will meet it.
  import { onMount, onDestroy, tick } from 'svelte';
  import { api } from '$lib/api';
  import { t, lang, type Lang } from '$lib/i18n';
  import { TITLE_MAX, DEK_MAX, BODY_MAX } from '$lib/gazette';
  import { ORNAMENT } from '$lib/tales';
  import type {
    FigurineListItem,
    GazetteLeaf,
    GazetteSeed,
    GazetteStatus,
    SaveGazetteLeafRequest,
  } from '$lib/types/api';

  let {
    seed = null,
    onSeedConsumed,
  }: { seed?: GazetteSeed | null; onSeedConsumed?: () => void } = $props();

  /** A little story stops being little somewhere around here. */
  const LITTLE = 2500;
  const AUTOSAVE_MS = 2500;
  const REORDER_MS = 600;
  const DRAFT_PREFIX = 'gotiga_tale_draft_';

  let tales = $state<GazetteLeaf[]>([]);
  let figurines = $state<FigurineListItem[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let savedAt = $state<Date | null>(null);
  let message = $state('');
  let listQuery = $state('');
  let ready = $state(false);

  let selectedId = $state<string | null>(null);
  let titleEn = $state('');
  let titleRu = $state('');
  let dekEn = $state('');
  let dekRu = $state('');
  let bodyEn = $state('');
  let bodyRu = $state('');
  let figurineId = $state('');
  let imageUrls = $state<string[]>([]);
  let slug = $state('');
  let status = $state<GazetteStatus>('draft');
  let pinned = $state(false);
  let scheduledAt = $state<string | null>(null);
  let updatedAt = $state('');

  // English is the source of truth for this house, so the desk opens in it.
  let editLang = $state<Lang>('en');
  let both = $state(false);
  let focus = $state(false);
  let sealed = $state(false);
  let uploading = $state(false);

  let figQuery = $state('');
  let figOpen = $state(false);
  let restorable = $state<Record<string, unknown> | null>(null);

  let bodyBox = $state<HTMLTextAreaElement | null>(null);
  let secondBox = $state<HTMLTextAreaElement | null>(null);
  let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
  let reorderTimer: ReturnType<typeof setTimeout> | null = null;
  let sealTimer: ReturnType<typeof setTimeout> | null = null;
  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);
  let syncing = false;

  let selectedFig = $derived(figurines.find((f) => f.id === figurineId) ?? null);
  let plate = $derived(imageUrls[0] ?? '');
  let titleNow = $derived(editLang === 'ru' ? titleRu : titleEn);
  let dekNow = $derived(editLang === 'ru' ? dekRu : dekEn);
  let bodyNow = $derived(editLang === 'ru' ? bodyRu : bodyEn);
  let otherLang = $derived<Lang>(editLang === 'ru' ? 'en' : 'ru');
  let otherTitle = $derived(editLang === 'ru' ? titleEn : titleRu);
  let otherBody = $derived(editLang === 'ru' ? bodyEn : bodyRu);
  let chars = $derived(bodyNow.length);
  let fill = $derived(Math.min(1, chars / LITTLE));
  let tooLong = $derived(chars > LITTLE);
  let open = $derived(selectedId !== null || titleEn !== '' || titleRu !== '');

  let visible = $derived.by(() => {
    const q = listQuery.trim().toLowerCase();
    if (!q) return tales;
    return tales.filter((tale) =>
      `${tale.titleEn} ${tale.titleRu} ${tale.figurineName ?? ''}`.toLowerCase().includes(q),
    );
  });
  let figMatches = $derived.by(() => {
    const q = figQuery.trim().toLowerCase();
    const list = q ? figurines.filter((f) => f.name.toLowerCase().includes(q)) : figurines;
    return list.slice(0, 12);
  });

  function fieldsKey(): string {
    return JSON.stringify({
      titleEn, titleRu, dekEn, dekRu, bodyEn, bodyRu,
      figurineId, imageUrls, slug, status, pinned, scheduledAt,
    });
  }
  let key = $derived(fieldsKey());
  let snapshot = $state('');
  let dirty = $derived(ready && key !== snapshot);

  function flash(text: string, ms = 4000) {
    message = text;
    setTimeout(() => {
      if (message === text) message = '';
    }, ms);
  }

  function draftKey(): string {
    return `${DRAFT_PREFIX}${selectedId ?? 'new'}`;
  }

  // ── Loading ────────────────────────────────────────────────────────────────

  /**
   * The shelf, in shelf order — the same order the room shows.
   *
   * The admin listing comes back newest-first, which is right for a newsroom
   * and wrong here: dragging in a recency-sorted list would write shelf
   * positions taken from an order nobody arranged, and every save would
   * quietly reshuffle the shelf. A tale with no place yet waits at the end.
   */
  async function loadTales() {
    const page = await api.adminListGazetteLeaves({ kind: 'tale', perPage: 200 });
    tales = [...page.items].sort((a, b) => {
      const ao = a.shelfOrder ?? Number.MAX_SAFE_INTEGER;
      const bo = b.shelfOrder ?? Number.MAX_SAFE_INTEGER;
      if (ao !== bo) return ao - bo;
      return (b.publishedAt ?? b.createdAt).localeCompare(a.publishedAt ?? a.createdAt);
    });
  }

  function blank() {
    selectedId = null;
    titleEn = '';
    titleRu = '';
    dekEn = '';
    dekRu = '';
    bodyEn = '';
    bodyRu = '';
    figurineId = '';
    imageUrls = [];
    slug = '';
    status = 'draft';
    pinned = false;
    scheduledAt = null;
    updatedAt = '';
    savedAt = null;
    figQuery = '';
    restorable = null;
    snapshot = fieldsKey();
  }

  function apply(leaf: GazetteLeaf) {
    selectedId = leaf.id;
    titleEn = leaf.titleEn;
    titleRu = leaf.titleRu;
    dekEn = leaf.dekEn ?? '';
    dekRu = leaf.dekRu ?? '';
    bodyEn = leaf.bodyEn ?? '';
    bodyRu = leaf.bodyRu ?? '';
    figurineId = leaf.figurineId ?? '';
    imageUrls = leaf.imageUrls?.length ? leaf.imageUrls.filter(Boolean) : leaf.imageUrl ? [leaf.imageUrl] : [];
    slug = leaf.slug;
    status = leaf.status;
    pinned = leaf.pinned;
    scheduledAt = leaf.scheduledAt;
    updatedAt = leaf.updatedAt;
    figQuery = '';
    snapshot = fieldsKey();
  }

  function openTale(leaf: GazetteLeaf) {
    if (dirty && !confirm($t('adminTalesUnsavedLeave'))) return;
    apply(leaf);
    savedAt = null;
    // A crash, a closed tab, a browser that went away mid-sentence: whatever is
    // in this browser wins only if it is newer than what the server holds.
    restorable = null;
    try {
      const raw = localStorage.getItem(draftKey());
      if (raw) {
        const draft = JSON.parse(raw) as { at?: string };
        if (draft?.at && new Date(draft.at) > new Date(leaf.updatedAt)) restorable = draft;
      }
    } catch {
      restorable = null;
    }
  }

  function restoreDraft() {
    const d = restorable as Record<string, string | string[] | boolean | null> | null;
    if (!d) return;
    titleEn = (d.titleEn as string) ?? titleEn;
    titleRu = (d.titleRu as string) ?? titleRu;
    dekEn = (d.dekEn as string) ?? dekEn;
    dekRu = (d.dekRu as string) ?? dekRu;
    bodyEn = (d.bodyEn as string) ?? bodyEn;
    bodyRu = (d.bodyRu as string) ?? bodyRu;
    restorable = null;
  }

  function dropDraft() {
    localStorage.removeItem(draftKey());
    restorable = null;
  }

  function startNew() {
    if (dirty && !confirm($t('adminTalesUnsavedLeave'))) return;
    blank();
  }

  // ── Saving ─────────────────────────────────────────────────────────────────

  function copyOr(a: string, b: string): string {
    return a.trim() || b.trim();
  }

  function payload(): SaveGazetteLeafRequest {
    return {
      slug: slug.trim() || null,
      kind: 'tale',
      status,
      // ONLY the title is mirrored, and only because the table requires both
      // titles to be non-empty. The description and the body are not: copying
      // them would put Russian prose on the English page — and, worse, the
      // copy used to land back in the editor, so writing in one language made
      // text appear in the other while the author was still typing.
      titleEn: copyOr(titleEn, titleRu),
      titleRu: copyOr(titleRu, titleEn),
      dekEn: dekEn.trim() || null,
      dekRu: dekRu.trim() || null,
      bodyEn: bodyEn.trim() || null,
      bodyRu: bodyRu.trim() || null,
      figurineId: figurineId || null,
      imageUrl: imageUrls[0] ?? null,
      // An empty ARRAY, never null: `image_urls` is a plain `Vec<String>` with
      // a serde default on the server, and `default` covers a missing field,
      // not an explicit null — sending null is a 422. An empty array is also
      // what clears a photograph the keeper has removed.
      imageUrls,
      pinned,
      scheduledAt,
    };
  }

  /**
   * Quiet save. Never changes the status — a draft stays a draft.
   *
   * Deliberately does NOT read the saved leaf back into the editor. A save
   * fires while the author is mid-sentence, and the response arrives a moment
   * later carrying the text as it was when the request left: applying it threw
   * away everything typed in between and threw the caret to the end. Only the
   * identity the server owns is taken back.
   */
  async function save(): Promise<boolean> {
    if (!titleEn.trim() && !titleRu.trim()) return false;
    const wasNew = selectedId === null;
    const sent = fieldsKey();
    saving = true;
    try {
      const saved = await api.adminSaveGazetteLeaf(payload(), selectedId ?? undefined);
      const stillTyping = fieldsKey() !== sent;
      selectedId = saved.id;
      slug = saved.slug;
      updatedAt = saved.updatedAt;
      savedAt = new Date();
      // Clean only if nothing was typed while the request was in flight;
      // otherwise stay dirty and let the next autosave carry the rest.
      if (!stillTyping) {
        snapshot = fieldsKey();
        localStorage.removeItem(draftKey());
      }
      // The list only needs re-reading when a row appears or its label moves.
      if (wasNew) {
        await loadTales();
      } else {
        tales = tales.map((tale) =>
          tale.id === saved.id
            ? { ...tale, titleEn: saved.titleEn, titleRu: saved.titleRu, status: saved.status, pinned: saved.pinned, figurineName: saved.figurineName }
            : tale,
        );
      }
      return true;
    } catch (e) {
      flash(String(e), 6000);
      return false;
    } finally {
      saving = false;
    }
  }

  async function publish() {
    if (!titleEn.trim() && !titleRu.trim()) {
      flash($t('adminTalesNeedTitle'));
      return;
    }
    status = 'published';
    if (await save()) {
      await loadTales();
      sealed = true;
      if (sealTimer) clearTimeout(sealTimer);
      sealTimer = setTimeout(() => (sealed = false), 2600);
    }
  }

  async function setStatus(next: GazetteStatus) {
    status = next;
    if (await save()) await loadTales();
  }

  async function destroy() {
    if (!selectedId) return;
    if (!confirm($t('adminTalesDeleteConfirm'))) return;
    await api.adminDeleteGazetteLeaf(selectedId);
    localStorage.removeItem(draftKey());
    blank();
    await loadTales();
  }

  // Autosave. The effect reads `key` and `snapshot`; writing `snapshot` in save()
  // re-runs it once, finds nothing changed, and stops — no loop.
  $effect(() => {
    const now = key;
    const mark = snapshot;
    if (!ready || now === mark) return;
    try {
      localStorage.setItem(draftKey(), JSON.stringify({ ...JSON.parse(now), at: new Date().toISOString() }));
    } catch {
      // A full or blocked store must never stop the writing.
    }
    if (autosaveTimer) clearTimeout(autosaveTimer);
    autosaveTimer = setTimeout(() => void save(), AUTOSAVE_MS);
    return () => {
      if (autosaveTimer) clearTimeout(autosaveTimer);
    };
  });

  // ── The paper's own gestures ───────────────────────────────────────────────

  function insertOrnament() {
    const box = bodyBox;
    if (!box) return;
    const at = box.selectionStart ?? box.value.length;
    const end = box.selectionEnd ?? at;
    const before = box.value.slice(0, at).replace(/\n+$/, '');
    const after = box.value.slice(end).replace(/^\n+/, '');
    const next = `${before}\n\n${ORNAMENT}\n\n${after}`;
    if (editLang === 'ru') bodyRu = next;
    else bodyEn = next;
    const caret = before.length + ORNAMENT.length + 4;
    void tick().then(() => {
      box.focus();
      box.setSelectionRange(caret, caret);
    });
  }

  function deskKeys(event: KeyboardEvent) {
    const meta = event.metaKey || event.ctrlKey;
    if (meta && event.key.toLowerCase() === 's') {
      event.preventDefault();
      if (autosaveTimer) clearTimeout(autosaveTimer);
      void save();
    } else if (meta && event.key === 'Enter') {
      event.preventDefault();
      insertOrnament();
    } else if (event.key === 'Escape' && focus) {
      focus = false;
    }
  }

  /** Two columns of the same tale, kept level so a translator can follow. */
  function mirrorScroll(from: HTMLTextAreaElement | null, to: HTMLTextAreaElement | null) {
    if (!both || syncing || !from || !to) return;
    const room = from.scrollHeight - from.clientHeight;
    if (room <= 0) return;
    syncing = true;
    to.scrollTop = (from.scrollTop / room) * (to.scrollHeight - to.clientHeight);
    requestAnimationFrame(() => (syncing = false));
  }

  // ── The shelf ──────────────────────────────────────────────────────────────

  function onDrop(to: number) {
    const from = dragFrom;
    dragFrom = null;
    dragOver = null;
    if (from == null || from === to) return;
    const next = [...tales];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    tales = next;
    if (reorderTimer) clearTimeout(reorderTimer);
    reorderTimer = setTimeout(async () => {
      try {
        await api.adminReorderTales(tales.map((tale) => tale.id));
        flash($t('adminTalesReordered'));
      } catch (e) {
        flash(String(e), 6000);
        await loadTales();
      }
    }, REORDER_MS);
  }

  async function useWorkPhoto() {
    if (!selectedFig?.faceImageUrl) return;
    imageUrls = [selectedFig.faceImageUrl];
  }

  async function uploadPhoto() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      uploading = true;
      try {
        const imported = await api.importMediaWithVariants(file, 'images', titleEn || titleRu || 'tale');
        imageUrls = [imported.url];
      } catch (e) {
        flash(String(e), 6000);
      } finally {
        uploading = false;
      }
    };
    input.click();
  }

  function clock(at: Date): string {
    return at.toLocaleTimeString($lang === 'ru' ? 'ru-RU' : 'en-GB', {
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function titleOf(tale: GazetteLeaf): string {
    return (editLang === 'ru' ? tale.titleRu : tale.titleEn) || tale.titleEn || tale.titleRu || $t('adminTalesUntitled');
  }

  const STATUS_TONE: Record<GazetteStatus, string> = {
    draft: 'bg-[#b9a68f]',
    scheduled: 'bg-amber-500',
    published: 'bg-emerald-600',
    archived: 'bg-[#b0a08e]',
  };

  onMount(async () => {
    try {
      const [_, figs] = await Promise.all([loadTales(), api.getAllFigurines()]);
      figurines = figs;
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      loading = false;
      blank();
      ready = true;
    }
  });

  onDestroy(() => {
    if (autosaveTimer) clearTimeout(autosaveTimer);
    if (reorderTimer) clearTimeout(reorderTimer);
    if (sealTimer) clearTimeout(sealTimer);
  });

  // Arriving from a work's form: the desk opens with that work already pinned.
  $effect(() => {
    if (!seed || !ready) return;
    const s = seed;
    onSeedConsumed?.();
    if (s.leafId) {
      const existing = tales.find((tale) => tale.id === s.leafId);
      if (existing) {
        openTale(existing);
        return;
      }
    }
    blank();
    if (s.figurineId) figurineId = s.figurineId;
    if (s.imageUrls?.length) imageUrls = [...s.imageUrls];
  });
</script>

<svelte:window on:keydown={deskKeys} />

<div class="h-full flex text-[#34251c]" class:focus-mode={focus}>
  <!-- ── The shelf ──────────────────────────────────────────────────────── -->
  <aside class="shelf-pane w-[260px] flex-shrink-0 border-r border-[#34251c]/10 flex flex-col">
    <div class="p-3 border-b border-[#34251c]/10 space-y-2">
      <button
        onclick={startNew}
        class="w-full py-2 text-[10px] uppercase tracking-[0.18em] border border-[#c65f3c]/40 text-[#c65f3c] hover:bg-[#c65f3c]/5 transition-colors"
      >{$t('adminTalesNew')}</button>
      <input
        bind:value={listQuery}
        placeholder={$t('adminTalesSearch')}
        class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 focus:border-[#34251c]/35 outline-none"
      />
    </div>

    <div class="flex-1 overflow-y-auto">
      {#if loading}
        <p class="p-3 text-xs text-[#5f4636]">…</p>
      {:else if visible.length === 0}
        <p class="p-3 text-xs italic text-[#5f4636]">{$t('adminTalesEmpty')}</p>
      {:else}
        <p class="px-3 py-2 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminTalesDragHint')}</p>
        <ul class="pb-4">
          {#each visible as tale, i (tale.id)}
            <li
              draggable={!listQuery}
              ondragstart={() => (dragFrom = i)}
              ondragover={(e) => { e.preventDefault(); dragOver = i; }}
              ondragleave={() => { if (dragOver === i) dragOver = null; }}
              ondrop={(e) => { e.preventDefault(); onDrop(i); }}
              ondragend={() => { dragFrom = null; dragOver = null; }}
              class="border-b border-[#34251c]/5 {dragOver === i ? 'bg-[#c65f3c]/10' : ''} {dragFrom === i ? 'opacity-40' : ''}"
            >
              <button
                onclick={() => openTale(tale)}
                class="w-full text-left px-3 py-2.5 flex gap-2 items-start hover:bg-[#34251c]/[0.04] transition-colors
                  {selectedId === tale.id ? 'bg-[#34251c]/[0.06]' : ''}"
              >
                <span class="mt-1.5 w-1.5 h-1.5 rounded-full flex-shrink-0 {STATUS_TONE[tale.status]}"></span>
                <span class="min-w-0">
                  <span class="block text-[13px] leading-snug truncate" style="font-family: 'Cormorant Garamond', Georgia, serif;">
                    {titleOf(tale)}
                  </span>
                  {#if tale.figurineName}
                    <span class="block text-[10px] text-[#8a6a55] truncate">{tale.figurineName}</span>
                  {/if}
                </span>
                {#if tale.pinned}<span class="ml-auto text-[#c65f3c] text-[10px]">◆</span>{/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </aside>

  <!-- ── The desk ───────────────────────────────────────────────────────── -->
  <div class="flex-1 flex flex-col min-w-0 bg-[#f8f1e7]">
    <div class="desk-chrome flex items-center gap-3 px-4 py-2 border-b border-[#34251c]/10 text-[10px] uppercase tracking-[0.16em]">
      <div class="flex border border-[#34251c]/15">
        {#each ['en', 'ru'] as code}
          <button
            onclick={() => (editLang = code as Lang)}
            class="px-2 py-1 transition-colors {editLang === code ? 'bg-[#34251c]/10 text-[#34251c]' : 'text-[#8a6a55]'}"
          >{code}</button>
        {/each}
      </div>
      <button
        onclick={() => (both = !both)}
        class="px-2 py-1 border transition-colors {both ? 'border-[#c65f3c]/50 text-[#c65f3c]' : 'border-[#34251c]/15 text-[#8a6a55]'}"
      >{$t('adminTalesBoth')}</button>
      <button
        onclick={insertOrnament}
        class="px-2 py-1 border border-[#34251c]/15 text-[#8a6a55] hover:text-[#34251c] transition-colors"
        title={$t('adminTalesOrnament')}
        aria-label={$t('adminTalesOrnament')}
      >{ORNAMENT}</button>
      <button
        onclick={() => (focus = !focus)}
        class="px-2 py-1 border border-[#34251c]/15 text-[#8a6a55] hover:text-[#34251c] transition-colors"
      >{focus ? $t('adminTalesFocusExit') : $t('adminTalesFocus')}</button>

      <span class="ml-auto normal-case tracking-normal text-[11px] text-[#8a6a55]">
        {#if saving}{$t('adminTalesSaving')}
        {:else if savedAt}{$t('adminTalesSaved').replace('{time}', clock(savedAt))}
        {:else if dirty}{$t('adminTalesUnsaved')}{/if}
      </span>
    </div>

    <div class="flex-1 overflow-y-auto">
      <div class="desk mx-auto px-6 py-8">
        {#if restorable}
          <div class="mb-6 px-3 py-2 border border-[#c65f3c]/35 bg-[#c65f3c]/5 text-xs flex items-center gap-3">
            <span class="flex-1">{$t('adminTalesRestore')}</span>
            <button onclick={restoreDraft} class="uppercase tracking-[0.14em] text-[10px] text-[#c65f3c]">{$t('adminTalesRestoreDo')}</button>
            <button onclick={dropDraft} class="uppercase tracking-[0.14em] text-[10px] text-[#8a6a55]">{$t('adminTalesRestoreDrop')}</button>
          </div>
        {/if}

        <div class="paper">
          <input
            bind:value={
              () => titleNow,
              (v) => { if (editLang === 'ru') titleRu = v; else titleEn = v; }
            }
            maxlength={TITLE_MAX}
            placeholder={$t('adminTalesTitlePh')}
            class="paper-title"
          />
          <input
            bind:value={
              () => dekNow,
              (v) => { if (editLang === 'ru') dekRu = v; else dekEn = v; }
            }
            maxlength={DEK_MAX}
            placeholder={$t('adminTalesEpigraphPh')}
            class="paper-epigraph"
          />

          <div class="columns" class:two={both}>
            <textarea
              bind:this={bodyBox}
              bind:value={
                () => bodyNow,
                (v) => { if (editLang === 'ru') bodyRu = v; else bodyEn = v; }
              }
              onscroll={() => mirrorScroll(bodyBox, secondBox)}
              maxlength={BODY_MAX}
              placeholder={$t('adminTalesBodyPh')}
              class="paper-body"
            ></textarea>

            {#if both}
              <textarea
                bind:this={secondBox}
                bind:value={
                  () => otherBody,
                  (v) => { if (otherLang === 'ru') bodyRu = v; else bodyEn = v; }
                }
                onscroll={() => mirrorScroll(secondBox, bodyBox)}
                maxlength={BODY_MAX}
                placeholder={otherTitle || otherLang.toUpperCase()}
                class="paper-body second"
              ></textarea>
            {/if}
          </div>

          <!-- Length as a shadow, not a number: a rising hairline that turns
               copper once the little story has stopped being little. -->
          <div class="measure-row">
            <span class="measure-label" class:over={tooLong}>
              {tooLong ? $t('adminTalesTooLong') : $t('adminTalesLength')} — {chars}
            </span>
            <span class="measure">
              <span class="measure-fill" class:over={tooLong} style="width: {fill * 100}%"></span>
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- ── The margin: the work, the status, the seal ──────────────────── -->
    <div class="desk-chrome border-t border-[#34251c]/10 px-4 py-3 flex items-start gap-5 text-xs">
      <div class="w-[220px] flex-shrink-0">
        <p class="text-[9px] uppercase tracking-[0.16em] text-[#8a6a55] mb-1.5">{$t('adminTalesWork')}</p>
        {#if selectedFig}
          <div class="flex gap-2 items-start">
            {#if plate}
              <img src={plate} alt="" class="w-12 h-14 object-cover border border-[#d8c6b1] bg-[#1a120e]" />
            {/if}
            <div class="min-w-0">
              <p class="truncate">{selectedFig.name}</p>
              <div class="flex flex-wrap gap-2 mt-1 text-[10px]">
                <button onclick={() => { figurineId = ''; }} class="text-[#8a6a55] hover:text-[#34251c]">{$t('adminTalesWorkClear')}</button>
                {#if !plate && selectedFig.faceImageUrl}
                  <button onclick={useWorkPhoto} class="text-[#c65f3c]">{$t('adminTalesUseWorkPhoto')}</button>
                {/if}
                <button onclick={uploadPhoto} class="text-[#8a6a55] hover:text-[#34251c]" disabled={uploading}>
                  {uploading ? '…' : $t('adminTalesUpload')}
                </button>
                {#if plate}
                  <button onclick={() => (imageUrls = [])} class="text-[#8a6a55] hover:text-[#34251c]">{$t('adminTalesDropPhoto')}</button>
                {/if}
              </div>
            </div>
          </div>
        {:else}
          <div class="relative">
            <input
              bind:value={figQuery}
              onfocus={() => (figOpen = true)}
              onblur={() => setTimeout(() => (figOpen = false), 150)}
              placeholder={$t('adminTalesWorkSearch')}
              class="w-full px-2 py-1 bg-transparent border border-[#34251c]/15 focus:border-[#34251c]/35 outline-none text-xs"
            />
            {#if figOpen && figMatches.length}
              <ul class="absolute bottom-full left-0 right-0 mb-1 max-h-56 overflow-y-auto bg-[#f8f1e7] border border-[#34251c]/15 shadow-lg z-20">
                {#each figMatches as fig (fig.id)}
                  <li>
                    <button
                      onclick={() => { figurineId = fig.id; figQuery = ''; figOpen = false; }}
                      class="w-full text-left px-2 py-1.5 hover:bg-[#34251c]/5 truncate"
                    >{fig.name}</button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}
      </div>

      <div class="flex-1 min-w-0">
        <p class="text-[9px] uppercase tracking-[0.16em] text-[#8a6a55] mb-1.5">{$t('adminTalesState')}</p>
        <div class="flex flex-wrap items-center gap-2">
          {#each [['draft', 'adminTalesStatusDraft'], ['scheduled', 'adminTalesStatusScheduled'], ['published', 'adminTalesStatusPublished'], ['archived', 'adminTalesStatusArchived']] as [value, label]}
            <button
              onclick={() => setStatus(value as GazetteStatus)}
              disabled={!open}
              class="px-2 py-1 text-[10px] uppercase tracking-[0.14em] border transition-colors
                {status === value ? 'border-[#34251c]/40 bg-[#34251c]/5 text-[#34251c]' : 'border-[#34251c]/12 text-[#8a6a55] hover:text-[#34251c]'}"
            >{$t(label as never)}</button>
          {/each}

          {#if status === 'scheduled'}
            <input
              type="datetime-local"
              value={scheduledAt ? scheduledAt.slice(0, 16) : ''}
              onchange={(e) => (scheduledAt = e.currentTarget.value ? new Date(e.currentTarget.value).toISOString() : null)}
              class="px-2 py-1 bg-transparent border border-[#34251c]/15 text-[11px]"
            />
          {/if}

          <label class="flex items-center gap-1.5 text-[10px] text-[#5f4636] ml-2">
            <input type="checkbox" bind:checked={pinned} class="accent-[#c65f3c]" />
            {$t('adminTalesPinned')}
          </label>
        </div>
      </div>

      <div class="flex-shrink-0 flex items-center gap-3">
        {#if sealed}
          <span class="seal">{$t('adminTalesPublishedSeal')}</span>
        {/if}
        <button
          onclick={publish}
          disabled={!open || saving}
          class="px-3 py-2 text-[10px] uppercase tracking-[0.18em] border border-[#c65f3c]/50 text-[#c65f3c] hover:bg-[#c65f3c]/5 transition-colors disabled:opacity-40"
        >{$t('adminTalesPublish')}</button>
        {#if selectedId}
          <button onclick={destroy} class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55] hover:text-red-700">
            {$t('adminTalesDelete')}
          </button>
        {/if}
      </div>
    </div>

    {#if message}
      <p class="px-4 py-2 text-xs border-t border-[#34251c]/10 text-[#6f3b24]">{message}</p>
    {/if}
  </div>
</div>

<style>
  /* The desk is the paper: same faces, same measure, same colour the room
     will read it in. Nothing here is a labelled field. */
  .desk { max-width: 780px; }

  .paper-title,
  .paper-epigraph,
  .paper-body {
    display: block;
    width: 100%;
    background: transparent;
    border: none;
    outline: none;
    color: #34251c;
    font-family: 'Cormorant Garamond', Georgia, serif;
  }
  .paper-title::placeholder,
  .paper-epigraph::placeholder,
  .paper-body::placeholder { color: #b9a68f; }

  .paper-title {
    font-size: clamp(30px, 3.4vw, 46px);
    font-weight: 300;
    line-height: 1.04;
    letter-spacing: -0.012em;
    margin-bottom: 12px;
  }

  .paper-epigraph {
    font-size: 19px;
    font-weight: 300;
    font-style: italic;
    color: #5f4636;
    margin-bottom: 26px;
  }

  .columns { display: grid; gap: 24px; }
  .columns.two { grid-template-columns: 1fr 1fr; }

  .paper-body {
    font-size: 19px;
    line-height: 1.72;
    min-height: 46vh;
    resize: vertical;
  }
  .paper-body.second { color: #6f5847; }

  .measure-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 18px;
  }

  .measure-label {
    flex-shrink: 0;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #8a6a55;
  }
  .measure-label.over { color: #c65f3c; }

  .measure {
    flex: 1;
    height: 1px;
    background: rgba(52, 37, 28, 0.1);
  }
  .measure-fill {
    display: block;
    height: 1px;
    background: rgba(52, 37, 28, 0.35);
    transition: width 0.4s ease, background-color 0.4s ease;
  }
  .measure-fill.over { background: #c65f3c; }

  /* Wax, the way the house does it everywhere else. */
  .seal {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 8px 14px;
    border-radius: 999px;
    background: radial-gradient(circle at 32% 28%, #d0704a 0%, #a4462a 60%, #7d3520 100%);
    color: #fbeee2;
    font-size: 9px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    box-shadow: inset 0 1px 0 rgba(255, 220, 200, 0.4), 0 2px 6px rgba(52, 37, 28, 0.25);
    animation: press 0.5s cubic-bezier(0.2, 0.9, 0.3, 1);
  }
  @keyframes press {
    from { transform: scale(1.5) rotate(-8deg); opacity: 0; }
    to { transform: scale(1) rotate(-2deg); opacity: 1; }
  }

  /* Nothing but paper. */
  .focus-mode :global(.shelf-pane),
  .focus-mode :global(.desk-chrome) { display: none; }

  @media (prefers-reduced-motion: reduce) {
    .seal { animation: none; }
    .measure-fill { transition: none; }
  }
</style>
