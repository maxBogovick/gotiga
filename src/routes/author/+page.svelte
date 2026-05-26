<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { api } from '$lib/api';
  import type { AuthorText, AuthorProfile } from '$lib/types/api';
  import { t } from '$lib/i18n';

  let texts = $state<AuthorText[]>([]);
  let profile = $state<AuthorProfile | null>(null);
  let isLoading = $state(true);
  let error = $state<string | null>(null);
  let prefersReducedMotion = $state(false);

  function getNoteStyle(index: number) {
    const seed = (index + 1) * 31;
    const alignments = ['ml-0 mr-auto', 'mx-auto', 'ml-auto mr-0', 'ml-[10%] mr-auto'];
    const rotation = ((seed % 7) - 3) * 0.7;
    return {
      alignment: alignments[seed % alignments.length],
      rotation: prefersReducedMotion ? '0deg' : `${rotation}deg`,
      maxWidth: ['max-w-md', 'max-w-lg', 'max-w-xl'][seed % 3],
    };
  }

  onMount(async () => {
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    prefersReducedMotion = mq.matches;

    try {
      [texts, profile] = await Promise.all([
        api.getAuthorTexts(),
        api.getAuthorProfile().catch(() => null),
      ]);
    } catch (e) {
      console.error('Failed to load author page:', e);
      error = $t('authorError');
    } finally {
      isLoading = false;
    }
  });

  const socialLinks = $derived(profile ? [
    profile.instagram ? { label: 'Instagram', icon: 'ig', href: `https://instagram.com/${profile.instagram.replace('@','')}` } : null,
    profile.telegram  ? { label: 'Telegram',  icon: 'tg', href: `https://t.me/${profile.telegram.replace('@','')}` }           : null,
    profile.vk        ? { label: 'VK',        icon: 'vk', href: `https://vk.com/${profile.vk.replace('@','')}`  }              : null,
    profile.email     ? { label: $t('authorContactLabel'), icon: 'em', href: `mailto:${profile.email}` }                       : null,
  ].filter(Boolean) : []);
</script>

<svelte:head>
  <title>About the Master — Archive</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous">
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=Fraunces:opsz,wght@9..144,500;9..144,650;9..144,750&display=swap" rel="stylesheet">
</svelte:head>

<div class="fixed inset-0 bg-[#f8f1e7] -z-50 overflow-hidden">
  <div class="absolute inset-0 pointer-events-none bg-noise opacity-[0.07] mix-blend-overlay"></div>
  <div class="absolute inset-0 pointer-events-none bg-[radial-gradient(circle_at_center,transparent_0%,#f8f1e7_90%)]"></div>
  <div class="absolute bottom-[-10%] right-[-5%] w-[50vw] h-[50vw] bg-[#c65f3c]/20 rounded-full blur-[100px] pointer-events-none"></div>
</div>

{#if isLoading}
  <div class="min-h-screen flex items-center justify-center" out:fade>
    <span class="font-['Inter'] text-[#5f4636] tracking-[0.12em] text-xs animate-pulse uppercase">{$t('authorSilence')}</span>
  </div>
{:else if error}
  <div class="min-h-screen flex flex-col items-center justify-center p-8 text-center" in:fade>
    <p class="font-['Inter'] text-[#5f4636] mb-6 tracking-wide">{$t('authorError')}</p>
    <a href="/" class="text-[#34251c] border-b border-[#34251c]/20 pb-1 text-xs tracking-wide">{$t('authorReturnLink')}</a>
  </div>
{:else}
  <div class="min-h-screen relative z-10 font-['Inter'] text-[#34251c] pb-32">
    <div class="max-w-5xl mx-auto px-6 py-12 lg:py-20">

      <nav class="mb-16" in:fade={{ duration: 1000 }}>
        <a href="/" class="group flex items-center text-[10px] tracking-[0.10em] text-[#5f4636] hover:text-[#34251c] transition-colors uppercase">
          <span class="mr-3 transition-transform group-hover:-translate-x-1">←</span> {$t('authorBack').replace('← ', '')}
        </a>
      </nav>

      <!-- Profile Card -->
      {#if profile && profile.name}
        <section
          class="mb-24 lg:mb-32"
          in:fly={{ y: 30, opacity: 0, duration: 1000, easing: cubicOut }}
        >
          <div class="relative border border-[#34251c]/10 bg-[#34251c]/[0.02] backdrop-blur-sm overflow-hidden">
            <!-- Corner ornaments -->
            <div class="absolute top-3 left-3 w-6 h-6 border-t border-l border-[#34251c]/20"></div>
            <div class="absolute bottom-3 right-3 w-6 h-6 border-b border-r border-[#34251c]/20"></div>

            <div class="p-8 lg:p-12 flex flex-col lg:flex-row gap-10 lg:gap-16 items-start">

              <!-- Portrait -->
              <div class="flex-shrink-0">
                {#if profile.photoUrl}
                  <div class="relative w-40 h-52 lg:w-52 lg:h-64 overflow-hidden border border-[#34251c]/15 shadow-2xl">
                    <img
                      src={profile.photoUrl}
                      alt={profile.name}
                      class="w-full h-full object-cover grayscale sepia opacity-80"
                    />
                    <div class="absolute inset-0 bg-[radial-gradient(circle_at_center,transparent_0%,rgba(111, 59, 36,0.5)_100%)]"></div>
                  </div>
                {:else}
                  <div class="w-40 h-52 lg:w-52 lg:h-64 border border-[#34251c]/10 bg-[#34251c]/5 flex items-center justify-center">
                    <span class="font-['Fraunces'] text-5xl text-[#34251c]/20">
                      {profile.name.charAt(0)}
                    </span>
                  </div>
                {/if}
              </div>

              <!-- Bio -->
              <div class="flex-1 min-w-0">
                <p class="text-[10px] tracking-[0.10em] text-[#5f4636] uppercase mb-3">{$t('authorCreator')}</p>
                <h1 class="font-['Fraunces'] text-4xl lg:text-5xl text-[#6f3b24] mb-4 leading-tight">
                  {profile.name}
                </h1>

                {#if profile.tagline}
                  <p class="font-['Georgia'] italic text-lg text-[#34251c]/90 mb-6 tracking-wide">
                    {profile.tagline}
                  </p>
                {/if}

                <div class="w-16 h-px bg-gradient-to-r from-[#34251c]/30 to-transparent mb-6"></div>

                {#if profile.bio}
                  <div class="font-['Georgia'] text-[#4b3326] leading-relaxed text-base lg:text-lg space-y-4 mb-8">
                    {#each profile.bio.split('\n\n') as para}
                      <p>{para}</p>
                    {/each}
                  </div>
                {/if}

                <!-- Social Links -->
                {#if socialLinks.length > 0}
                  <div class="flex flex-wrap gap-3">
                    {#each socialLinks as link}
                      {#if link}
                        <a
                          href={link.href}
                          target={link.icon !== 'em' ? '_blank' : undefined}
                          rel="noopener noreferrer"
                          class="group flex items-center gap-2 px-4 py-2 border border-[#34251c]/15 hover:border-[#34251c]/40 text-[#5f4636] hover:text-[#34251c] transition-all duration-300 text-[10px] tracking-[0.06em] uppercase"
                        >
                          {#if link.icon === 'ig'}
                            <svg class="w-3 h-3 fill-current" viewBox="0 0 24 24"><path d="M12 2.163c3.204 0 3.584.012 4.85.07 3.252.148 4.771 1.691 4.919 4.919.058 1.265.069 1.645.069 4.849 0 3.205-.012 3.584-.069 4.849-.149 3.225-1.664 4.771-4.919 4.919-1.266.058-1.644.07-4.85.07-3.204 0-3.584-.012-4.849-.07-3.26-.149-4.771-1.699-4.919-4.92-.058-1.265-.07-1.644-.07-4.849 0-3.204.013-3.583.07-4.849.149-3.227 1.664-4.771 4.919-4.919 1.266-.057 1.645-.069 4.849-.069zm0-2.163c-3.259 0-3.667.014-4.947.072-4.358.2-6.78 2.618-6.98 6.98-.059 1.281-.073 1.689-.073 4.948 0 3.259.014 3.668.072 4.948.2 4.358 2.618 6.78 6.98 6.98 1.281.058 1.689.072 4.948.072 3.259 0 3.668-.014 4.948-.072 4.354-.2 6.782-2.618 6.979-6.98.059-1.28.073-1.689.073-4.948 0-3.259-.014-3.667-.072-4.947-.196-4.354-2.617-6.78-6.979-6.98-1.281-.059-1.69-.073-4.949-.073zm0 5.838c-3.403 0-6.162 2.759-6.162 6.162s2.759 6.163 6.162 6.163 6.162-2.759 6.162-6.163c0-3.403-2.759-6.162-6.162-6.162zm0 10.162c-2.209 0-4-1.79-4-4 0-2.209 1.791-4 4-4s4 1.791 4 4c0 2.21-1.791 4-4 4zm6.406-11.845c-.796 0-1.441.645-1.441 1.44s.645 1.44 1.441 1.44c.795 0 1.439-.645 1.439-1.44s-.644-1.44-1.439-1.44z"/></svg>
                          {:else if link.icon === 'tg'}
                            <svg class="w-3 h-3 fill-current" viewBox="0 0 24 24"><path d="M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z"/></svg>
                          {:else if link.icon === 'vk'}
                            <svg class="w-3 h-3 fill-current" viewBox="0 0 24 24"><path d="M15.684 0H8.316C1.592 0 0 1.592 0 8.316v7.368C0 22.408 1.592 24 8.316 24h7.368C22.408 24 24 22.408 24 15.684V8.316C24 1.592 22.391 0 15.684 0zm3.692 17.123h-1.744c-.66 0-.864-.525-2.05-1.727-1.033-1-1.49-1.135-1.744-1.135-.356 0-.458.102-.458.593v1.575c0 .424-.135.678-1.253.678-1.846 0-3.896-1.118-5.335-3.202C4.624 10.857 4.03 8.57 4.03 8.096c0-.254.102-.491.593-.491h1.744c.44 0 .61.203.78.677.864 2.49 2.303 4.675 2.896 4.675.22 0 .322-.102.322-.66V9.721c-.068-1.186-.695-1.287-.695-1.71 0-.203.17-.407.44-.407h2.744c.373 0 .508.203.508.643v3.473c0 .372.17.508.271.508.22 0 .407-.136.813-.542 1.254-1.406 2.151-3.574 2.151-3.574.119-.254.322-.491.763-.491h1.744c.525 0 .644.27.525.643-.22 1.017-2.354 4.031-2.354 4.031-.186.305-.254.44 0 .78.186.254.796.779 1.203 1.253.745.847 1.32 1.558 1.473 2.05.17.49-.085.744-.576.744z"/></svg>
                          {:else}
                            <svg class="w-3 h-3 fill-current" viewBox="0 0 24 24"><path d="M20 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 4l-8 5-8-5V6l8 5 8-5v2z"/></svg>
                          {/if}
                          {link.label}
                        </a>
                      {/if}
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          </div>
        </section>
      {:else}
        <header class="mb-24 text-center" in:fade={{ duration: 1000 }}>
          <h1 class="font-['Fraunces'] text-5xl lg:text-7xl text-[#6f3b24] mb-6 opacity-80 drop-shadow-2xl">
            {$t('authorVoiceTitle')}
          </h1>
          <div class="w-24 h-px bg-gradient-to-r from-transparent via-[#34251c]/30 to-transparent mx-auto"></div>
        </header>
      {/if}

      <!-- Author Texts -->
      {#if texts.length > 0}
        {#if profile && profile.name}
          <div class="mb-16 text-center" in:fade={{ delay: 400 }}>
            <p class="text-[10px] tracking-[0.10em] text-[#5f4636] uppercase">{$t('authorMasterNotes')}</p>
            <div class="w-24 h-px bg-gradient-to-r from-transparent via-[#34251c]/20 to-transparent mx-auto mt-4"></div>
          </div>
        {/if}

        <div class="space-y-24 lg:space-y-32 relative" role="list">
          {#each texts as text, i}
            {@const style = getNoteStyle(i)}
            <div
              class="relative {style.alignment} {style.maxWidth} group"
              style="transform: rotate({style.rotation});"
              role="listitem"
              in:fly={{ y: 40, opacity: 0, duration: 1200, delay: i * 200, easing: cubicOut }}
            >
              <div class="relative p-8 lg:p-12 bg-[#34251c]/[0.02] border border-[#34251c]/5 backdrop-blur-[2px] shadow-2xl transition-all duration-700 group-hover:bg-[#34251c]/[0.04] group-hover:border-[#34251c]/10">
                <span class="absolute -top-4 -left-4 font-['Fraunces'] text-2xl text-[#5f4636]/30 group-hover:text-[#5f4636]/90 transition-colors">
                  {i + 1}
                </span>
                <blockquote class="relative">
                  <p class="text-lg lg:text-xl text-[#34251c] leading-relaxed italic opacity-90 font-['Georgia']">
                    «{text.content}»
                  </p>
                </blockquote>
                {#if i % 2 === 0}
                  <div class="mt-8 flex justify-end opacity-20 group-hover:opacity-70 transition-opacity">
                    <svg width="60" height="20" viewBox="0 0 60 20" fill="none"><path d="M1 18C15 -2 45 22 59 2" stroke="#34251c" stroke-width="0.5"/></svg>
                  </div>
                {/if}
              </div>
              <div class="absolute inset-0 bg-[#6f3b24]/10 blur-2xl -z-10 translate-y-4 translate-x-2 opacity-0 group-hover:opacity-100 transition-opacity duration-700"></div>
            </div>
          {/each}
        </div>
      {:else if !profile?.name}
        <div class="text-center py-20 opacity-70">
          <p class="tracking-[0.08em] uppercase text-xs">{$t('authorEmpty')}</p>
        </div>
      {/if}

      <div class="mt-40 text-center opacity-10 select-none pointer-events-none" in:fade={{ delay: 1500 }}>
        <span class="font-['Fraunces'] text-8xl text-[#34251c]">Finis.</span>
      </div>

    </div>
  </div>
{/if}

<style>
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }

  blockquote p {
    text-shadow: 0 0 20px rgba(198, 95, 60, 0);
    transition: text-shadow 0.5s ease;
  }

  .group:hover blockquote p {
    text-shadow: 0 0 15px rgba(198, 95, 60, 0.2);
  }

  :global(body) {
    background-color: #f8f1e7;
    scrollbar-color: #d8c6b1 #f8f1e7;
  }
</style>
