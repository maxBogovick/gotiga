<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { AuthorProfile } from '$lib/types/api';
  import { fade } from 'svelte/transition';
  import { t } from '$lib/i18n';

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
      message = $t('adminProfileSaved');
      messageType = 'ok';
    } catch (e) {
      message = $t('adminProfileError');
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
        <h2 class="text-sm tracking-[0.08em] uppercase text-[#34251c]">{$t('adminProfileHeading')}</h2>
        <p class="text-[10px] text-[#5f4636] mt-1">{$t('adminProfileDesc')}</p>
      </div>
      <button
        onclick={save}
        disabled={isSaving}
        class="btn-gothic text-xs px-5 py-2 disabled:opacity-75"
      >
        {isSaving ? $t('adminSaving') : $t('adminSave')}
      </button>
    </div>

    {#if message}
      <p class="text-xs tracking-wide {messageType === 'ok' ? 'text-emerald-700' : 'text-red-700'}" in:fade>
        {message}
      </p>
    {/if}

    {#if isLoading}
      <p class="text-xs text-[#5f4636] animate-pulse">{$t('adminLoading')}</p>
    {:else}
      <div class="space-y-5">

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div>
            <label for="profile-name" class="block text-[10px] tracking-[0.06em] uppercase text-[#5f4636] mb-1">{$t('adminProfileName')}</label>
            <input
              id="profile-name"
              bind:value={profile.name}
              type="text"
              placeholder="Ivan Masterov"
              class="admin-input w-full"
            />
          </div>
          <div>
            <label for="profile-tagline" class="block text-[10px] tracking-[0.06em] uppercase text-[#5f4636] mb-1">{$t('adminProfileTagline')}</label>
            <input
              id="profile-tagline"
              bind:value={profile.tagline}
              type="text"
              placeholder="Gothic miniature sculptor"
              class="admin-input w-full"
            />
          </div>
        </div>

        <div>
          <label for="profile-bio" class="block text-[10px] tracking-[0.06em] uppercase text-[#5f4636] mb-1">{$t('adminProfileBio')}</label>
          <textarea
            id="profile-bio"
            bind:value={profile.bio}
            rows="6"
            placeholder={$t('adminProfileBioPlaceholder')}
            class="admin-input w-full resize-y"
          ></textarea>
        </div>

        <div>
          <label for="profile-photo" class="block text-[10px] tracking-[0.06em] uppercase text-[#5f4636] mb-1">{$t('adminProfilePhoto')}</label>
          <input
            id="profile-photo"
            bind:value={profile.photoUrl}
            type="text"
            placeholder="https://... or /static/images/photo.jpg"
            class="admin-input w-full"
          />
          {#if profile.photoUrl}
            <div class="mt-2 w-20 h-24 overflow-hidden border border-[#34251c]/10">
              <img src={profile.photoUrl} alt="preview" class="w-full h-full object-cover opacity-70 grayscale" />
            </div>
          {/if}
        </div>

        <div class="border-t border-[#34251c]/10 pt-5">
          <p class="text-[10px] tracking-[0.08em] uppercase text-[#5f4636] mb-4">{$t('adminProfileSocials')}</p>
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label for="profile-instagram" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">Instagram</label>
              <div class="flex items-center">
                <span class="text-[#5f4636] text-xs mr-1">@</span>
                <input id="profile-instagram" bind:value={profile.instagram} type="text" placeholder="username" class="admin-input flex-1" />
              </div>
            </div>
            <div>
              <label for="profile-telegram" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">Telegram</label>
              <div class="flex items-center">
                <span class="text-[#5f4636] text-xs mr-1">@</span>
                <input id="profile-telegram" bind:value={profile.telegram} type="text" placeholder="username" class="admin-input flex-1" />
              </div>
            </div>
            <div>
              <label for="profile-vk" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">VK</label>
              <div class="flex items-center">
                <span class="text-[#5f4636] text-xs mr-1">{$t('adminProfileVkPath')}</span>
                <input id="profile-vk" bind:value={profile.vk} type="text" placeholder="id or short address" class="admin-input flex-1" />
              </div>
            </div>
            <div>
              <label for="profile-email" class="block text-[10px] tracking-wide text-[#5f4636] mb-1">Email</label>
              <input id="profile-email" bind:value={profile.email} type="email" placeholder="master@gotiga.art" class="admin-input w-full" />
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
    border: 1px solid rgba(198, 95, 60, 0.15);
    color: #34251c;
    padding: 0.4rem 0.6rem;
    font-size: 0.75rem;
    font-family: inherit;
    outline: none;
    transition: border-color 0.2s;
  }
  .admin-input:focus {
    border-color: rgba(198, 95, 60, 0.4);
  }
  .admin-input::placeholder {
    color: rgba(95, 70, 54, 0.78);
  }
</style>
