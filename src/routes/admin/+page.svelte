<script lang="ts">
    import { onMount } from 'svelte';
    import { api, isTauri } from '$lib/api';
    import type { Figurine, FigurineListItem } from '$lib/types/api';
    import { fade, slide } from 'svelte/transition';
    import SettingsModal from '$lib/components/SettingsModal.svelte';
    import ZoneEditor from '$lib/components/admin/ZoneEditor.svelte';
    import TextEditor from '$lib/components/admin/TextEditor.svelte';
    import ReleaseManager from '$lib/components/admin/ReleaseManager.svelte';
    import ProfileEditor from '$lib/components/admin/ProfileEditor.svelte';

    // === AUTH ===
    let isAuthenticated = $state(false);
    let loginForm = $state({ login: '', password: '' });
    let loginError = $state('');
    let loginLoading = $state(false);

    async function handleLogin() {
        loginLoading = true;
        loginError = '';
        try {
            const token = await api.adminLogin(loginForm.login, loginForm.password);
            // Save token so subsequent admin API calls use it
            localStorage.setItem('gotiga_api_key', token);
            sessionStorage.setItem('gotiga_admin', '1');
            isAuthenticated = true;
            await loadFigurines();
        } catch {
            loginError = 'Неверный логин или пароль';
        } finally {
            loginLoading = false;
        }
    }

    function handleLogout() {
        sessionStorage.removeItem('gotiga_admin');
        isAuthenticated = false;
        selectedFigurine = null;
    }

    // === STATE ===
    let figurines = $state<FigurineListItem[]>([]);
    let selectedFigurine = $state<Figurine | null>(null);
    let savedSnapshot = $state<string>('');
    let isSaving = $state(false);
    let showSettings = $state(false);
    let message = $state({ text: '', type: 'info' });
    let activeTab = $state<'registry' | 'zones' | 'author' | 'workshop' | 'releases'>('registry');
    let activeAuthorSubTab = $state<'profile' | 'texts'>('profile');
    let searchQuery = $state('');
    let isDeleting = $state(false);
    let uploadingVideo = $state(false);
    let uploadingAudio = $state(false);
    let externalVideoUrl = $state('');

    let hasUnsaved = $derived(
        selectedFigurine !== null && JSON.stringify(selectedFigurine) !== savedSnapshot
    );

    let filteredFigurines = $derived(
        searchQuery.trim()
            ? figurines.filter(f => f.name.toLowerCase().includes(searchQuery.toLowerCase()))
            : figurines
    );

    function resolveUrl(path: string | null): string {
        if (!path) return '';
        if (path.startsWith('http')) return path;
        if (path.startsWith('/static/')) {
            // Web-uploaded relative path — prepend server origin
            if (typeof localStorage !== 'undefined') {
                const serverUrl = localStorage.getItem('gotiga_server_url') ?? '';
                return serverUrl ? `${serverUrl}${path}` : path;
            }
        }
        return path;
    }

    const emptyFigurine: Figurine = {
        id: '',
        name: '',
        shortText: '',
        fullDescription: '',
        dimensions: '',
        material: '',
        technique: '',
        year: new Date().getFullYear(),
        ambiencePath: null,
        videoUrl: null,
        secretText: '',
        status: 'available',
        sortOrder: 0,
        isVisible: true,
        images: [],
        processSteps: [],
        relatedItems: []
    };

    async function loadFigurines() {
        try {
            figurines = isTauri
                ? await api.getAllFigurines()
                : await api.getAllFigurinesAdmin();
        } catch (e) {
            showMessage('Ошибка загрузки: ' + e, 'error');
        }
    }

    async function moveFigurine(id: string, direction: 1 | -1) {
        const full = await api.getFigurine(id);
        if (!full) return;
        full.sortOrder = (full.sortOrder ?? 0) + direction;
        try {
            await api.saveFigurine(full);
            await loadFigurines();
            if (selectedFigurine?.id === id) {
                selectedFigurine = { ...full };
                savedSnapshot = JSON.stringify(selectedFigurine);
            }
        } catch { /* silent */ }
    }

    async function editFigurine(id: string) {
        if (hasUnsaved && !confirm('Есть несохранённые изменения. Покинуть?')) return;
        const full = await api.getFigurine(id);
        if (full) {
            selectedFigurine = { ...full };
            savedSnapshot = JSON.stringify(selectedFigurine);
        }
    }

    function createNew() {
        if (hasUnsaved && !confirm('Есть несохранённые изменения. Покинуть?')) return;
        selectedFigurine = { ...emptyFigurine, id: crypto.randomUUID(), sortOrder: figurines.length };
        savedSnapshot = '';
    }

    function duplicateFigurine(fig: FigurineListItem) {
        if (hasUnsaved && !confirm('Есть несохранённые изменения. Покинуть?')) return;
        api.getFigurine(fig.id).then(full => {
            if (!full) return;
            selectedFigurine = {
                ...full,
                id: crypto.randomUUID(),
                name: full.name + ' (копия)',
                sortOrder: figurines.length,
                isVisible: false,
            };
            savedSnapshot = '';
        });
    }

    async function deleteFigurine(fig: FigurineListItem) {
        if (!confirm(`Удалить «${fig.name}»? Это действие необратимо.`)) return;
        isDeleting = true;
        try {
            await api.deleteFigurine(fig.id);
            if (selectedFigurine?.id === fig.id) {
                selectedFigurine = null;
                savedSnapshot = '';
            }
            await loadFigurines();
            showMessage('Запись удалена', 'success');
        } catch (e) {
            showMessage('Ошибка удаления: ' + e, 'error');
        } finally {
            isDeleting = false;
        }
    }

    function pickFileWeb(type: 'images' | 'videos' | 'audio'): Promise<File> {
        return new Promise((resolve, reject) => {
            const input = document.createElement('input');
            input.type = 'file';
            if (type === 'images') input.accept = 'image/jpeg,image/png,image/webp';
            else if (type === 'videos') input.accept = 'video/mp4,video/webm';
            else input.accept = 'audio/mpeg,audio/wav,audio/ogg';
            input.onchange = () => {
                const file = input.files?.[0];
                if (file) resolve(file);
                else reject(new Error('Файл не выбран'));
            };
            input.click();
        });
    }

    async function handlePickFile(type: 'images' | 'videos' | 'audio', stepIndex?: number) {
        if (!selectedFigurine) return;
        if (type === 'videos') uploadingVideo = true;
        if (type === 'audio') uploadingAudio = true;
        try {
            let fileOrPath: string | File;
            if (isTauri) {
                const { open } = await import('@tauri-apps/plugin-dialog');
                const filters = [];
                if (type === 'images') filters.push({ name: 'Images', extensions: ['jpg', 'png', 'webp'] });
                else if (type === 'videos') filters.push({ name: 'Videos', extensions: ['mp4', 'webm', 'mov'] });
                else filters.push({ name: 'Audio', extensions: ['mp3', 'wav', 'ogg', 'm4a'] });
                const selected = await open({ multiple: false, filters });
                if (!selected || typeof selected !== 'string') return;
                fileOrPath = selected;
            } else {
                fileOrPath = await pickFileWeb(type);
            }

            const localUrl = await api.importMedia(fileOrPath, type === 'videos' ? 'videos' : type === 'audio' ? 'audio' : 'images');

            if (type === 'videos') {
                selectedFigurine.videoUrl = localUrl;
            } else if (type === 'audio') {
                selectedFigurine.ambiencePath = localUrl;
            } else if (typeof stepIndex === 'number') {
                selectedFigurine.processSteps[stepIndex].imageUrl = localUrl;
            } else {
                selectedFigurine.images = [...selectedFigurine.images, {
                    id: crypto.randomUUID(),
                    imageType: 'full',
                    url: localUrl,
                    altText: ''
                }];
            }
            showMessage('Файл загружен', 'success');
        } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : String(e);
            if (msg !== 'Файл не выбран') showMessage('Ошибка: ' + msg, 'error');
        } finally {
            if (type === 'videos') uploadingVideo = false;
            if (type === 'audio') uploadingAudio = false;
        }
    }

    function moveImage(index: number, direction: -1 | 1) {
        if (!selectedFigurine) return;
        const newIdx = index + direction;
        if (newIdx < 0 || newIdx >= selectedFigurine.images.length) return;
        const imgs = [...selectedFigurine.images];
        [imgs[index], imgs[newIdx]] = [imgs[newIdx], imgs[index]];
        selectedFigurine.images = imgs;
    }

    function addProcessStep() {
        if (!selectedFigurine) return;
        selectedFigurine.processSteps = [...selectedFigurine.processSteps, {
            id: crypto.randomUUID(), stepType: 'sketch', description: '', imageUrl: ''
        }];
    }

    function removeProcessStep(index: number) {
        if (!selectedFigurine) return;
        selectedFigurine.processSteps = selectedFigurine.processSteps.filter((_, i) => i !== index);
    }

    function setFaceImage(imageId: string) {
        if (!selectedFigurine) return;
        selectedFigurine.images = selectedFigurine.images.map(img => ({
            ...img, imageType: img.id === imageId ? 'face' : 'full'
        }));
    }

    async function save() {
        if (!selectedFigurine) return;
        isSaving = true;
        try {
            await api.saveFigurine(selectedFigurine);
            savedSnapshot = JSON.stringify(selectedFigurine);
            showMessage(isTauri ? 'Сохранено в архив' : 'Сохранено на сервере', 'success');
            await loadFigurines();
        } catch (e) {
            showMessage('Ошибка: ' + e, 'error');
        } finally {
            isSaving = false;
        }
    }

    function cancelEdit() {
        if (hasUnsaved && !confirm('Есть несохранённые изменения. Отменить?')) return;
        selectedFigurine = null;
        savedSnapshot = '';
    }

    function showMessage(text: string, type = 'info') {
        message = { text, type };
        setTimeout(() => message.text = '', 3000);
    }

    onMount(() => {
        // Check session
        const session = sessionStorage.getItem('gotiga_admin');
        if (isTauri) {
            isAuthenticated = true;
            loadFigurines();
        } else if (session === '1' && localStorage.getItem('gotiga_api_key')) {
            isAuthenticated = true;
            loadFigurines();
        }
    });
</script>

<!-- ===== LOGIN SCREEN ===== -->
{#if !isAuthenticated && !isTauri}
<div class="h-screen bg-[#0a0806] flex items-center justify-center font-cinzel">
    <div class="relative w-full max-w-sm p-10 border border-[#d4c5b0]/20 bg-[#0c0a08] shadow-[0_0_80px_rgba(0,0,0,0.9)]">
        <!-- Corner marks -->
        <div class="absolute top-0 left-0 w-5 h-5 border-t border-l border-[#d4c5b0]/40"></div>
        <div class="absolute top-0 right-0 w-5 h-5 border-t border-r border-[#d4c5b0]/40"></div>
        <div class="absolute bottom-0 left-0 w-5 h-5 border-b border-l border-[#d4c5b0]/40"></div>
        <div class="absolute bottom-0 right-0 w-5 h-5 border-b border-r border-[#d4c5b0]/40"></div>

        <div class="text-center mb-10">
            <div class="text-5xl mb-4 opacity-60">🗝</div>
            <h1 class="text-2xl font-bold tracking-[0.3em] uppercase text-[#e6decb]">Реестр</h1>
            <p class="text-[10px] tracking-widest text-[#8a7f70] uppercase mt-1">Смотрителя</p>
            <div class="w-full h-px bg-gradient-to-r from-transparent via-[#d4c5b0]/30 to-transparent mt-6"></div>
        </div>

        <form onsubmit={(e) => { e.preventDefault(); handleLogin(); }} class="space-y-6">
            <label class="block">
                <span class="text-[10px] uppercase tracking-widest text-[#8a7f70] block mb-2">Имя</span>
                <input
                    bind:value={loginForm.login}
                    type="text"
                    autocomplete="username"
                    class="w-full bg-[#141210] border border-[#d4c5b0]/20 p-3 text-sm text-[#d4c5b0] focus:border-[#d4c5b0]/60 outline-none transition-colors"
                />
            </label>
            <label class="block">
                <span class="text-[10px] uppercase tracking-widest text-[#8a7f70] block mb-2">Пароль</span>
                <input
                    bind:value={loginForm.password}
                    type="password"
                    autocomplete="current-password"
                    class="w-full bg-[#141210] border border-[#d4c5b0]/20 p-3 text-sm text-[#d4c5b0] focus:border-[#d4c5b0]/60 outline-none transition-colors"
                />
            </label>

            {#if loginError}
                <p class="text-red-400 text-xs text-center" in:fade>{loginError}</p>
            {/if}

            <button
                type="submit"
                disabled={loginLoading}
                class="w-full py-3 bg-[#d4c5b0]/10 border border-[#d4c5b0]/30 text-[#d4c5b0] text-xs uppercase tracking-widest hover:bg-[#d4c5b0]/20 transition-all disabled:opacity-40"
            >
                {loginLoading ? 'Проверка...' : 'Войти'}
            </button>
        </form>
    </div>
</div>

<!-- ===== ADMIN UI ===== -->
{:else}
<div class="h-screen bg-[#0a0806] text-[#d4c5b0] font-cinzel flex flex-col overflow-hidden">

    <!-- Header -->
    <header class="flex justify-between items-center px-6 py-4 border-b border-[#d4c5b0]/20 bg-[#0a0806] z-10 shrink-0">
        <div>
            <h1 class="text-2xl font-gothic mb-0.5">Реестр Смотрителя</h1>
            <p class="text-[10px] tracking-[0.3em] text-[#8a7f70] uppercase">Панель управления</p>
        </div>

        <nav class="flex gap-1 bg-[#141210] p-1 border border-[#d4c5b0]/20">
            {#each [['registry','Реестр'],['zones','Зоны'],['author','Автор'],['workshop','Мастерская'],['releases','Релизы']] as [tab, label]}
                <button
                    onclick={() => activeTab = tab as typeof activeTab}
                    class="px-4 py-2 text-xs uppercase tracking-widest transition-colors {activeTab === tab ? 'bg-[#d4c5b0]/10 text-white' : 'text-[#8a7f70] hover:text-[#d4c5b0]'}"
                >{label}</button>
            {/each}
        </nav>

        <div class="flex gap-2 items-center">
            <button onclick={() => showSettings = true} class="btn-gothic text-lg px-3" title="Настройки">⚙</button>
            {#if !isTauri}
                <button onclick={handleLogout} class="btn-gothic text-[10px] opacity-50 hover:opacity-100">Выход</button>
            {/if}
            <a href="/" class="btn-gothic opacity-60">← В Музей</a>
        </div>
    </header>

    <SettingsModal isOpen={showSettings} onClose={() => showSettings = false} />

    <!-- Content -->
    <div class="flex-1 overflow-hidden p-6 relative">

        {#if activeTab === 'registry'}
        <div class="grid grid-cols-12 gap-6 h-full" in:fade>

            <!-- Sidebar -->
            <aside class="col-span-3 flex flex-col gap-3 border-r border-[#d4c5b0]/10 pr-5 overflow-hidden">
                <div class="flex justify-between items-center shrink-0">
                    <h2 class="text-xs uppercase tracking-widest text-[#8a7f70]">Архив</h2>
                    <button onclick={createNew} class="btn-gothic text-[10px]">✚ Новый</button>
                </div>

                <!-- Search -->
                <div class="shrink-0 relative">
                    <input
                        bind:value={searchQuery}
                        type="text"
                        placeholder="Поиск..."
                        class="w-full bg-[#0a0806] border border-[#d4c5b0]/15 px-3 py-2 text-xs text-[#d4c5b0] outline-none focus:border-[#d4c5b0]/40 transition-colors"
                    />
                    {#if searchQuery}
                        <button
                            onclick={() => searchQuery = ''}
                            class="absolute right-2 top-1/2 -translate-y-1/2 text-[#8a7f70] hover:text-[#d4c5b0] text-xs"
                        >✕</button>
                    {/if}
                </div>

                <div class="flex-1 overflow-y-auto space-y-0.5 pr-1">
                    {#each filteredFigurines as fig (fig.id)}
                        <div class="flex items-stretch gap-0.5 group/row">
                            <!-- Main row button -->
                            <button
                                onclick={() => editFigurine(fig.id)}
                                class="flex-1 text-left px-3 py-2.5 border transition-colors min-w-0
                                    {selectedFigurine?.id === fig.id
                                        ? 'bg-[#d4c5b0]/5 border-[#d4c5b0]/30'
                                        : 'border-[#d4c5b0]/8 hover:border-[#d4c5b0]/30 border-transparent'}"
                            >
                                <div class="text-xs font-bold truncate group-hover/row:text-white transition-colors">
                                    {fig.name}
                                    {#if hasUnsaved && selectedFigurine?.id === fig.id}
                                        <span class="text-amber-400 ml-1">*</span>
                                    {/if}
                                </div>
                                <div class="text-[9px] uppercase opacity-30 flex gap-2">
                                    <span>{fig.status}</span>
                                </div>
                            </button>

                            <!-- Action column (visible on hover) -->
                            <div class="flex flex-col opacity-0 group-hover/row:opacity-100 transition-opacity shrink-0">
                                <button onclick={() => moveFigurine(fig.id, -1)}
                                    class="flex-1 w-5 border border-[#d4c5b0]/8 hover:bg-[#d4c5b0]/10 text-[#8a7f70] hover:text-[#d4c5b0] text-[9px] flex items-center justify-center"
                                    title="Выше">▲</button>
                                <button onclick={() => moveFigurine(fig.id, 1)}
                                    class="flex-1 w-5 border border-[#d4c5b0]/8 hover:bg-[#d4c5b0]/10 text-[#8a7f70] hover:text-[#d4c5b0] text-[9px] flex items-center justify-center"
                                    title="Ниже">▼</button>
                                <button onclick={() => duplicateFigurine(fig)}
                                    class="flex-1 w-5 border border-[#d4c5b0]/8 hover:bg-[#d4c5b0]/10 text-[#8a7f70] hover:text-amber-400 text-[9px] flex items-center justify-center"
                                    title="Дублировать">⎘</button>
                                <button onclick={() => deleteFigurine(fig)} disabled={isDeleting}
                                    class="flex-1 w-5 border border-[#d4c5b0]/8 hover:bg-red-950 text-[#8a7f70] hover:text-red-400 text-[9px] flex items-center justify-center"
                                    title="Удалить">✕</button>
                            </div>
                        </div>
                    {/each}

                    {#if filteredFigurines.length === 0}
                        <div class="text-center text-[#8a7f70] text-xs py-6 opacity-40">
                            {searchQuery ? 'Не найдено' : 'Нет записей'}
                        </div>
                    {/if}
                </div>

                <div class="pt-3 border-t border-[#d4c5b0]/10 shrink-0 text-[10px] text-[#8a7f70] text-center opacity-50">
                    {figurines.length} записей
                </div>
            </aside>

            <!-- Editor -->
            <main class="col-span-9 bg-[#141210]/50 border border-[#d4c5b0]/10 relative h-full overflow-y-auto">
                {#if selectedFigurine}
                    <div class="p-8">
                        <!-- Unsaved indicator bar -->
                        {#if hasUnsaved}
                            <div class="mb-6 px-4 py-2 bg-amber-950/40 border border-amber-900/40 text-amber-400 text-[10px] uppercase tracking-widest flex items-center gap-2" in:fade>
                                <span class="w-1.5 h-1.5 rounded-full bg-amber-400 animate-pulse"></span>
                                Есть несохранённые изменения
                            </div>
                        {/if}

                        <div class="grid grid-cols-2 gap-6 mb-8">
                            <div class="space-y-4">
                                <label class="block">
                                    <span class="label">Название</span>
                                    <input bind:value={selectedFigurine.name} class="input-gothic" />
                                </label>
                                <label class="block">
                                    <span class="label">Год (Anno)</span>
                                    <input type="number" bind:value={selectedFigurine.year} class="input-gothic" />
                                </label>
                                <label class="block">
                                    <span class="label">Статус</span>
                                    <select bind:value={selectedFigurine.status} class="input-gothic">
                                        <option value="available">В наличии</option>
                                        <option value="sold">Утрачено</option>
                                        <option value="reserved">Бронь</option>
                                    </select>
                                </label>
                            </div>
                            <div class="space-y-4">
                                <label class="block">
                                    <span class="label">Размеры</span>
                                    <input bind:value={selectedFigurine.dimensions} class="input-gothic" placeholder="20×15×10 см" />
                                </label>
                                <label class="block">
                                    <span class="label">Материал</span>
                                    <input bind:value={selectedFigurine.material} class="input-gothic" />
                                </label>
                                <label class="block">
                                    <span class="label">Техника</span>
                                    <input bind:value={selectedFigurine.technique} class="input-gothic" />
                                </label>
                                <div class="flex gap-4">
                                    <label class="block flex-1">
                                        <span class="label">Сортировка</span>
                                        <input type="number" bind:value={selectedFigurine.sortOrder} class="input-gothic" />
                                    </label>
                                    <label class="flex items-end gap-2 pb-3">
                                        <input type="checkbox" bind:checked={selectedFigurine.isVisible} class="accent-[#d4c5b0] w-4 h-4" />
                                        <span class="text-xs text-[#d4c5b0]">Видимый</span>
                                    </label>
                                </div>
                            </div>
                        </div>

                        <label class="block mb-6">
                            <span class="label">Краткое описание (цитата)</span>
                            <textarea bind:value={selectedFigurine.shortText} class="input-gothic h-20"></textarea>
                        </label>

                        <label class="block mb-6">
                            <span class="label">Секретный текст (для лупы)</span>
                            <textarea bind:value={selectedFigurine.secretText} class="input-gothic h-16 opacity-70"></textarea>
                        </label>

                        <label class="block mb-8">
                            <span class="label">Полная история</span>
                            <textarea bind:value={selectedFigurine.fullDescription} class="input-gothic h-40"></textarea>
                        </label>

                        <!-- Media -->
                        <div class="border-t border-[#d4c5b0]/10 pt-8 mb-8">
                            <h3 class="text-xl font-gothic mb-6">Медиа-материалы</h3>
                            <div class="grid grid-cols-2 gap-6 mb-6">
                                <!-- Video -->
                                <div class="p-4 border border-dashed border-[#d4c5b0]/20 flex flex-col gap-2">
                                    <span class="label block">Видео ролик</span>
                                    {#if selectedFigurine.videoUrl}
                                        <video
                                            src={resolveUrl(selectedFigurine.videoUrl)}
                                            controls
                                            class="w-full max-h-36 bg-black"
                                            preload="metadata"
                                        ></video>
                                        <div class="flex gap-2">
                                            <button
                                                onclick={() => handlePickFile('videos')}
                                                disabled={uploadingVideo}
                                                class="text-[10px] text-[#d4c5b0]/60 hover:text-white uppercase disabled:opacity-40"
                                            >Заменить</button>
                                            <button
                                                onclick={() => { selectedFigurine!.videoUrl = null; externalVideoUrl = ''; }}
                                                class="text-[10px] text-red-700 hover:text-red-400 uppercase"
                                            >✕ Удалить</button>
                                        </div>
                                    {:else}
                                        <div class="flex flex-col gap-2">
                                            <input
                                                type="url"
                                                bind:value={externalVideoUrl}
                                                placeholder="https://... внешняя ссылка"
                                                class="input-gothic text-xs"
                                            />
                                            {#if externalVideoUrl.trim()}
                                                <button
                                                    onclick={() => { selectedFigurine!.videoUrl = externalVideoUrl.trim(); externalVideoUrl = ''; }}
                                                    class="btn-gothic text-xs w-full"
                                                >Использовать ссылку</button>
                                            {:else}
                                                <button
                                                    onclick={() => handlePickFile('videos')}
                                                    disabled={uploadingVideo}
                                                    class="btn-gothic text-xs w-full disabled:opacity-40"
                                                >{uploadingVideo ? 'Загружается…' : 'Выбрать MP4'}</button>
                                            {/if}
                                        </div>
                                    {/if}
                                </div>
                                <!-- Audio -->
                                <div class="p-4 border border-dashed border-[#d4c5b0]/20 flex flex-col gap-2">
                                    <span class="label block">Атмосфера (аудио)</span>
                                    {#if selectedFigurine.ambiencePath}
                                        <audio
                                            src={resolveUrl(selectedFigurine.ambiencePath)}
                                            controls
                                            class="w-full"
                                            preload="metadata"
                                        ></audio>
                                        <div class="flex gap-2">
                                            <button
                                                onclick={() => handlePickFile('audio')}
                                                disabled={uploadingAudio}
                                                class="text-[10px] text-[#d4c5b0]/60 hover:text-white uppercase disabled:opacity-40"
                                            >Заменить</button>
                                            <button
                                                onclick={() => selectedFigurine!.ambiencePath = null}
                                                class="text-[10px] text-red-700 hover:text-red-400 uppercase"
                                            >✕ Удалить</button>
                                        </div>
                                    {:else}
                                        <button
                                            onclick={() => handlePickFile('audio')}
                                            disabled={uploadingAudio}
                                            class="btn-gothic text-xs w-full disabled:opacity-40"
                                        >{uploadingAudio ? 'Загружается…' : 'Выбрать MP3'}</button>
                                    {/if}
                                </div>
                            </div>

                            <!-- Images gallery -->
                            <div class="p-4 border border-dashed border-[#d4c5b0]/20">
                                <div class="flex justify-between items-center mb-4">
                                    <span class="label">Фотографии ({selectedFigurine.images.length})</span>
                                    <button onclick={() => handlePickFile('images')} class="btn-gothic text-[10px]">+ Добавить</button>
                                </div>
                                <div class="flex flex-wrap gap-3">
                                    {#each selectedFigurine.images as img, imgIdx}
                                        <div class="flex flex-col gap-1">
                                            <div class="w-28 h-28 border overflow-hidden relative group transition-colors
                                                {img.imageType === 'face' ? 'border-amber-500' : 'border-[#d4c5b0]/20'}">
                                                <img src={resolveUrl(img.url)} alt={img.altText ?? ''} class="w-full h-full object-cover" />

                                                <!-- Overlay controls -->
                                                <div class="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex flex-col items-center justify-center gap-1">
                                                    <button onclick={() => selectedFigurine!.images = selectedFigurine!.images.filter(i => i.id !== img.id)}
                                                        class="text-[10px] text-red-300 hover:text-red-100 uppercase px-2 py-0.5 border border-red-900/50 hover:bg-red-950">✕ Удалить</button>
                                                    {#if img.imageType !== 'face'}
                                                        <button onclick={() => setFaceImage(img.id)}
                                                            class="text-[9px] text-[#d4c5b0] hover:text-amber-300 uppercase px-2 py-0.5 border border-[#d4c5b0]/20">★ Обложка</button>
                                                    {:else}
                                                        <span class="text-[9px] text-amber-400 uppercase">★ Обложка</span>
                                                    {/if}
                                                    <div class="flex gap-1">
                                                        <button onclick={() => moveImage(imgIdx, -1)} disabled={imgIdx === 0}
                                                            class="text-[10px] text-[#d4c5b0] hover:text-white px-1.5 border border-[#d4c5b0]/20 disabled:opacity-30">←</button>
                                                        <button onclick={() => moveImage(imgIdx, 1)} disabled={imgIdx === selectedFigurine.images.length - 1}
                                                            class="text-[10px] text-[#d4c5b0] hover:text-white px-1.5 border border-[#d4c5b0]/20 disabled:opacity-30">→</button>
                                                    </div>
                                                </div>

                                                {#if img.imageType === 'face'}
                                                    <div class="absolute bottom-0 left-0 right-0 bg-amber-500/80 text-black text-[8px] text-center py-0.5 font-bold">ОБЛОЖКА</div>
                                                {/if}
                                            </div>
                                            <!-- Alt text -->
                                            <input
                                                bind:value={img.altText}
                                                type="text"
                                                placeholder="Alt текст..."
                                                class="w-28 bg-[#0a0806] border border-[#d4c5b0]/10 px-1.5 py-1 text-[9px] text-[#8a7f70] focus:border-[#d4c5b0]/30 outline-none"
                                            />
                                        </div>
                                    {/each}
                                </div>
                            </div>
                        </div>

                        <!-- Process Steps -->
                        <div class="border-t border-[#d4c5b0]/10 pt-8 mb-8">
                            <div class="flex justify-between items-center mb-6">
                                <h3 class="text-xl font-gothic">Гримуар (Этапы создания)</h3>
                                <button onclick={addProcessStep} class="btn-gothic text-xs">+ Этап</button>
                            </div>
                            <div class="space-y-3">
                                {#each selectedFigurine.processSteps as step, i}
                                    <div class="p-4 bg-[#0a0806] border border-[#d4c5b0]/10 flex gap-4 items-start">
                                        <div class="w-20 h-20 bg-[#1a1816] flex items-center justify-center border border-[#d4c5b0]/20 relative group shrink-0">
                                            {#if step.imageUrl}
                                                <img src={resolveUrl(step.imageUrl)} alt="" class="w-full h-full object-cover" />
                                                <button onclick={() => step.imageUrl = ''} class="absolute top-0 right-0 bg-black/70 text-white p-0.5 text-[9px] opacity-0 group-hover:opacity-100">✕</button>
                                            {:else}
                                                <button onclick={() => handlePickFile('images', i)} class="text-[10px] uppercase text-[#8a7f70] hover:text-[#d4c5b0]">Фото</button>
                                            {/if}
                                        </div>
                                        <div class="flex-1 grid gap-2">
                                            <select bind:value={step.stepType} class="input-gothic text-xs py-1.5">
                                                <option value="sketch">Sketch</option>
                                                <option value="prototype">Prototype</option>
                                                <option value="modeling">Modeling</option>
                                                <option value="painting">Painting</option>
                                                <option value="finish">Finish</option>
                                            </select>
                                            <textarea bind:value={step.description} class="input-gothic h-14 text-xs" placeholder="Описание этапа..."></textarea>
                                        </div>
                                        <button onclick={() => removeProcessStep(i)} class="text-[#8a7f70] hover:text-red-500 self-center text-sm">✕</button>
                                    </div>
                                {/each}
                                {#if selectedFigurine.processSteps.length === 0}
                                    <div class="text-center text-[#8a7f70] text-xs py-4 opacity-40">Нет этапов</div>
                                {/if}
                            </div>
                        </div>

                        <!-- Action bar -->
                        <div class="flex justify-end gap-3 pb-10">
                            <button onclick={cancelEdit} class="btn-gothic opacity-50">Отмена</button>
                            {#if isTauri}
                                <button
                                    onclick={async () => {
                                        const isPushing = true;
                                        try {
                                            await api.pushFigurine(selectedFigurine!);
                                            savedSnapshot = JSON.stringify(selectedFigurine);
                                            showMessage('Отправлено в облако', 'success');
                                        } catch(e) { showMessage('Ошибка: ' + e, 'error'); }
                                    }}
                                    class="btn-gothic border-blue-900/40 text-blue-300 min-w-[160px]"
                                >📡 В облако</button>
                            {/if}
                            <button onclick={save} disabled={isSaving}
                                class="btn-gothic min-w-[200px] transition-colors
                                    {hasUnsaved ? 'bg-amber-950/40 border-amber-800/60 text-amber-200 hover:bg-amber-900/40' : 'bg-[#d4c5b0]/10'}">
                                {isSaving ? 'Сохранение...' : hasUnsaved ? '● Сохранить изменения' : 'Сохранено ✓'}
                            </button>
                        </div>
                    </div>
                {:else}
                    <div class="h-full flex flex-col items-center justify-center text-[#8a7f70] opacity-30">
                        <span class="text-5xl mb-4">📜</span>
                        <p class="text-sm">Выберите запись или создайте новую</p>
                    </div>
                {/if}
            </main>
        </div>

        {:else if activeTab === 'zones'}
            <div in:fade class="h-full"><ZoneEditor /></div>
        {:else if activeTab === 'author'}
            <div in:fade class="h-full flex flex-col">
                <!-- Sub-tabs for author section -->
                <div class="flex gap-1 border-b border-[#d4c5b0]/10 px-4 pt-2 flex-shrink-0">
                    {#each [['profile','Профиль'],['texts','Записи']] as [sub, label]}
                        {@const authorSubTab = activeAuthorSubTab}
                        <button
                            onclick={() => activeAuthorSubTab = sub as 'profile' | 'texts'}
                            class="px-3 py-1.5 text-[10px] uppercase tracking-widest border-b-2 transition-colors -mb-px
                                {authorSubTab === sub ? 'border-[#d4c5b0]/50 text-[#d4c5b0]' : 'border-transparent text-[#8a7f70] hover:text-[#d4c5b0]'}"
                        >{label}</button>
                    {/each}
                </div>
                <div class="flex-1 overflow-hidden">
                    {#if activeAuthorSubTab === 'profile'}
                        <ProfileEditor />
                    {:else}
                        <TextEditor category="author" />
                    {/if}
                </div>
            </div>
        {:else if activeTab === 'workshop'}
            <div in:fade class="h-full"><TextEditor category="workshop" /></div>
        {:else if activeTab === 'releases'}
            <div in:fade class="h-full"><ReleaseManager /></div>
        {/if}
    </div>

    {#if message.text}
        <div
            class="fixed bottom-6 right-6 px-5 py-3 border text-sm z-50
                {message.type === 'error' ? 'bg-red-950 border-red-800 text-red-200' : 'bg-[#1a1814] border-[#d4c5b0]/30 text-[#d4c5b0]'}"
            in:slide={{ axis: 'x' }}
        >{message.text}</div>
    {/if}
</div>
{/if}

<style>
    .label {
        font-size: 10px;
        text-transform: uppercase;
        letter-spacing: 0.1em;
        color: #8a7f70;
        margin-bottom: 0.35rem;
        display: block;
        font-weight: 700;
    }

    .input-gothic {
        width: 100%;
        background-color: #0a0806;
        border: 1px solid rgba(212, 197, 176, 0.2);
        padding: 0.65rem 0.75rem;
        font-size: 0.875rem;
        color: #d4c5b0;
        outline: none;
        transition: border-color 0.2s;
        font-family: inherit;
    }

    .input-gothic:focus {
        border-color: rgba(212, 197, 176, 0.55);
    }

    textarea.input-gothic { resize: none; }

    .btn-gothic {
        padding: 0.45rem 1.25rem;
        border: 1px solid rgba(212, 197, 176, 0.3);
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        cursor: pointer;
        transition: all 0.2s;
        background: transparent;
        color: #d4c5b0;
        font-family: inherit;
    }

    .btn-gothic:hover { background-color: rgba(212, 197, 176, 0.06); }
    .btn-gothic:disabled { opacity: 0.3; cursor: not-allowed; }
</style>
