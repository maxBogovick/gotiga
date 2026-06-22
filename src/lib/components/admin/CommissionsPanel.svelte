<script lang="ts">
  import { onMount } from 'svelte';
  import { api, resolveMediaUrl } from '$lib/api';
  import type { CommissionDto, CommissionStatus, ThreadDetailDto, AttachmentInput, Figurine, FigurineListItem } from '$lib/types/api';
  import MessageAttachments from '$lib/components/MessageAttachments.svelte';

  let { onNewCount = (_n: number) => {} } = $props();

  const PER_PAGE = 20;
  const STATUSES: CommissionStatus[] = ['new', 'reviewing', 'accepted', 'in_progress', 'completed', 'declined'];

  let items = $state<CommissionDto[]>([]);
  let total = $state(0);
  let newCount = $state(0);
  let page = $state(1);
  let loading = $state(true);
  let error = $state('');
  let statusFilter = $state<'' | CommissionStatus>('');
  let similarOnly = $state(false);
  let updatingId = $state<string | null>(null);
  let savingId = $state<string | null>(null);
  let savedId = $state<string | null>(null);
  let notesDraft = $state<Record<string, string>>({});
  let figurineDraft = $state<Record<string, string>>({});

  // Registry of existing works, so the link is a pick — not a hand-typed UUID.
  let registry = $state<FigurineListItem[]>([]);
  let creatingWorkId = $state<string | null>(null);
  let registryById = $derived(new Map(registry.map((f) => [f.id, f])));
  // Resolve a tag back to a work: reference-work tags carry the exact figurine name,
  // so they become links; free tags (technique/material) won't match and stay plain.
  let registryByName = $derived(new Map(registry.filter((f) => f.name).map((f) => [f.name, f] as const)));
  let visibleItems = $derived(similarOnly ? items.filter((c) => Boolean(c.sourceFigurineId)) : items);

  // Accordion: collapse cards by default so a long list stays scannable.
  let expandedIds = $state<string[]>([]);
  function toggleExpand(id: string) {
    expandedIds = expandedIds.includes(id) ? expandedIds.filter((x) => x !== id) : [...expandedIds, id];
  }

  async function loadRegistry() {
    try { registry = await api.getAllFigurinesAdmin(); } catch { registry = []; }
  }

  // Spin up a fresh in-progress, non-public work pre-filled from the request, then
  // link it and move the petition to "in progress". Saves the master from creating a
  // figurine by hand and pasting its id back here.
  async function createWork(c: CommissionDto) {
    if (creatingWorkId) return;
    creatingWorkId = c.id;
    try {
      const fig: Figurine = {
        id: crypto.randomUUID(),
        name: c.title?.trim() || (c.requesterName ? `Petition — ${c.requesterName}` : 'New work'),
        shortText: '',
        fullDescription: c.description ?? '',
        dimensions: c.sizeNote ?? '',
        material: '',
        technique: '',
        year: new Date().getFullYear(),
        passportNumber: '',
        edition: '',
        createdPeriod: '',
        careInstructions: '',
        provenanceNote: '',
        authenticityNote: '',
        includedItems: '',
        ambiencePath: null,
        videoUrl: null,
        secretText: '',
        status: 'in_progress',
        sortOrder: registry.length,
        isVisible: false,
        isFeatured: false,
        series: null,
        images: [],
        processSteps: [],
        relatedItems: [],
      };
      await api.saveFigurine(fig);
      figurineDraft[c.id] = fig.id;
      const updated = await api.updateCommissionStatus(c.id, 'in_progress', {
        adminNotes: notesDraft[c.id] ?? c.adminNotes ?? undefined,
        figurineId: fig.id,
      });
      applyUpdate(updated);
      await loadRegistry();
      await load();
    } catch {
      // ignore — the master can retry or link manually
    } finally {
      creatingWorkId = null;
    }
  }

  // Embedded conversation
  let openChatId = $state<string | null>(null);
  let chatDetail = $state<ThreadDetailDto | null>(null);
  let chatLoading = $state(false);
  let chatReply = $state('');
  let chatSending = $state(false);
  let chatAttachments = $state<AttachmentInput[]>([]);
  let chatUploading = $state(false);

  let totalPages = $derived(Math.max(1, Math.ceil(total / PER_PAGE)));

  async function load(resetPage = false) {
    if (resetPage) page = 1;
    loading = true;
    error = '';
    try {
      const res = await api.adminListCommissions({ status: statusFilter || undefined, similar: similarOnly, page, perPage: PER_PAGE });
      items = res.items;
      total = res.total;
      newCount = res.newCount;
      onNewCount(res.newCount);
    } catch {
      error = 'Failed to load requests';
    } finally {
      loading = false;
    }
  }

  async function goTo(p: number) { page = p; await load(); }

  async function setStatus(c: CommissionDto, status: CommissionStatus) {
    updatingId = c.id;
    try {
      const updated = await api.updateCommissionStatus(c.id, status, {
        adminNotes: notesDraft[c.id] ?? undefined,
        figurineId: figurineDraft[c.id] ?? undefined,
      });
      const idx = items.findIndex((x) => x.id === c.id);
      if (idx >= 0) { items[idx] = updated; items = [...items]; }
      await load();
    } catch {
      // ignore
    } finally {
      updatingId = null;
    }
  }

  function applyUpdate(updated: CommissionDto) {
    const idx = items.findIndex((x) => x.id === updated.id);
    if (idx >= 0) { items[idx] = updated; items = [...items]; }
  }

  async function saveDraft(c: CommissionDto) {
    savingId = c.id;
    try {
      const updated = await api.updateCommissionStatus(c.id, c.status, {
        adminNotes: notesDraft[c.id] ?? c.adminNotes ?? '',
        figurineId: figurineDraft[c.id] ?? c.figurineId ?? '',
      });
      applyUpdate(updated);
      savedId = c.id;
      setTimeout(() => { if (savedId === c.id) savedId = null; }, 1800);
    } catch {
      // ignore
    } finally {
      savingId = null;
    }
  }

  async function toggleChat(c: CommissionDto) {
    if (openChatId === c.id) { openChatId = null; chatDetail = null; return; }
    if (!c.threadId) return;
    openChatId = c.id;
    chatDetail = null;
    chatReply = '';
    chatAttachments = [];
    chatLoading = true;
    try {
      chatDetail = await api.adminGetThread(c.threadId);
    } catch {
      chatDetail = null;
    } finally {
      chatLoading = false;
    }
  }

  async function handleChatFiles(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files) return;
    for (const file of Array.from(input.files)) {
      if (chatAttachments.length >= 5) break;
      chatUploading = true;
      try {
        const media = await api.importMediaWithVariants(file, 'images');
        chatAttachments = [...chatAttachments, { url: media.url, thumbUrl: media.thumbUrl ?? null }];
      } catch { /* ignore */ }
      finally { chatUploading = false; }
    }
    input.value = '';
  }

  async function sendChat() {
    if ((!chatReply.trim() && chatAttachments.length === 0) || chatSending || !chatDetail) return;
    chatSending = true;
    try {
      const msg = await api.adminReplyToThread(chatDetail.thread.id, chatReply.trim(), chatAttachments);
      chatDetail = { ...chatDetail, messages: [...chatDetail.messages, msg] };
      chatReply = '';
      chatAttachments = [];
    } catch {
      // ignore
    } finally {
      chatSending = false;
    }
  }

  function chatFormatDate(iso: string) {
    return new Date(iso).toLocaleString('en-US', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' });
  }

  // Delete petition as moderation (refused once work has started).
  // The petition's content belongs to its author; the master manages it through
  // status, notes, the figurine link and the conversation — never by rewriting it.
  let confirmDeleteId = $state<string | null>(null);
  let deletingId = $state<string | null>(null);
  async function removeCommission(c: CommissionDto) {
    deletingId = c.id;
    try {
      await api.adminDeleteCommission(c.id);
      items = items.filter((x) => x.id !== c.id);
      total = Math.max(0, total - 1);
      confirmDeleteId = null;
    } catch {
      // ignore
    } finally {
      deletingId = null;
    }
  }

  onMount(() => { load(); loadRegistry(); });

  const statusLabel: Record<CommissionStatus, string> = {
    new: 'New', reviewing: 'Reviewing', accepted: 'Accepted',
    in_progress: 'In progress', completed: 'Completed', declined: 'Declined',
  };
  const statusColor: Record<CommissionStatus, string> = {
    new: 'bg-red-100 text-red-800 border-red-200',
    reviewing: 'bg-yellow-100 text-yellow-800 border-yellow-200',
    accepted: 'bg-blue-100 text-blue-800 border-blue-200',
    in_progress: 'bg-indigo-100 text-indigo-800 border-indigo-200',
    completed: 'bg-green-100 text-green-800 border-green-200',
    declined: 'bg-gray-100 text-gray-600 border-gray-200',
  };

  function formatDate(iso: string) {
    return new Date(iso).toLocaleString('en-US', { day: '2-digit', month: '2-digit', year: '2-digit', hour: '2-digit', minute: '2-digit' });
  }
  const filterTabs: [string, string][] = [['', 'All'], ...STATUSES.map((s) => [s, statusLabel[s]] as [string, string])];
</script>

<div class="h-full flex flex-col overflow-hidden">
  <div class="flex items-center gap-3 px-6 py-3 border-b border-[#34251c]/10 flex-shrink-0 bg-[#fff9f0]">
    <h2 class="font-['Fraunces'] text-lg text-[#34251c]">
      Commissions
      {#if newCount > 0}
        <span class="ml-2 inline-flex items-center justify-center w-5 h-5 rounded-full bg-red-500 text-white text-[10px] font-bold">{newCount}</span>
      {/if}
    </h2>
    <div class="flex gap-1 ml-auto flex-wrap">
      {#each filterTabs as [val, label]}
        <button
          onclick={() => { statusFilter = val as typeof statusFilter; load(true); }}
          class="px-2.5 py-1 text-[10px] uppercase tracking-wide border transition-colors
            {statusFilter === val ? 'bg-[#34251c] text-[#fff9f0] border-[#34251c]' : 'border-[#34251c]/20 text-[#5f4636] hover:border-[#34251c]/50'}"
        >{label}</button>
      {/each}
      <button
        onclick={() => { similarOnly = !similarOnly; load(true); }}
        class="px-2.5 py-1 text-[10px] uppercase tracking-wide border transition-colors
          {similarOnly ? 'bg-[#6f3b24] text-[#fff9f0] border-[#6f3b24]' : 'border-[#34251c]/20 text-[#5f4636] hover:border-[#34251c]/50'}"
      >Similar</button>
    </div>
    <button onclick={() => load()} class="text-xs text-[#5f4636] hover:text-[#34251c] border border-[#34251c]/20 px-2 py-1 transition-colors" title="Refresh">↺</button>
  </div>

  <div class="flex-1 overflow-y-auto px-6 py-4">
    {#if loading}
      <div class="text-center text-[#5f4636] py-12 text-sm">Loading…</div>
    {:else if error}
      <div class="text-center text-red-700 py-12 text-sm">{error}</div>
    {:else if visibleItems.length === 0}
      <div class="text-center text-[#5f4636]/60 py-12 font-['Fraunces'] text-lg">No requests</div>
    {:else}
      <div class="space-y-3">
        {#each visibleItems as c (c.id)}
          {@const expanded = expandedIds.includes(c.id)}
          <div class="border border-[#34251c]/10 bg-white {c.status === 'new' ? 'border-l-4 border-l-red-400' : ''}">
            <button
              type="button"
              onclick={() => toggleExpand(c.id)}
              aria-expanded={expanded}
              class="w-full text-left flex items-start gap-3 p-4 hover:bg-[#fff9f0] transition-colors"
            >
              <span class="mt-1 text-[#5f4636]/70 text-xs flex-shrink-0">{expanded ? '▾' : '▸'}</span>
              <div class="flex-1 min-w-0">
                <div class="font-['Fraunces'] text-[#34251c] font-semibold">{c.title || '(untitled)'}</div>
                <div class="text-xs text-[#5f4636]/70 mt-0.5">
                  {formatDate(c.createdAt)}
                  {#if c.requesterName}· <span class="font-medium text-[#5f4636]">{c.requesterName}</span>{/if}
                  · {c.requesterEmail}
                </div>
                {#if !expanded}
                  <div class="text-xs text-[#5f4636]/80 mt-1 truncate">{c.description}</div>
                {/if}
              </div>
              <span class="text-[10px] px-2 py-0.5 border rounded flex-shrink-0 {statusColor[c.status]}">{statusLabel[c.status]}</span>
            </button>

          {#if expanded}
            <div class="px-4 pb-4">
            <div class="text-sm text-[#34251c] mb-1">
              {#if c.requesterName}<span class="font-medium">{c.requesterName}</span> · {/if}
              <a href="mailto:{c.requesterEmail}" class="text-[#c65f3c] hover:underline">{c.requesterEmail}</a>
              {#if c.requesterPhone}· <span class="text-[#5f4636]">{c.requesterPhone}</span>{/if}
            </div>

            <p class="text-sm text-[#5f4636] whitespace-pre-wrap border-l-2 border-[#d8c6b1] pl-2 mt-2">{c.description}</p>

            {#if c.sourceFigurineId}
              {@const source = registryById.get(c.sourceFigurineId)}
              <div class="mt-2 border border-[#6f3b24]/15 bg-[#fff9f0] p-2 flex gap-2 items-start">
                {#if source?.faceImageUrl}
                  <a href="/figurines/{c.sourceFigurineId}" target="_blank" rel="noopener" class="block w-14 h-14 border border-[#d8c6b1] overflow-hidden flex-shrink-0">
                    <img src={resolveMediaUrl(source.faceImageUrl)} alt="" class="w-full h-full object-cover" />
                  </a>
                {:else}
                  <a href="/figurines/{c.sourceFigurineId}" target="_blank" rel="noopener" class="grid place-items-center w-14 h-14 border border-[#d8c6b1] bg-[#f0e6d6] text-[10px] tracking-wide text-[#6f3b24] flex-shrink-0">GT</a>
                {/if}
                <div class="min-w-0 flex-1">
                  <div class="text-[9px] uppercase tracking-[0.14em] text-[#c65f3c] font-semibold">Create similar source</div>
                  {#if source}
                    <a href="/figurines/{c.sourceFigurineId}" target="_blank" rel="noopener" class="font-['Fraunces'] text-sm text-[#34251c] hover:text-[#c65f3c] truncate block">{source.name}</a>
                    <div class="text-[10px] text-[#5f4636]/70">{source.status}{#if source.year} · {source.year}{/if}</div>
                  {:else}
                    <a href="/figurines/{c.sourceFigurineId}" target="_blank" rel="noopener" class="font-mono text-xs text-[#34251c] hover:text-[#c65f3c] break-all">{c.sourceFigurineId}</a>
                    <div class="text-[10px] text-[#5f4636]/70">Source is not in current registry list</div>
                  {/if}
                  {#if c.similarKeepNote || c.similarChangeNote}
                    <div class="mt-1 grid gap-1 text-xs text-[#5f4636]">
                      {#if c.similarKeepNote}<p class="m-0"><b>Keep:</b> {c.similarKeepNote}</p>{/if}
                      {#if c.similarChangeNote}<p class="m-0"><b>Change:</b> {c.similarChangeNote}</p>{/if}
                    </div>
                  {/if}
                  {#if c.similarTags.length > 0}
                    <div class="mt-1 flex flex-wrap gap-1">
                      {#each c.similarTags as tag}
                        {@const work = registryByName.get(tag)}
                        {#if work}
                          <a href="/figurines/{work.id}" target="_blank" rel="noopener" class="text-[9px] px-1.5 py-0.5 border border-[#c65f3c]/40 text-[#6f3b24] bg-white hover:bg-[#c65f3c]/10 hover:border-[#c65f3c] transition-colors">{tag} ↗</a>
                        {:else}
                          <span class="text-[9px] px-1.5 py-0.5 border border-[#34251c]/10 text-[#5f4636] bg-white">{tag}</span>
                        {/if}
                      {/each}
                    </div>
                  {/if}
                </div>
              </div>
            {/if}

            <div class="flex flex-wrap gap-x-4 gap-y-1 text-xs text-[#5f4636] mt-2">
              {#if c.sizeNote}<span><b>Size:</b> {c.sizeNote}</span>{/if}
              {#if c.mood}<span><b>Mood:</b> {c.mood}</span>{/if}
              {#if c.deadline}<span><b>Deadline:</b> {c.deadline}</span>{/if}
              {#if c.budgetNote}<span><b>Budget:</b> {c.budgetNote}</span>{/if}
              {#if c.occasion}<span><b>Occasion:</b> {c.occasion}</span>{/if}
            </div>

            {#if c.attachments.length > 0}
              <div class="flex flex-wrap gap-1.5 mt-2">
                {#each c.attachments as att (att.id)}
                  <a href={resolveMediaUrl(att.url)} target="_blank" rel="noopener" class="block w-16 h-16 border border-[#d8c6b1] overflow-hidden">
                    <img src={resolveMediaUrl(att.thumbUrl ?? att.url)} alt="" class="w-full h-full object-cover" />
                  </a>
                {/each}
              </div>
            {/if}

            <!-- Admin controls -->
            <div class="mt-3 pt-2 border-t border-[#34251c]/5 space-y-2">
              <div class="flex flex-wrap gap-1">
                {#each STATUSES as s}
                  <button
                    onclick={() => setStatus(c, s)}
                    disabled={c.status === s || updatingId === c.id}
                    class="text-[10px] px-2 py-1 border transition-colors disabled:opacity-40 disabled:cursor-default
                      {c.status === s ? 'bg-[#34251c]/8 border-[#34251c]/20 text-[#34251c] font-semibold' : 'border-[#34251c]/10 text-[#5f4636] hover:border-[#34251c]/30 hover:text-[#34251c]'}"
                  >{statusLabel[s]}</button>
                {/each}
              </div>
              <div class="flex flex-wrap gap-2 items-center">
                <select
                  value={figurineDraft[c.id] ?? c.figurineId ?? ''}
                  onchange={(e) => figurineDraft[c.id] = (e.target as HTMLSelectElement).value}
                  class="flex-1 min-w-[160px] text-xs border border-[#34251c]/15 px-2 py-1 bg-[#fff9f0]"
                  title="Link this request to a work from the registry"
                >
                  <option value="">— no linked work —</option>
                  {#each registry as f (f.id)}
                    <option value={f.id}>{f.name} · {f.status}</option>
                  {/each}
                </select>
                <button
                  onclick={() => createWork(c)}
                  disabled={creatingWorkId === c.id}
                  class="text-[10px] px-2 py-1 border border-[#6f3b24]/40 text-[#6f3b24] hover:border-[#6f3b24] hover:bg-[#6f3b24]/5 transition-colors disabled:opacity-50 whitespace-nowrap"
                  title="Create a new in-progress work pre-filled from this request and link it"
                >{creatingWorkId === c.id ? '…' : '＋ Create work'}</button>
                <input
                  type="text"
                  placeholder="Note for the sender"
                  value={notesDraft[c.id] ?? c.adminNotes ?? ''}
                  oninput={(e) => notesDraft[c.id] = (e.target as HTMLInputElement).value}
                  class="flex-[2] min-w-[180px] text-xs border border-[#34251c]/15 px-2 py-1 bg-[#fff9f0]"
                />
                <button
                  onclick={() => saveDraft(c)}
                  disabled={savingId === c.id}
                  class="text-[10px] px-3 py-1 border border-[#6f3b24] bg-[#6f3b24] text-[#fff9f0] hover:bg-[#c65f3c] hover:border-[#c65f3c] transition-colors disabled:opacity-50"
                >{savingId === c.id ? '…' : savedId === c.id ? '✓ Saved' : 'Save'}</button>
              </div>
              <p class="text-[10px] text-[#5f4636]/50">The note is added to the email sent to the sender when the status changes (Accepted / In progress / Completed / Declined).</p>

              <!-- Edit / delete (refused once work has started) -->
              <div class="flex items-center gap-3 flex-wrap">
                {#if c.started}
                  <span class="text-[10px] italic text-[#8a7a6a]">Work has started — editing and deleting are disabled</span>
                {:else if confirmDeleteId === c.id}
                  <span class="text-[10px] text-[#a3361d]">Delete this request? The author will be notified.</span>
                  <button onclick={() => removeCommission(c)} disabled={deletingId === c.id} class="text-[10px] text-[#a3361d] underline disabled:opacity-50">{deletingId === c.id ? '…' : 'Yes, delete'}</button>
                  <button onclick={() => confirmDeleteId = null} class="text-[10px] text-[#5f4636] underline">Cancel</button>
                {:else}
                  <button onclick={() => confirmDeleteId = c.id} class="text-[10px] text-[#a3361d] underline">Delete</button>
                {/if}
              </div>

              <!-- Conversation -->
              {#if c.threadId}
                <button onclick={() => toggleChat(c)} class="text-[11px] text-[#c65f3c] hover:underline">
                  {openChatId === c.id ? '▾ Hide thread' : '▸ Thread'}
                </button>
                {#if openChatId === c.id}
                  <div class="border border-[#34251c]/15 bg-[#fffaf2] p-3 mt-1">
                    {#if chatLoading}
                      <p class="text-xs text-[#5f4636] text-center py-3">Loading…</p>
                    {:else if chatDetail}
                      <div class="max-h-72 overflow-y-auto space-y-2 mb-3">
                        {#each chatDetail.messages as msg (msg.id)}
                          <div class="text-sm {msg.fromAdmin ? 'text-right' : 'text-left'}">
                            <div class="inline-block max-w-[85%] px-3 py-2 border {msg.fromAdmin ? 'bg-[#6f3b24]/8 border-[#6f3b24]/20' : 'bg-white border-[#34251c]/10'}">
                              <div class="text-[9px] uppercase tracking-wide text-[#5f4636]/60 mb-0.5">{msg.fromAdmin ? 'Master' : (chatDetail.user?.displayName ?? 'Sender')}</div>
                              {#if msg.body}<p class="text-[#34251c] whitespace-pre-wrap text-left">{msg.body}</p>{/if}
                              {#if msg.attachments && msg.attachments.length > 0}
                                <MessageAttachments attachments={msg.attachments} />
                              {/if}
                              <div class="text-[9px] text-[#5f4636]/50 mt-0.5">{chatFormatDate(msg.createdAt)}</div>
                            </div>
                          </div>
                        {/each}
                      </div>
                      {#if chatDetail.thread.status === 'resolved'}
                        <p class="text-[11px] text-[#5f4636]/60 italic">Thread closed — a reply will reopen it.</p>
                      {/if}
                      {#if chatAttachments.length > 0}
                        <div class="flex flex-wrap gap-1 mb-2">
                          {#each chatAttachments as att, i (att.url)}
                            <div class="relative w-12 h-12 border border-[#d8c6b1] overflow-hidden">
                              <img src={resolveMediaUrl(att.thumbUrl ?? att.url)} alt="" class="w-full h-full object-cover" />
                              <button type="button" onclick={() => chatAttachments = chatAttachments.filter((_, idx) => idx !== i)} aria-label="×" class="absolute top-0 right-0 w-4 h-4 bg-[#34251c]/80 text-white text-[10px] leading-none">×</button>
                            </div>
                          {/each}
                        </div>
                      {/if}
                      <div class="flex items-end gap-2">
                        <textarea
                          bind:value={chatReply}
                          rows="2"
                          placeholder="Reply to the sender…"
                          class="flex-1 text-sm border border-[#34251c]/15 px-2 py-1 bg-white resize-none"
                        ></textarea>
                        <label class="grid place-items-center w-8 h-8 border border-[#d8c6b1] cursor-pointer text-sm" title="Attach image">
                          <input type="file" accept="image/*" multiple hidden onchange={handleChatFiles} />
                          {chatUploading ? '…' : '📎'}
                        </label>
                        <button
                          onclick={sendChat}
                          disabled={chatSending || (!chatReply.trim() && chatAttachments.length === 0)}
                          class="px-3 py-1.5 text-xs border border-[#6f3b24] bg-[#6f3b24] text-[#fff9f0] hover:bg-[#c65f3c] hover:border-[#c65f3c] transition-colors disabled:opacity-50"
                        >{chatSending ? '…' : 'Send'}</button>
                      </div>
                    {:else}
                      <p class="text-xs text-red-700 text-center py-3">Failed to load the thread.</p>
                    {/if}
                  </div>
                {/if}
              {:else}
                <div class="text-[11px] text-[#5f4636]/50">
                  The thread becomes available once the sender links an account.
                  <a href="mailto:{c.requesterEmail}" class="text-[#c65f3c] hover:underline">Email →</a>
                </div>
              {/if}
            </div>
            </div>
          {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>

  {#if totalPages > 1}
    <div class="flex items-center justify-end gap-1 px-6 py-3 border-t border-[#34251c]/10 flex-shrink-0 bg-[#fff9f0]">
      <button onclick={() => goTo(page - 1)} disabled={page === 1 || loading} class="px-2 py-1 text-xs border border-[#34251c]/20 text-[#5f4636] hover:border-[#34251c]/50 disabled:opacity-30 transition-colors">←</button>
      <span class="text-xs text-[#5f4636] px-2">{page} / {totalPages}</span>
      <button onclick={() => goTo(page + 1)} disabled={page === totalPages || loading} class="px-2 py-1 text-xs border border-[#34251c]/20 text-[#5f4636] hover:border-[#34251c]/50 disabled:opacity-30 transition-colors">→</button>
    </div>
  {/if}
</div>
