<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { AuthorProfile } from '$lib/types/api';
  import { fade } from 'svelte/transition';

  let profile = $state<AuthorProfile>({
    name: '',
    tagline: null,
    bio: null,
    photoUrl: null,
    instagram: null,
    telegram: null,
    vk: null,
    email: null,
  });

  let isLoading = $state(true);
  let isSaving = $state(false);
  let message = $state('');
  let messageType = $state<'ok' | 'err'>('ok');

  onMount(async () => {
    try {
      const p = await api.getAuthorProfile();
      if (p) {
        profile = { ...profile, ...p };
      }
    } catch {
      // fresh profile, leave defaults
    } finally {
      isLoading = false;
    }
  });

  async function save() {
    isSaving = true;
    message = '';
    try {
      await api.saveAuthorProfile(profile);
      message = 'Профиль сохранён';
      messageType = 'ok';
    } catch (e) {
      message = 'Ошибка сохранения';
      messageType = 'err';
    } finally {
      isSaving = false;
      setTimeout(() => { message = ''; }, 3000);
    }
  }
</script>

<div class="h-full overflow-y-auto p-6">
  <div class="max-w-2xl mx-auto space-y-8">

    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-sm tracking-[0.3em] uppercase text-[#d4c5b0]">Профиль мастера</h2>
        <p class="text-[10px] text-[#8a7f70] mt-1">Отображается на странице /author</p>
      </div>
      <button
        onclick={save}
        disabled={isSaving}
        class="btn-gothic text-xs px-5 py-2 disabled:opacity-50"
      >
        {isSaving ? 'Сохранение...' : 'Сохранить'}
      </button>
    </div>

    {#if message}
      <p class="text-xs tracking-widest {messageType === 'ok' ? 'text-emerald-400' : 'text-red-400'}" in:fade>
        {message}
      </p>
    {/if}

    {#if isLoading}
      <p class="text-xs text-[#8a7f70] animate-pulse">Загрузка...</p>
    {:else}
      <div class="space-y-5">

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div>
            <label class="block text-[10px] tracking-[0.2em] uppercase text-[#8a7f70] mb-1">Имя *</label>
            <input
              bind:value={profile.name}
              type="text"
              placeholder="Иван Мастеров"
              class="admin-input w-full"
            />
          </div>
          <div>
            <label class="block text-[10px] tracking-[0.2em] uppercase text-[#8a7f70] mb-1">Подзаголовок</label>
            <input
              bind:value={profile.tagline}
              type="text"
              placeholder="Скульптор готических миниатюр"
              class="admin-input w-full"
            />
          </div>
        </div>

        <div>
          <label class="block text-[10px] tracking-[0.2em] uppercase text-[#8a7f70] mb-1">Биография</label>
          <textarea
            bind:value={profile.bio}
            rows="6"
            placeholder="Расскажите о себе... Используйте пустую строку для разделения абзацев."
            class="admin-input w-full resize-y"
          ></textarea>
        </div>

        <div>
          <label class="block text-[10px] tracking-[0.2em] uppercase text-[#8a7f70] mb-1">URL фотографии</label>
          <input
            bind:value={profile.photoUrl}
            type="text"
            placeholder="https://... или /static/images/photo.jpg"
            class="admin-input w-full"
          />
          {#if profile.photoUrl}
            <div class="mt-2 w-20 h-24 overflow-hidden border border-[#d4c5b0]/10">
              <img src={profile.photoUrl} alt="preview" class="w-full h-full object-cover opacity-70 grayscale" />
            </div>
          {/if}
        </div>

        <div class="border-t border-[#d4c5b0]/10 pt-5">
          <p class="text-[10px] tracking-[0.3em] uppercase text-[#8a7f70] mb-4">Социальные сети</p>
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label class="block text-[10px] tracking-widest text-[#8a7f70] mb-1">Instagram</label>
              <div class="flex items-center">
                <span class="text-[#8a7f70] text-xs mr-1">@</span>
                <input bind:value={profile.instagram} type="text" placeholder="username" class="admin-input flex-1" />
              </div>
            </div>
            <div>
              <label class="block text-[10px] tracking-widest text-[#8a7f70] mb-1">Telegram</label>
              <div class="flex items-center">
                <span class="text-[#8a7f70] text-xs mr-1">@</span>
                <input bind:value={profile.telegram} type="text" placeholder="username" class="admin-input flex-1" />
              </div>
            </div>
            <div>
              <label class="block text-[10px] tracking-widest text-[#8a7f70] mb-1">VK</label>
              <div class="flex items-center">
                <span class="text-[#8a7f70] text-xs mr-1">vk.com/</span>
                <input bind:value={profile.vk} type="text" placeholder="id или короткий адрес" class="admin-input flex-1" />
              </div>
            </div>
            <div>
              <label class="block text-[10px] tracking-widest text-[#8a7f70] mb-1">Email</label>
              <input bind:value={profile.email} type="email" placeholder="master@gotiga.art" class="admin-input w-full" />
            </div>
          </div>
        </div>

      </div>
    {/if}
  </div>
</div>

<style>
  .admin-input {
    background: transparent;
    border: 1px solid rgba(212, 197, 176, 0.15);
    color: #d4c5b0;
    padding: 0.4rem 0.6rem;
    font-size: 0.75rem;
    font-family: inherit;
    outline: none;
    transition: border-color 0.2s;
  }
  .admin-input:focus {
    border-color: rgba(212, 197, 176, 0.4);
  }
  .admin-input::placeholder {
    color: rgba(138, 127, 112, 0.4);
  }
</style>
