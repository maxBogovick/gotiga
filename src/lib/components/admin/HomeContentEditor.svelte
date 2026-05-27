<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { HomeContent } from '$lib/types/api';
  import { t } from '$lib/i18n';

  let content = $state<HomeContent>({
    title: '',
    kicker: '',
    lead: '',
  });
  let savedSnapshot = $state('');
  let isLoading = $state(true);
  let isSaving = $state(false);
  let message = $state('');

  let hasUnsaved = $derived(JSON.stringify(content) !== savedSnapshot);

  onMount(async () => {
    try {
      const loaded = await api.getHomeContent();
      content = {
        title: loaded.title ?? '',
        kicker: loaded.kicker ?? '',
        lead: loaded.lead ?? '',
      };
      savedSnapshot = JSON.stringify(content);
    } finally {
      isLoading = false;
    }
  });

  async function save() {
    isSaving = true;
    message = '';
    try {
      const payload = {
        title: content.title?.trim() || null,
        kicker: content.kicker?.trim() || null,
        lead: content.lead?.trim() || null,
      };
      await api.saveHomeContent(payload);
      content = {
        title: payload.title ?? '',
        kicker: payload.kicker ?? '',
        lead: payload.lead ?? '',
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
      title: '',
      kicker: '',
      lead: '',
    };
  }
</script>

<section class="h-full overflow-y-auto bg-[#fff9f0]/50 border border-[#34251c]/10">
  <div class="p-8 max-w-4xl">
    <div class="mb-8 flex items-start justify-between gap-6">
      <div>
        <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-2">Homepage</p>
        <h2 class="font-gothic text-3xl text-[#6f3b24]">Текст главной страницы</h2>
        <p class="mt-3 text-sm leading-7 text-[#5f4636] max-w-2xl">
          Эти поля заменяют hero-текст на главной. Если поле пустое, сайт использует перевод по умолчанию.
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
        <label class="block">
          <span class="label">Hero title / H1</span>
          <input
            bind:value={content.title}
            class="input-gothic"
            placeholder="Gotiga"
          />
        </label>

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

        <div class="border border-[#34251c]/10 bg-[#f8f1e7]/70 p-6">
          <p class="text-[10px] uppercase tracking-[0.14em] text-[#5f4636] mb-3">Preview</p>
          <p class="text-xs uppercase tracking-[0.18em] text-[#6f3b24] mb-3">
            {content.kicker?.trim() || $t('homeKicker')}
          </p>
          <h3 class="font-gothic text-5xl text-[#2c1710] leading-none mb-4">
            {content.title?.trim() || 'Gotiga'}
          </h3>
          <p class="font-serif text-xl leading-8 text-[#5f4636] max-w-2xl">
            {content.lead?.trim() || $t('homeLead')}
          </p>
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
</style>
