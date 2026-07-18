<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { resolveMediaUrl } from '$lib/api';
  import { SITE_URL } from '$lib/site';
  import { t, brandName } from '$lib/i18n';
  import { createSiteAnalytics } from '$lib/analytics';

  // Data comes from the universal load (+page.ts): real values at prerender time so
  // bots see the bio, and a fresh fetch on client-side navigation.
  let { data } = $props();
  let texts = $derived(data.texts);
  let profile = $derived(data.profile);
  let portraitUrl = $derived(resolveMediaUrl(profile?.photoUrl) ?? '');
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

  const siteAnalytics = createSiteAnalytics();

  onMount(() => {
    siteAnalytics.pageView();
    prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  });

  const socialLinks = $derived(profile ? [
    profile.instagram  ? { label: 'Instagram',  icon: 'ig', href: `https://instagram.com/${profile.instagram.replace('@','')}` }         : null,
    profile.telegram   ? { label: 'Telegram',   icon: 'tg', href: `https://t.me/${profile.telegram.replace('@','')}` }                   : null,
    profile.vk         ? { label: 'VK',         icon: 'vk', href: `https://vk.com/${profile.vk.replace('@','')}` }                      : null,
    profile.artstation ? { label: 'ArtStation', icon: 'as', href: `https://artstation.com/${profile.artstation.replace('@','')}` }       : null,
    profile.pinterest  ? { label: 'Pinterest',  icon: 'pi', href: `https://pinterest.com/${profile.pinterest.replace('@','')}` }         : null,
    profile.youtube    ? { label: 'YouTube',    icon: 'yt', href: `https://youtube.com/@${profile.youtube.replace('@','')}` }            : null,
    profile.website    ? { label: 'Website',    icon: 'ws', href: profile.website.startsWith('http') ? profile.website : `https://${profile.website}` } : null,
    profile.email      ? { label: $t('authorContactLabel'), icon: 'em', href: `mailto:${profile.email}` }                               : null,
  ].filter(Boolean) : []);

  // Person entity — lets search engines and LLMs attach a named maker to the works,
  // with sameAs pointing at the verified social profiles. Only emitted when there is
  // a real profile to describe.
  let personJsonLd = $derived(profile?.name ? JSON.stringify({
    '@context': 'https://schema.org',
    '@type': 'Person',
    name: profile.name,
    url: `${SITE_URL}/author`,
    ...(profile.tagline ? { jobTitle: profile.tagline } : {}),
    ...(profile.bio ? { description: profile.bio } : {}),
    ...(portraitUrl ? { image: portraitUrl } : {}),
    worksFor: { '@type': 'Organization', name: $brandName, url: SITE_URL },
    sameAs: socialLinks
      .map((l) => l?.href)
      .filter((h): h is string => !!h && h.startsWith('http')),
  }) : '');
</script>

<svelte:head>
  <title>About the Master — {$brandName}</title>
  <meta name="description" content={profile?.tagline ?? profile?.bio ?? 'The maker behind the gothic miniatures of Ritunia.'} />
  <meta property="og:site_name" content={$brandName} />
  <meta property="og:locale" content="en_US" />
  <meta property="og:type" content="profile" />
  <meta property="og:title" content="About the Master — {$brandName}" />
  <meta property="og:description" content={profile?.tagline ?? profile?.bio ?? 'The maker behind the gothic miniatures of Ritunia.'} />
  <meta property="og:url" content="{SITE_URL}/author" />
  {#if portraitUrl}<meta property="og:image" content={portraitUrl} />{/if}
  {#if personJsonLd}{@html `<script type="application/ld+json">${personJsonLd}<\/script>`}{/if}
  {@html `<script type="application/ld+json">${JSON.stringify({ '@context': 'https://schema.org', '@type': 'BreadcrumbList', itemListElement: [ { '@type': 'ListItem', position: 1, name: $brandName, item: SITE_URL }, { '@type': 'ListItem', position: 2, name: 'About the Master', item: `${SITE_URL}/author` } ] })}<\/script>`}
  <!-- Fonts loaded once globally in app.html -->
</svelte:head>

<div class="fixed inset-0 bg-[#f8f1e7] -z-50 overflow-hidden">
  <div class="absolute inset-0 pointer-events-none bg-noise opacity-[0.07] mix-blend-overlay"></div>
  <div class="absolute inset-0 pointer-events-none bg-[radial-gradient(circle_at_center,transparent_0%,#f8f1e7_90%)]"></div>
  <div class="absolute bottom-[-10%] right-[-5%] w-[50vw] h-[50vw] bg-[#c65f3c]/20 rounded-full blur-[100px] pointer-events-none"></div>
</div>

<div class="min-h-screen relative z-10 font-['Inter'] text-[#34251c] pb-32">
    <div class="max-w-5xl mx-auto px-6 py-12 lg:py-20">

      <nav class="mb-16" in:fade={{ duration: 1000 }}>
        <a href="/" class="group flex items-center min-h-[44px] text-[10px] tracking-[0.10em] text-[#5f4636] hover:text-[#34251c] active:text-[#34251c] transition-colors uppercase">
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
                {#if portraitUrl}
                  <div class="relative w-40 h-52 lg:w-52 lg:h-64 overflow-hidden border border-[#34251c]/15 shadow-2xl">
                    <img
                      src={portraitUrl}
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
                          class="group flex items-center gap-2 px-4 py-2 min-h-[44px] border border-[#34251c]/15 hover:border-[#34251c]/40 active:border-[#34251c]/40 text-[#5f4636] hover:text-[#34251c] active:text-[#34251c] transition-all duration-300 text-[10px] tracking-[0.06em] uppercase"
                        >
                          {#if link.icon === 'ig'}
                            <svg class="w-3 h-3 fill-current" viewBox="0 0 24 24"><path d="M12 2.163c3.204 0 3.584.012 4.85.07 3.252.148 4.771 1.691 4.919 4.919.058 1.265.069 1.645.069 4.849 0 3.205-.012 3.584-.069 4.849-.149 3.225-1.664 4.771-4.919 4.919-1.266.058-1.644.07-4.85.07-3.204 0-3.584-.012-4.849-.07-3.26-.149-4.771-1.699-4.919-4.92-.058-1.265-.07-1.644-.07-4.849 0-3.204.013-3.583.07-4.849.149-3.227 1.664-4.771 4.919-4.919 1.266-.057 1.645-.069 4.849-.069zm0-2.163c-3.259 0-3.667.014-4.947.072-4.358.2-6.78 2.618-6.98 6.98-.059 1.281-.073 1.689-.073 4.948 0 3.259.014 3.668.072 4.948.2 4.358 2.618 6.78 6.98 6.98 1.281.058 1.689.072 4.948.072 3.259 0 3.668-.014 4.948-.072 4.354-.2 6.782-2.618 6.979-6.98.059-1.28.073-1.689.073-4.948 0-3.259-.014-3.667-.072-4.947-.196-4.354-2.617-6.78-6.979-6.98-1.281-.059-1.69-.073-4.949-.073zm0 5.838c-3.403 0-6.162 2.759-6.162 6.162s2.759 6.163 6.162 6.163 6.162-2.759 6.162-6.163c0-3.403-2.759-6.162-6.162-6.162zm0 10.162c-2.209 0-4-1.79-4-4 0-2.209 1.791-4 4-4s4 1.791 4 4c0 2.21-1.791 4-4 4zm6.406-11.845c-.796 0-1.441.645-1.441 1.44s.645 1.44 1.441 1.44c.795 0 1.439-.645 1.439-1.44s-.644-1.44-1.439-1.44z"/></svg>
                          {:else if link.icon === 'tg'}
                            <svg class="w-3 h-3 fill-current" viewBox="0 0 24 24"><path d="M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z"/></svg>
                          {:else if link.icon === 'vk'}
                            <svg class="w-3 h-3 fill-current" viewBox="0 0 24 24"><path d="M15.684 0H8.316C1.592 0 0 1.592 0 8.316v7.368C0 22.408 1.592 24 8.316 24h7.368C22.408 24 24 22.408 24 15.684V8.316C24 1.592 22.391 0 15.684 0zm3.692 17.123h-1.744c-.66 0-.864-.525-2.05-1.727-1.033-1-1.49-1.135-1.744-1.135-.356 0-.458.102-.458.593v1.575c0 .424-.135.678-1.253.678-1.846 0-3.896-1.118-5.335-3.202C4.624 10.857 4.03 8.57 4.03 8.096c0-.254.102-.491.593-.491h1.744c.44 0 .61.203.78.677.864 2.49 2.303 4.675 2.896 4.675.22 0 .322-.102.322-.66V9.721c-.068-1.186-.695-1.287-.695-1.71 0-.203.17-.407.44-.407h2.744c.373 0 .508.203.508.643v3.473c0 .372.17.508.271.508.22 0 .407-.136.813-.542 1.254-1.406 2.151-3.574 2.151-3.574.119-.254.322-.491.763-.491h1.744c.525 0 .644.27.525.643-.22 1.017-2.354 4.031-2.354 4.031-.186.305-.254.44 0 .78.186.254.796.779 1.203 1.253.745.847 1.32 1.558 1.473 2.05.17.49-.085.744-.576.744z"/></svg>
                          {:else if link.icon === 'as'}
                            <svg class="w-3 h-3 fill-current" viewBox="0 0 24 24"><path d="M0 17.723l2.027-5.902C4.654 6.645 9.455 4.053 14.5 4.5c4.5.4 8.5 3.5 9.5 8-.5-5-4.5-9.5-10-10.5C8.5.9 3 4.3.7 9.5L0 11.723v6zm24 .277l-2.067 5.902C19.346 17.355 14.545 14.947 9.5 15.5 5 15.9 1 18.9 0 23.4c.5-5 4.5-9.5 10-10.5C15.5 11.9 21 15.3 23.3 20.5L24 22.277V18z"/></svg>
                          {:else if link.icon === 'pi'}
                            <svg class="w-3 h-3 fill-current" viewBox="0 0 24 24"><path d="M12 0C5.373 0 0 5.373 0 12c0 5.084 3.163 9.426 7.627 11.174-.105-.949-.2-2.405.042-3.441.218-.937 1.407-5.965 1.407-5.965s-.359-.719-.359-1.782c0-1.668.967-2.914 2.171-2.914 1.023 0 1.518.769 1.518 1.69 0 1.029-.655 2.568-.994 3.995-.283 1.194.599 2.169 1.777 2.169 2.133 0 3.772-2.249 3.772-5.495 0-2.873-2.064-4.882-5.012-4.882-3.414 0-5.418 2.561-5.418 5.207 0 1.031.397 2.138.893 2.738a.36.36 0 0 1 .083.345l-.333 1.36c-.053.22-.174.267-.402.161-1.499-.698-2.436-2.889-2.436-4.649 0-3.785 2.75-7.262 7.929-7.262 4.163 0 7.398 2.967 7.398 6.931 0 4.136-2.607 7.464-6.227 7.464-1.216 0-2.359-.632-2.75-1.378l-.748 2.853c-.271 1.043-1.002 2.35-1.492 3.146C9.57 23.812 10.763 24 12 24c6.627 0 12-5.373 12-12S18.627 0 12 0z"/></svg>
                          {:else if link.icon === 'yt'}
                            <svg class="w-3 h-3 fill-current" viewBox="0 0 24 24"><path d="M23.498 6.186a3.016 3.016 0 0 0-2.122-2.136C19.505 3.545 12 3.545 12 3.545s-7.505 0-9.377.505A3.017 3.017 0 0 0 .502 6.186C0 8.07 0 12 0 12s0 3.93.502 5.814a3.016 3.016 0 0 0 2.122 2.136c1.871.505 9.376.505 9.376.505s7.505 0 9.377-.505a3.015 3.015 0 0 0 2.122-2.136C24 15.93 24 12 24 12s0-3.93-.502-5.814zM9.545 15.568V8.432L15.818 12l-6.273 3.568z"/></svg>
                          {:else if link.icon === 'ws'}
                            <svg class="w-3 h-3 fill-current" viewBox="0 0 24 24"><path d="M12 0C5.373 0 0 5.373 0 12s5.373 12 12 12 12-5.373 12-12S18.627 0 12 0zm1 16.057v-3.5h3.5c-.55 2.013-1.822 3.5-3.5 3.5zm-4.5-3.5H12v3.5c-1.678 0-2.95-1.487-3.5-3.5zm-1-1c-.282-.96-.45-2.008-.45-3.057 0-.05.001-.098.002-.147H11v3.204H7.5zm5.5 0V8.353h3.948c.001.049.002.097.002.147 0 1.049-.168 2.097-.45 3.057H13zm3.693-4.204H13V5.057c1.82.022 3.203 1.55 3.693 3.796zm-4.693-3.8v3.8H8.307C8.797 6.607 10.18 5.079 12 5.057v-.004zM6.307 8.853H2.812a9.48 9.48 0 0 1 3.181-4.804 9.89 9.89 0 0 0-.686 4.804zm-3.5 1H6.5c-.051.63-.05 1.374 0 2H2.807a9.518 9.518 0 0 1 0-2zm.5 3H6.308c.19 1.685.551 3.232 1.025 4.441A9.48 9.48 0 0 1 3.307 12.853zm13.386 4.441c.474-1.21.835-2.756 1.025-4.441h2.975a9.48 9.48 0 0 1-4 4.441zm3.5-5.441H17.5c.051-.63.05-1.374 0-2h3.693a9.518 9.518 0 0 1 0 2zm-2.806-3h-3.694V5.057a9.89 9.89 0 0 0-.686-4.804 9.48 9.48 0 0 1 4.38 4.6z"/></svg>
                          {:else}
                            <svg class="w-3 h-3 fill-current" viewBox="0 0 24 24"><path d="M20 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 4l-8 5-8-5V6l8 5 8-5v2z"/></svg>
                          {/if}
                          {link.label}
                        </a>
                      {/if}
                    {/each}
                  </div>
                {/if}

                <!-- Quiet petition invitation — the most intimate place to ask the master -->
                <div class="mt-10 pt-6 border-t border-[#34251c]/10">
                  <p class="font-['Georgia'] italic text-[#4b3326] text-base mb-2">{$t('authorCommissionLead')}</p>
                  <a
                    href="/commission"
                    class="inline-block font-['Georgia'] italic text-[#c65f3c] hover:text-[#34251c] border-b border-[#c65f3c]/40 hover:border-[#34251c] transition-colors pb-0.5"
                  >{$t('commissionInvite')}</a>
                </div>
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
