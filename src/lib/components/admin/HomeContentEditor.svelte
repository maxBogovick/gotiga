<script lang="ts">
  import { onMount } from 'svelte';
  import { api, isTauri } from '$lib/api';
  import type { FigurineListItem, HomeContent } from '$lib/types/api';
  import { t } from '$lib/i18n';

  let content = $state<HomeContent>({
    title: null,
    kicker: '',
    lead: '',
    heroFigurineId: null,
    heroCaptionTitle: '',
    heroCaptionMeta: '',
    heroCaptionCta: '',
    heroMode: 'auto',
  });
  let figurines = $state<FigurineListItem[]>([]);
  let bgImage = $state<string | null>(null);
  let savedSnapshot = $state('');
  let isLoading = $state(true);
  let isSaving = $state(false);
  let isUploadingBg = $state(false);
  let message = $state('');

  let hasUnsaved = $derived(JSON.stringify(content) !== savedSnapshot);
  let selectedHeroFigurine = $derived(
    content.heroFigurineId
      ? figurines.find((item) => item.id === content.heroFigurineId) ?? null
      : null
  );

  function normalizeHomeContent(loaded: HomeContent): HomeContent {
    const legacyCaption = loaded.heroCaptionTitle ?? loaded.title ?? '';
    return {
      title: null,
      kicker: loaded.kicker ?? '',
      lead: loaded.lead ?? '',
      heroFigurineId: loaded.heroFigurineId ?? null,
      heroCaptionTitle: legacyCaption,
      heroCaptionMeta: loaded.heroCaptionMeta ?? '',
      heroCaptionCta: loaded.heroCaptionCta ?? '',
      heroMode: loaded.heroMode ?? 'auto',
    };
  }

  onMount(async () => {
    try {
      const [loaded, loadedFigurines, loadedBg] = await Promise.all([
        api.getHomeContent(),
        api.getAllFigurinesAdmin().catch(() => api.getAllFigurines()),
        api.getMainBackground().catch(() => null),
      ]);
      figurines = loadedFigurines;
      bgImage = loadedBg;
      content = normalizeHomeContent(loaded);
      savedSnapshot = JSON.stringify(content);
    } finally {
      isLoading = false;
    }
  });

  function selectHeroFigurine(id: string) {
    content.heroFigurineId = id || null;
    const fig = figurines.find((item) => item.id === id) ?? null;
    if (fig && !content.heroCaptionTitle?.trim()) content.heroCaptionTitle = fig.name;
  }

  function clearHeroFigurine() {
    content.heroFigurineId = null;
  }

  function useSelectedName() {
    if (!selectedHeroFigurine) return;
    content.heroCaptionTitle = selectedHeroFigurine.name;
  }

  async function changeBackground() {
    isUploadingBg = true;
    message = '';
    try {
      let fileOrPath: string | File;
      if (isTauri) {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const selected = await open({ multiple: false, filters: [{ name: 'Images', extensions: ['jpg', 'png', 'webp'] }] });
        if (!selected || typeof selected !== 'string') return;
        fileOrPath = selected;
      } else {
        fileOrPath = await new Promise<File>((resolve, reject) => {
          const input = document.createElement('input');
          input.type = 'file';
          input.accept = 'image/jpeg,image/png,image/webp';
          input.onchange = () => {
            const file = input.files?.[0];
            if (file) resolve(file); else reject(new Error('no file'));
          };
          input.click();
        });
      }
      bgImage = await api.setMainBackground(fileOrPath);
      message = 'Hero-фото обновлено';
    } catch (e) {
      if (String(e) !== 'Error: no file') {
        console.error('Failed to update hero image:', e);
        message = 'Не удалось обновить hero-фото';
      }
    } finally {
      isUploadingBg = false;
    }
  }

  async function save() {
    isSaving = true;
    message = '';
    try {
      const payload = {
        title: null,
        kicker: content.kicker?.trim() || null,
        lead: content.lead?.trim() || null,
        heroFigurineId: content.heroFigurineId || null,
        heroCaptionTitle: content.heroCaptionTitle?.trim() || null,
        heroCaptionMeta: content.heroCaptionMeta?.trim() || null,
        heroCaptionCta: content.heroCaptionCta?.trim() || null,
        heroMode: content.heroMode ?? 'auto',
      };
      await api.saveHomeContent(payload);
      content = {
        title: null,
        kicker: payload.kicker ?? '',
        lead: payload.lead ?? '',
        heroFigurineId: payload.heroFigurineId,
        heroCaptionTitle: payload.heroCaptionTitle ?? '',
        heroCaptionMeta: payload.heroCaptionMeta ?? '',
        heroCaptionCta: payload.heroCaptionCta ?? '',
        heroMode: payload.heroMode,
      };
      savedSnapshot = JSON.stringify(content);
      message = 'Главная обновлена';
    } catch (e) {
      console.error('Failed to save home content:', e);
      message = 'Не удалось сохранить текст главной';
    } finally {
      isSaving = false;
    }
  }

  function resetToFallbacks() {
    content = {
      title: null,
      kicker: '',
      lead: '',
      heroFigurineId: null,
      heroCaptionTitle: '',
      heroCaptionMeta: '',
      heroCaptionCta: '',
      heroMode: 'auto',
    };
  }
</script>

<section class="h-full overflow-y-auto bg-[#fff9f0]/50 border border-[#34251c]/10">
  <div class="p-8 max-w-4xl">
    <div class="mb-8 flex items-start justify-between gap-6">
      <div>
        <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Homepage</p>
        <h2 class="font-gothic text-3xl text-[#6f3b24]">Первый экран главной</h2>
        <p class="mt-3 text-sm leading-7 text-[#5f4636] max-w-2xl">
          Здесь связывается hero-фото, подпись и работа, которую откроет пользователь. Так картинка и ссылка не расходятся.
        </p>
      </div>

      {#if hasUnsaved}
        <span class="shrink-0 px-3 py-1.5 text-[10px] uppercase tracking-[0.1em] border border-amber-700/30 bg-amber-50 text-amber-800">
          Есть изменения
        </span>
      {/if}
    </div>

    {#if isLoading}
      <p class="text-xs uppercase tracking-[0.12em] text-[#5f4636] animate-pulse">Загрузка…</p>
    {:else}
      <div class="space-y-6">
        <div class="border border-[#34251c]/10 bg-[#f8f1e7]/70 p-6 space-y-4">
          <div>
            <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Hero mode</p>
            <h3 class="font-gothic text-2xl text-[#6f3b24]">Режим первого экрана</h3>
            <p class="mt-2 text-sm leading-6 text-[#5f4636] max-w-2xl">
              Витрина ведёт к доступным работам. Редкий выпуск лучше, когда доступных работ нет и нужно вести в архив или к работам в процессе.
            </p>
          </div>

          <div class="mode-grid">
            <label class:active={content.heroMode === 'auto'}>
              <input type="radio" bind:group={content.heroMode} value="auto" />
              <span>Авто</span>
              <small>Если доступных нет, включит редкий выпуск</small>
            </label>
            <label class:active={content.heroMode === 'showcase'}>
              <input type="radio" bind:group={content.heroMode} value="showcase" />
              <span>Витрина</span>
              <small>Главный CTA ведёт к доступным работам</small>
            </label>
            <label class:active={content.heroMode === 'release'}>
              <input type="radio" bind:group={content.heroMode} value="release" />
              <span>Редкий выпуск</span>
              <small>Главный CTA ведёт в архив, второй — в работу</small>
            </label>
          </div>
        </div>

        <label class="block">
          <span class="label">Kicker</span>
          <input
            bind:value={content.kicker}
            class="input-gothic"
            placeholder={$t('homeKicker')}
          />
        </label>

        <label class="block">
          <span class="label">Lead</span>
          <textarea
            bind:value={content.lead}
            class="input-gothic h-32"
            placeholder={$t('homeLead')}
          ></textarea>
        </label>

        <div class="border border-[#34251c]/10 bg-[#f8f1e7]/70 p-6 space-y-5">
          <div class="flex items-start justify-between gap-4">
            <div>
              <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Hero work</p>
              <h3 class="font-gothic text-2xl text-[#6f3b24]">Работа, которую откроет подпись</h3>
              <p class="mt-2 text-sm leading-6 text-[#5f4636] max-w-2xl">
                Выберите фигурку, соответствующую hero-фото. Подпись на главной будет вести именно на эту работу.
              </p>
            </div>
            {#if selectedHeroFigurine}
              <span class="shrink-0 px-3 py-1.5 text-[10px] uppercase tracking-[0.1em] border border-emerald-700/25 bg-emerald-50 text-emerald-800">
                Связано
              </span>
            {:else}
              <span class="shrink-0 px-3 py-1.5 text-[10px] uppercase tracking-[0.1em] border border-amber-700/30 bg-amber-50 text-amber-800">
                Не выбрано
              </span>
            {/if}
          </div>

          <label class="block">
            <span class="label">Фигурка для hero</span>
            <select
              class="input-gothic"
              value={content.heroFigurineId ?? ''}
              onchange={(e) => selectHeroFigurine((e.currentTarget as HTMLSelectElement).value)}
            >
              <option value="">Не связывать с конкретной работой</option>
              {#each figurines as fig}
                <option value={fig.id}>
                  {fig.name} · {fig.status}{fig.year ? ` · ${fig.year}` : ''}
                </option>
              {/each}
            </select>
          </label>

          {#if selectedHeroFigurine}
            <div class="selected-work">
              {#if selectedHeroFigurine.faceImageUrl}
                <img src={selectedHeroFigurine.faceImageUrl} alt={selectedHeroFigurine.name} />
              {/if}
              <div>
                <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-1">Будет открываться</p>
                <p class="font-gothic text-2xl text-[#2c1710]">{selectedHeroFigurine.name}</p>
                <p class="mt-1 text-xs uppercase tracking-[0.12em] text-[#5f4636]">
                  /figurines/{selectedHeroFigurine.id}
                </p>
              </div>
              <div class="ml-auto flex gap-2">
                <button type="button" onclick={useSelectedName} class="btn-small">Название в подпись</button>
                <button type="button" onclick={clearHeroFigurine} class="btn-small">Очистить</button>
              </div>
            </div>
          {:else}
            <p class="warning">
              Если работа не выбрана, подпись не будет вести на случайную featured-фигурку. Клик откроет архив.
            </p>
          {/if}
        </div>

        <div class="border border-[#34251c]/10 bg-[#fff9f0]/70 p-6 space-y-5">
          <div class="flex items-start justify-between gap-4">
            <div>
              <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Hero image</p>
              <h3 class="font-gothic text-2xl text-[#6f3b24]">Фото первого экрана</h3>
              <p class="mt-2 text-sm leading-6 text-[#5f4636] max-w-2xl">
                Это то же фото, которое используется на главной. Оно должно визуально соответствовать выбранной работе.
              </p>
            </div>
            <button type="button" onclick={changeBackground} disabled={isUploadingBg} class="btn-gothic disabled:opacity-40">
              {isUploadingBg ? 'Загрузка…' : 'Заменить фото'}
            </button>
          </div>

          {#if bgImage}
            <div class="hero-image-preview">
              <img src={bgImage} alt="Hero" />
            </div>
          {:else}
            <p class="warning">Hero-фото не загружено. Будет использован fallback.</p>
          {/if}
        </div>

        <div class="border border-[#34251c]/10 bg-[#f8f1e7]/70 p-6 space-y-5">
          <div>
            <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Hero caption</p>
            <h3 class="font-gothic text-2xl text-[#6f3b24]">Подпись на фото</h3>
          </div>

          <label class="block">
            <span class="label">Название в подписи</span>
            <input
              bind:value={content.heroCaptionTitle}
              class="input-gothic"
              placeholder={selectedHeroFigurine?.name || 'Простак'}
            />
          </label>

          <label class="block">
            <span class="label">Мета-строка</span>
            <input
              bind:value={content.heroCaptionMeta}
              class="input-gothic"
              placeholder={$t('homeHeroObjectMeta')}
            />
          </label>

          <label class="block">
            <span class="label">Текст ссылки</span>
            <input
              bind:value={content.heroCaptionCta}
              class="input-gothic"
              placeholder={$t('homeHeroObjectOpen')}
            />
          </label>
        </div>

        <div class="border border-[#34251c]/10 bg-[#f8f1e7]/70 p-6">
          <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-3">Preview</p>
          <p class="text-xs uppercase tracking-[0.18em] text-[#6f3b24] mb-3">
            {content.kicker?.trim() || $t('homeKicker')}
          </p>
          <h3 class="font-gothic text-5xl text-[#2c1710] leading-none mb-4">
            {$t('homeTitle')}
          </h3>
          <p class="font-serif text-xl leading-8 text-[#5f4636] max-w-2xl">
            {content.lead?.trim() || $t('homeLead')}
          </p>
          <div class="mt-6 border-l border-[#c65f3c]/50 pl-4">
            <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636]">Подпись на фото</p>
            <p class="font-gothic text-3xl italic text-[#2c1710]">
              {content.heroCaptionTitle?.trim() || selectedHeroFigurine?.name || '—'}
            </p>
            <p class="text-[10px] uppercase tracking-[0.12em] text-[#5f4636]">
              {content.heroCaptionMeta?.trim() || $t('homeHeroObjectMeta')}
            </p>
            <p class="mt-2 text-[10px] uppercase tracking-[0.12em] text-[#6f3b24]">
              {content.heroCaptionCta?.trim() || (selectedHeroFigurine ? $t('homeHeroObjectOpen') : $t('homeSecondaryCta'))} →
            </p>
          </div>
        </div>

        {#if message}
          <p class="text-xs uppercase tracking-[0.1em] text-[#5f4636]">{message}</p>
        {/if}

        <div class="flex gap-3">
          <button
            onclick={save}
            disabled={isSaving || !hasUnsaved}
            class="btn-gothic disabled:opacity-40"
          >
            {isSaving ? 'Сохранение…' : 'Сохранить главную'}
          </button>
          <button
            onclick={resetToFallbacks}
            disabled={isSaving}
            class="btn-gothic opacity-70"
          >
            Вернуть тексты по умолчанию
          </button>
        </div>
      </div>
    {/if}
  </div>
</section>

<style>
  .label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #5f4636;
    margin-bottom: 0.35rem;
    display: block;
    font-weight: 700;
  }

  .input-gothic {
    width: 100%;
    background-color: #f8f1e7;
    border: 1px solid rgba(52, 37, 28, 0.18);
    padding: 0.65rem 0.75rem;
    color: #34251c;
    outline: none;
    transition: border-color 0.2s, background-color 0.2s;
    font-size: 0.875rem;
    line-height: 1.6;
  }

  .input-gothic:focus {
    border-color: rgba(198, 95, 60, 0.5);
    background-color: #fff9f0;
  }

  textarea.input-gothic {
    resize: vertical;
  }

  .selected-work {
    display: flex;
    align-items: center;
    gap: 1rem;
    border: 1px solid rgba(52, 37, 28, 0.12);
    background: rgba(255, 249, 240, 0.72);
    padding: 0.75rem;
  }

  .selected-work img {
    width: 4.5rem;
    height: 4.5rem;
    object-fit: cover;
    border: 1px solid rgba(52, 37, 28, 0.12);
  }

  .hero-image-preview {
    position: relative;
    overflow: hidden;
    aspect-ratio: 16 / 7;
    border: 1px solid rgba(52, 37, 28, 0.12);
    background: #f8f1e7;
  }

  .hero-image-preview img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .warning {
    border-left: 1px solid rgba(198, 95, 60, 0.5);
    padding-left: 0.85rem;
    color: #5f4636;
    font-size: 0.82rem;
    line-height: 1.7;
  }

  .btn-small {
    border: 1px solid rgba(52, 37, 28, 0.18);
    background: #f8f1e7;
    color: #34251c;
    padding: 0.45rem 0.65rem;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    transition: border-color 0.2s, background-color 0.2s;
  }

  .btn-small:hover {
    border-color: rgba(198, 95, 60, 0.45);
    background: #fff9f0;
  }

  .mode-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .mode-grid label {
    display: grid;
    gap: 0.45rem;
    align-content: start;
    min-height: 7rem;
    border: 1px solid rgba(52, 37, 28, 0.14);
    background: rgba(255, 249, 240, 0.58);
    padding: 0.85rem;
    color: #5f4636;
    cursor: pointer;
    transition: border-color 0.2s, background-color 0.2s;
  }

  .mode-grid label.active {
    border-color: rgba(198, 95, 60, 0.5);
    background: rgba(255, 249, 240, 0.95);
  }

  .mode-grid input {
    width: 0.85rem;
    height: 0.85rem;
    accent-color: #6f3b24;
  }

  .mode-grid span {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: #34251c;
    font-weight: 700;
  }

  .mode-grid small {
    font-size: 0.78rem;
    line-height: 1.55;
    color: #5f4636;
  }

  @media (max-width: 780px) {
    .mode-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
