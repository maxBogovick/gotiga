<script lang="ts">
  import { onMount } from 'svelte';
  import { api, resolveMediaUrl } from '$lib/api';
  import type { CommissionDto, CommissionStatus, ThreadDetailDto, AttachmentInput } from '$lib/types/api';
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
  let updatingId = $state<string | null>(null);
  let savingId = $state<string | null>(null);
  let savedId = $state<string | null>(null);
  let notesDraft = $state<Record<string, string>>({});
  let figurineDraft = $state<Record<string, string>>({});

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
      const res = await api.adminListCommissions({ status: statusFilter || undefined, page, perPage: PER_PAGE });
      items = res.items;
      total = res.total;
      newCount = res.newCount;
      onNewCount(res.newCount);
    } catch {
      error = 'Не удалось загрузить прошения';
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
    return new Date(iso).toLocaleString('ru-RU', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' });
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

  onMount(() => load());

  const statusLabel: Record<CommissionStatus, string> = {
    new: 'Новое', reviewing: 'На рассмотрении', accepted: 'Принято',
    in_progress: 'В работе', completed: 'Завершено', declined: 'Отклонено',
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
    return new Date(iso).toLocaleString('ru-RU', { day: '2-digit', month: '2-digit', year: '2-digit', hour: '2-digit', minute: '2-digit' });
  }
  const filterTabs: [string, string][] = [['', 'Все'], ...STATUSES.map((s) => [s, statusLabel[s]] as [string, string])];
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
    </div>
    <button onclick={() => load()} class="text-xs text-[#5f4636] hover:text-[#34251c] border border-[#34251c]/20 px-2 py-1 transition-colors" title="Обновить">↺</button>
  </div>

  <div class="flex-1 overflow-y-auto px-6 py-4">
    {#if loading}
      <div class="text-center text-[#5f4636] py-12 text-sm">Загрузка…</div>
    {:else if error}
      <div class="text-center text-red-700 py-12 text-sm">{error}</div>
    {:else if items.length === 0}
      <div class="text-center text-[#5f4636]/60 py-12 font-['Fraunces'] text-lg">Прошений нет</div>
    {:else}
      <div class="space-y-3">
        {#each items as c (c.id)}
          <div class="border border-[#34251c]/10 bg-white p-4 {c.status === 'new' ? 'border-l-4 border-l-red-400' : ''}">
            <div class="flex items-start gap-3 mb-2">
              <div class="flex-1 min-w-0">
                <div class="font-['Fraunces'] text-[#34251c] font-semibold">{c.title || '(без названия)'}</div>
                <div class="text-xs text-[#5f4636]/60 mt-0.5">{formatDate(c.createdAt)}</div>
              </div>
              <span class="text-[10px] px-2 py-0.5 border rounded flex-shrink-0 {statusColor[c.status]}">{statusLabel[c.status]}</span>
            </div>

            <div class="text-sm text-[#34251c] mb-1">
              {#if c.requesterName}<span class="font-medium">{c.requesterName}</span> · {/if}
              <a href="mailto:{c.requesterEmail}" class="text-[#c65f3c] hover:underline">{c.requesterEmail}</a>
              {#if c.requesterPhone}· <span class="text-[#5f4636]">{c.requesterPhone}</span>{/if}
            </div>

            <p class="text-sm text-[#5f4636] whitespace-pre-wrap border-l-2 border-[#d8c6b1] pl-2 mt-2">{c.description}</p>

            <div class="flex flex-wrap gap-x-4 gap-y-1 text-xs text-[#5f4636] mt-2">
              {#if c.sizeNote}<span><b>Размер:</b> {c.sizeNote}</span>{/if}
              {#if c.mood}<span><b>Настроение:</b> {c.mood}</span>{/if}
              {#if c.deadline}<span><b>Срок:</b> {c.deadline}</span>{/if}
              {#if c.budgetNote}<span><b>Бюджет:</b> {c.budgetNote}</span>{/if}
              {#if c.occasion}<span><b>Повод:</b> {c.occasion}</span>{/if}
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
                <input
                  type="text"
                  placeholder="ID фигурки (при принятии)"
                  value={figurineDraft[c.id] ?? c.figurineId ?? ''}
                  oninput={(e) => figurineDraft[c.id] = (e.target as HTMLInputElement).value}
                  class="flex-1 min-w-[140px] text-xs border border-[#34251c]/15 px-2 py-1 bg-[#fff9f0]"
                />
                <input
                  type="text"
                  placeholder="Заметка для отправителя"
                  value={notesDraft[c.id] ?? c.adminNotes ?? ''}
                  oninput={(e) => notesDraft[c.id] = (e.target as HTMLInputElement).value}
                  class="flex-[2] min-w-[180px] text-xs border border-[#34251c]/15 px-2 py-1 bg-[#fff9f0]"
                />
                <button
                  onclick={() => saveDraft(c)}
                  disabled={savingId === c.id}
                  class="text-[10px] px-3 py-1 border border-[#6f3b24] bg-[#6f3b24] text-[#fff9f0] hover:bg-[#c65f3c] hover:border-[#c65f3c] transition-colors disabled:opacity-50"
                >{savingId === c.id ? '…' : savedId === c.id ? '✓ Сохранено' : 'Сохранить'}</button>
              </div>
              <p class="text-[10px] text-[#5f4636]/50">Заметка добавляется к письму отправителю при смене статуса (Принято / В работе / Завершено / Отклонено).</p>

              <!-- Edit / delete (refused once work has started) -->
              <div class="flex items-center gap-3 flex-wrap">
                {#if c.started}
                  <span class="text-[10px] italic text-[#8a7a6a]">Работа начата — редактировать и удалять нельзя</span>
                {:else if confirmDeleteId === c.id}
                  <span class="text-[10px] text-[#a3361d]">Удалить это прошение? Автор получит уведомление.</span>
                  <button onclick={() => removeCommission(c)} disabled={deletingId === c.id} class="text-[10px] text-[#a3361d] underline disabled:opacity-50">{deletingId === c.id ? '…' : 'Да, удалить'}</button>
                  <button onclick={() => confirmDeleteId = null} class="text-[10px] text-[#5f4636] underline">Отмена</button>
                {:else}
                  <button onclick={() => confirmDeleteId = c.id} class="text-[10px] text-[#a3361d] underline">Удалить</button>
                {/if}
              </div>

              <!-- Conversation -->
              {#if c.threadId}
                <button onclick={() => toggleChat(c)} class="text-[11px] text-[#c65f3c] hover:underline">
                  {openChatId === c.id ? '▾ Скрыть переписку' : '▸ Переписка'}
                </button>
                {#if openChatId === c.id}
                  <div class="border border-[#34251c]/15 bg-[#fffaf2] p-3 mt-1">
                    {#if chatLoading}
                      <p class="text-xs text-[#5f4636] text-center py-3">Загрузка…</p>
                    {:else if chatDetail}
                      <div class="max-h-72 overflow-y-auto space-y-2 mb-3">
                        {#each chatDetail.messages as msg (msg.id)}
                          <div class="text-sm {msg.fromAdmin ? 'text-right' : 'text-left'}">
                            <div class="inline-block max-w-[85%] px-3 py-2 border {msg.fromAdmin ? 'bg-[#6f3b24]/8 border-[#6f3b24]/20' : 'bg-white border-[#34251c]/10'}">
                              <div class="text-[9px] uppercase tracking-wide text-[#5f4636]/60 mb-0.5">{msg.fromAdmin ? 'Мастер' : (chatDetail.user?.displayName ?? 'Отправитель')}</div>
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
                        <p class="text-[11px] text-[#5f4636]/60 italic">Переписка завершена — ответ откроет её снова.</p>
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
                          placeholder="Ответ отправителю…"
                          class="flex-1 text-sm border border-[#34251c]/15 px-2 py-1 bg-white resize-none"
                        ></textarea>
                        <label class="grid place-items-center w-8 h-8 border border-[#d8c6b1] cursor-pointer text-sm" title="Прикрепить изображение">
                          <input type="file" accept="image/*" multiple hidden onchange={handleChatFiles} />
                          {chatUploading ? '…' : '📎'}
                        </label>
                        <button
                          onclick={sendChat}
                          disabled={chatSending || (!chatReply.trim() && chatAttachments.length === 0)}
                          class="px-3 py-1.5 text-xs border border-[#6f3b24] bg-[#6f3b24] text-[#fff9f0] hover:bg-[#c65f3c] hover:border-[#c65f3c] transition-colors disabled:opacity-50"
                        >{chatSending ? '…' : 'Отправить'}</button>
                      </div>
                    {:else}
                      <p class="text-xs text-red-700 text-center py-3">Не удалось загрузить переписку.</p>
                    {/if}
                  </div>
                {/if}
              {:else}
                <div class="text-[11px] text-[#5f4636]/50">
                  Переписка станет доступна после того, как отправитель привяжет аккаунт.
                  <a href="mailto:{c.requesterEmail}" class="text-[#c65f3c] hover:underline">Написать на email →</a>
                </div>
              {/if}
            </div>
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
