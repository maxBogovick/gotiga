<script lang="ts">
    import { onMount } from 'svelte';
    import { api } from '$lib/api';
    import type { Figurine, FigurineListItem } from '$lib/types/api';
    import { open } from '@tauri-apps/plugin-dialog';
    import { fade, slide } from 'svelte/transition';
    import SettingsModal from '$lib/components/SettingsModal.svelte';
    import ZoneEditor from '$lib/components/admin/ZoneEditor.svelte';
    import TextEditor from '$lib/components/admin/TextEditor.svelte';
    import ReleaseManager from '$lib/components/admin/ReleaseManager.svelte';

    let figurines = $state<FigurineListItem[]>([]);
    let selectedFigurine = $state<Figurine | null>(null);
    let isSaving = $state(false);
    let isPushing = $state(false);
    let showSettings = $state(false);
    let message = $state({ text: '', type: 'info' });
    let activeTab = $state<'registry' | 'zones' | 'author' | 'workshop' | 'releases'>('registry');

    function resolveUrl(path: string | null) {
        // cabinet:// protocol is handled by Rust backend
        return path ?? '';
    }

    // Форма новой фигурки
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
        figurines = await api.getAllFigurines();
    }

    async function editFigurine(id: string) {
        const full = await api.getFigurine(id);
        if (full) {
            selectedFigurine = { ...full };
        }
    }

    function createNew() {
        selectedFigurine = { ...emptyFigurine, id: crypto.randomUUID() };
    }

    async function handlePickFile(type: 'images' | 'videos' | 'audio', stepIndex?: number) {
        if (!selectedFigurine) return;

        const filters = [];
        if (type === 'images') filters.push({ name: 'Images', extensions: ['jpg', 'png', 'webp'] });
        else if (type === 'videos') filters.push({ name: 'Videos', extensions: ['mp4', 'webm'] });
        else if (type === 'audio') filters.push({ name: 'Audio', extensions: ['mp3', 'wav', 'ogg'] });

        const selected = await open({
            multiple: false,
            filters
        });

        if (selected && typeof selected === 'string') {
            try {
                const targetFolder = type === 'audio' ? 'audio' : 'images'; 
                const backendType = type === 'videos' ? 'videos' : (type === 'audio' ? 'audio' : 'images');
                
                const localUrl = await api.importMedia(selected, backendType);
                
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
                showMessage('Файл импортирован', 'success');
            } catch (e) {
                showMessage('Ошибка импорта: ' + e, 'error');
            }
        }
    }

    function addProcessStep() {
        if (!selectedFigurine) return;
        selectedFigurine.processSteps = [...selectedFigurine.processSteps, {
            id: crypto.randomUUID(),
            stepType: 'sketch',
            description: '',
            imageUrl: ''
        }];
    }

    function removeProcessStep(index: number) {
        if (!selectedFigurine) return;
        selectedFigurine.processSteps = selectedFigurine.processSteps.filter((_, i) => i !== index);
    }

    function setFaceImage(imageId: string) {
        if (!selectedFigurine) return;
        selectedFigurine.images = selectedFigurine.images.map(img => ({
            ...img,
            imageType: img.id === imageId ? 'face' : 'full'
        }));
    }

    async function save() {
        if (!selectedFigurine) return;
        isSaving = true;
        try {
            await api.saveFigurine(selectedFigurine);
            showMessage('Сохранено в локальный архив', 'success');
            await loadFigurines();
        } catch (e) {
            showMessage('Ошибка сохранения: ' + e, 'error');
        } finally {
            isSaving = false;
        }
    }

    async function pushToServer() {
        if (!selectedFigurine) return;
        isPushing = true;
        try {
            await api.pushFigurine(selectedFigurine);
            showMessage('Успешно отправлено на сервер', 'success');
        } catch (e) {
            showMessage('Ошибка отправки: ' + e, 'error');
        } finally {
            isPushing = false;
        }
    }

    function showMessage(text: string, type = 'info') {
        message = { text, type };
        setTimeout(() => message.text = '', 3000);
    }

    onMount(loadFigurines);
</script>

<div class="h-screen bg-[#0a0806] text-[#d4c5b0] font-cinzel flex flex-col overflow-hidden">
    
    <!-- Header -->
    <header class="flex justify-between items-center px-8 py-6 border-b border-[#d4c5b0]/20 bg-[#0a0806] z-10">
        <div>
            <h1 class="text-3xl font-gothic mb-1">Реестр Смотрителя</h1>
            <p class="text-[10px] tracking-[0.3em] text-[#8a7f70] uppercase">Панель управления коллекцией</p>
        </div>
        
        <!-- Tabs -->
        <nav class="flex gap-1 bg-[#141210] p-1 rounded border border-[#d4c5b0]/20">
            <button 
                onclick={() => activeTab = 'registry'} 
                class="px-4 py-2 text-xs uppercase tracking-widest transition-colors {activeTab === 'registry' ? 'bg-[#d4c5b0]/10 text-white' : 'text-[#8a7f70] hover:text-[#d4c5b0]'}"
            >Реестр</button>
            <button 
                onclick={() => activeTab = 'zones'} 
                class="px-4 py-2 text-xs uppercase tracking-widest transition-colors {activeTab === 'zones' ? 'bg-[#d4c5b0]/10 text-white' : 'text-[#8a7f70] hover:text-[#d4c5b0]'}"
            >Зоны</button>
            <button 
                onclick={() => activeTab = 'author'} 
                class="px-4 py-2 text-xs uppercase tracking-widest transition-colors {activeTab === 'author' ? 'bg-[#d4c5b0]/10 text-white' : 'text-[#8a7f70] hover:text-[#d4c5b0]'}"
            >Автор</button>
            <button 
                onclick={() => activeTab = 'workshop'} 
                class="px-4 py-2 text-xs uppercase tracking-widest transition-colors {activeTab === 'workshop' ? 'bg-[#d4c5b0]/10 text-white' : 'text-[#8a7f70] hover:text-[#d4c5b0]'}"
            >Мастерская</button>
            <button 
                onclick={() => activeTab = 'releases'} 
                class="px-4 py-2 text-xs uppercase tracking-widest transition-colors {activeTab === 'releases' ? 'bg-[#d4c5b0]/10 text-white' : 'text-[#8a7f70] hover:text-[#d4c5b0]'}"
            >Релизы</button>
        </nav>

        <div class="flex gap-3">
            <button onclick={() => showSettings = true} class="btn-gothic border-[#d4c5b0]/20 text-[#d4c5b0] text-lg px-3 hover:bg-[#d4c5b0]/10" title="Настройки">
                ⚙
            </button>
            <a href="/" class="btn-gothic opacity-60">← В Музей</a>
        </div>
    </header>

    <SettingsModal isOpen={showSettings} onClose={() => showSettings = false} />

    <!-- Content Area -->
    <div class="flex-1 overflow-hidden p-8 relative">
        {#if activeTab === 'registry'}
            <div class="grid grid-cols-12 gap-8 h-full" in:fade>
                
                <!-- Sidebar: List -->
                <aside class="col-span-3 flex flex-col gap-4 border-r border-[#d4c5b0]/10 pr-6 overflow-hidden">
                    <div class="flex justify-between items-center shrink-0">
                        <h2 class="text-xs uppercase tracking-widest text-[#8a7f70]">Архив записей</h2>
                        <button onclick={createNew} class="btn-gothic text-[10px]">✚ Новый</button>
                    </div>
                    
                    <div class="flex-1 overflow-y-auto space-y-2 pr-2">
                        {#each figurines as fig}
                            <button 
                                onclick={() => editFigurine(fig.id)}
                                class="w-full text-left p-3 border border-[#d4c5b0]/10 hover:border-[#d4c5b0]/40 transition-colors group {selectedFigurine?.id === fig.id ? 'bg-[#d4c5b0]/5 border-[#d4c5b0]/30' : ''}"
                            >
                                <div class="text-sm font-bold group-hover:text-white transition-colors">{fig.name}</div>
                                <div class="text-[10px] uppercase opacity-40">{fig.status} • {fig.id.slice(0,8)}</div>
                            </button>
                        {/each}
                    </div>

                    <div class="pt-4 border-t border-[#d4c5b0]/10 shrink-0 flex flex-col gap-2 opacity-50 text-[10px] text-[#8a7f70] text-center">
                         Для синхронизации перейдите на вкладку "Релизы"
                    </div>
                </aside>

                <!-- Main: Editor -->
                <main class="col-span-9 bg-[#141210]/50 p-8 border border-[#d4c5b0]/10 relative h-full overflow-y-auto">
                    {#if selectedFigurine}
                        <div>
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
                                        <input bind:value={selectedFigurine.dimensions} class="input-gothic" placeholder="20x15x10 см" />
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
                                        <label class="block flex-1 pt-6">
                                            <input type="checkbox" bind:checked={selectedFigurine.isVisible} class="accent-[#d4c5b0] w-4 h-4" />
                                            <span class="ml-2 text-sm text-[#d4c5b0]">Показывать</span>
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

                            <!-- Media Section -->
                            <div class="border-t border-[#d4c5b0]/10 pt-8 mb-8">
                                <h3 class="text-xl font-gothic mb-6">Медиа-материалы</h3>
                                
                                <div class="grid grid-cols-3 gap-8">
                                    <!-- Video -->
                                    <div class="p-4 border border-dashed border-[#d4c5b0]/20">
                                        <span class="label block mb-4">Видео ролик</span>
                                        {#if selectedFigurine.videoUrl}
                                            <div class="text-[10px] mb-2 truncate text-amber-200/60">{selectedFigurine.videoUrl}</div>
                                            <button onclick={() => selectedFigurine!.videoUrl = null} class="text-[10px] text-red-800 uppercase mb-4">Удалить</button>
                                        {:else}
                                            <button onclick={() => handlePickFile('videos')} class="btn-gothic text-xs w-full">
                                                Выбрать MP4
                                            </button>
                                        {/if}
                                    </div>

                                    <!-- Audio -->
                                    <div class="p-4 border border-dashed border-[#d4c5b0]/20">
                                        <span class="label block mb-4">Атмосфера (Аудио)</span>
                                        {#if selectedFigurine.ambiencePath}
                                            <div class="text-[10px] mb-2 truncate text-amber-200/60">{selectedFigurine.ambiencePath}</div>
                                            <button onclick={() => selectedFigurine!.ambiencePath = null} class="text-[10px] text-red-800 uppercase mb-4">Удалить</button>
                                        {:else}
                                            <button onclick={() => handlePickFile('audio')} class="btn-gothic text-xs w-full">
                                                Выбрать MP3
                                            </button>
                                        {/if}
                                    </div>

                                    <!-- Images -->
                                    <div class="p-4 border border-dashed border-[#d4c5b0]/20">
                                        <span class="label block mb-4">Фотографии ({selectedFigurine.images.length})</span>
                                        <div class="flex flex-wrap gap-2 mb-4">
                                            {#each selectedFigurine.images as img}
                                                <div class="w-24 h-24 border overflow-hidden relative group transition-colors {img.imageType === 'face' ? 'border-amber-500' : 'border-[#d4c5b0]/20'}">
                                                    <img src={resolveUrl(img.url)} alt="" class="w-full h-full object-cover" />
                                                    
                                                    <!-- Remove Button -->
                                                    <button 
                                                        onclick={() => selectedFigurine!.images = selectedFigurine!.images.filter(i => i.id !== img.id)}
                                                        class="absolute top-0 right-0 bg-red-900/80 w-5 h-5 opacity-0 group-hover:opacity-100 flex items-center justify-center text-xs text-white z-10 cursor-pointer"
                                                        title="Удалить"
                                                    >✕</button>

                                                    <!-- Set Face Button -->
                                                    {#if img.imageType !== 'face'}
                                                        <button 
                                                            onclick={() => setFaceImage(img.id)}
                                                            class="absolute bottom-0 left-0 right-0 bg-black/80 text-[9px] uppercase py-1 text-center opacity-0 group-hover:opacity-100 text-[#d4c5b0] hover:bg-amber-900/80 cursor-pointer"
                                                        >
                                                            Обложка
                                                        </button>
                                                    {:else}
                                                        <div class="absolute bottom-0 left-0 right-0 bg-amber-500/80 text-black text-[9px] px-1 font-bold text-center">Обложка</div>
                                                    {/if}
                                                </div>
                                            {/each}
                                        </div>
                                        <button onclick={() => handlePickFile('images')} class="btn-gothic text-xs w-full">
                                            Добавить фото
                                        </button>
                                    </div>
                                </div>
                            </div>

                            <!-- Process Steps (Grimoire) -->
                            <div class="border-t border-[#d4c5b0]/10 pt-8 mb-8">
                                <div class="flex justify-between items-center mb-6">
                                    <h3 class="text-xl font-gothic">Гримуар (Этапы создания)</h3>
                                    <button onclick={addProcessStep} class="btn-gothic text-xs">+ Этап</button>
                                </div>

                                <div class="space-y-4">
                                    {#each selectedFigurine.processSteps as step, i}
                                        <div class="p-4 bg-[#0a0806] border border-[#d4c5b0]/10 flex gap-4 items-start">
                                            <div class="w-24 h-24 bg-[#1a1816] flex items-center justify-center border border-[#d4c5b0]/20 relative group shrink-0">
                                                {#if step.imageUrl}
                                                    <img src={resolveUrl(step.imageUrl)} alt="" class="w-full h-full object-cover" />
                                                    <button onclick={() => step.imageUrl = ''} class="absolute top-0 right-0 bg-black/50 text-white p-1 text-[10px]">✕</button>
                                                {:else}
                                                    <button onclick={() => handlePickFile('images', i)} class="text-[10px] uppercase text-[#8a7f70] hover:text-[#d4c5b0]">
                                                        Фото
                                                    </button>
                                                {/if}
                                            </div>
                                            
                                            <div class="flex-1 grid grid-cols-1 gap-2">
                                                <select bind:value={step.stepType} class="input-gothic text-xs py-1">
                                                    <option value="sketch">Sketch</option>
                                                    <option value="prototype">Prototype</option>
                                                    <option value="modeling">Modeling</option>
                                                    <option value="painting">Painting</option>
                                                    <option value="finish">Finish</option>
                                                </select>
                                                <textarea bind:value={step.description} class="input-gothic h-16 text-xs" placeholder="Описание этапа..."></textarea>
                                            </div>

                                            <button onclick={() => removeProcessStep(i)} class="text-[#8a7f70] hover:text-red-500 self-center">
                                                ✕
                                            </button>
                                        </div>
                                    {/each}
                                    {#if selectedFigurine.processSteps.length === 0}
                                        <div class="text-center text-[#8a7f70] text-xs py-4 opacity-50">Нет этапов</div>
                                    {/if}
                                </div>
                            </div>

                            <div class="flex justify-end gap-4 pb-12">
                                <button onclick={() => selectedFigurine = null} class="btn-gothic opacity-50">Отмена</button>
                                <button onclick={pushToServer} class="btn-gothic border-blue-900/40 text-blue-300 min-w-[150px]" disabled={isPushing}>
                                    {isPushing ? 'Отправка...' : '📡 Отправить в облако'}
                                </button>
                                <button onclick={save} class="btn-gothic bg-[#d4c5b0]/10 min-w-[200px]" disabled={isSaving}>
                                    {isSaving ? 'Сохранение...' : 'Записать в реестр'}
                                </button>
                            </div>
                        </div>
                    {:else}
                        <div class="h-full flex flex-col items-center justify-center text-[#8a7f70] opacity-40">
                            <span class="text-6xl mb-4">📜</span>
                            <p>Выберите запись для редактирования или создайте новую</p>
                        </div>
                    {/if}
                </main>
            </div>
        {:else if activeTab === 'zones'}
            <div in:fade class="h-full">
                <ZoneEditor />
            </div>
        {:else if activeTab === 'author'}
            <div in:fade class="h-full">
                <TextEditor category="author" />
            </div>
        {:else if activeTab === 'workshop'}
            <div in:fade class="h-full">
                <TextEditor category="workshop" />
            </div>
        {:else if activeTab === 'releases'}
            <div in:fade class="h-full">
                <ReleaseManager />
            </div>
        {/if}
    </div>

    {#if message.text}
        <div 
            class="fixed bottom-8 right-8 p-4 border transition-all z-50 {message.type === 'error' ? 'bg-red-950 border-red-800 text-red-200' : 'bg-green-950 border-green-800 text-green-200'}"
            in:slide={{ axis: 'x' }}
        >
            {message.text}
        </div>
    {/if}
</div>

<style>
    .label {
        font-size: 10px;
        text-transform: uppercase;
        letter-spacing: 0.1em;
        color: #8a7f70;
        margin-bottom: 0.5rem;
        display: block;
        font-weight: 700;
    }

    .input-gothic {
        width: 100%;
        background-color: #0a0806;
        border: 1px solid rgba(212, 197, 176, 0.2);
        padding: 0.75rem;
        font-size: 0.875rem;
        color: #d4c5b0;
        outline: none;
        transition: all 0.2s;
    }
    
    .input-gothic:focus {
        border-color: rgba(212, 197, 176, 0.6);
    }

    textarea.input-gothic {
        resize: none;
    }

    .btn-gothic {
        padding: 0.5rem 1.5rem;
        border: 1px solid rgba(212, 197, 176, 0.3);
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 0.1em;
        cursor: pointer;
        transition: all 0.2s;
        background: transparent;
    }
    
    .btn-gothic:hover {
        background-color: rgba(212, 197, 176, 0.05);
    }
    
    .btn-gothic:disabled {
        opacity: 0.3;
        cursor: not-allowed;
    }
</style>