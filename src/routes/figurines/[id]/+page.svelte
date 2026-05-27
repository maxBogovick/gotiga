<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { fade } from 'svelte/transition';
  import { api } from '$lib/api';
  import type { Figurine } from '$lib/types/api';
  import DustParticles from '$lib/components/DustParticles.svelte';
  import FigurineDetailView from '$lib/components/FigurineDetailView.svelte';
  import { t } from '$lib/i18n';

  let figurine = $state<Figurine | null>(null);
  let isLoading = $state(true);
  let error = $state<string | null>(null);

  let id = $derived(page.params.id ?? '');

  onMount(async () => {
    try {
      const result = await api.getFigurine(id);
      if (!result) {
        error = $t('figurineError');
      } else {
        figurine = result;
        await new Promise((resolve) => setTimeout(resolve, 300));
      }
    } catch (e) {
      console.error('Failed to load figurine:', e);
      error = $t('figurineError');
    } finally {
      isLoading = false;
    }
  });
</script>

<svelte:head>
  <title>{figurine?.name ?? $t('zoneShowcase')} — Details</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous">
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=Fraunces:opsz,wght@9..144,500;9..144,650;9..144,750&display=swap" rel="stylesheet">
</svelte:head>

<div class="fixed inset-0 bg-cabinet-bg -z-50"></div>
<div class="fixed inset-0 pointer-events-none z-0 bg-noise opacity-[0.08] mix-blend-overlay"></div>
<div class="fixed inset-0 pointer-events-none z-0 detail-backdrop"></div>

<DustParticles />

{#if isLoading}
  <div class="min-h-screen flex flex-col items-center justify-center text-cabinet-bone" out:fade>
    <div class="relative w-16 h-16 mb-8">
      <div class="absolute inset-0 border border-cabinet-bone/20 rounded-full animate-ping"></div>
      <div class="absolute inset-0 border-t border-cabinet-bone rounded-full animate-spin"></div>
    </div>
    <span class="font-['Inter'] tracking-[0.08em] text-xs uppercase animate-pulse text-cabinet-dust">{$t('figurineExtracting')}</span>
  </div>
{:else if error}
  <div class="min-h-screen flex flex-col items-center justify-center p-8 text-center" in:fade>
    <h2 class="font-['Fraunces'] text-5xl text-cabinet-fabric mb-6">{$t('figurineError')}</h2>
    <p class="font-['Inter'] text-cabinet-bone mb-12 text-lg">{error}</p>
    <a href="/figurines" class="px-8 py-3 border border-cabinet-bone/30 text-cabinet-bone font-['Inter'] hover:bg-cabinet-wood-light transition-colors uppercase text-sm tracking-wide relative group">
      <span class="absolute inset-0 w-0 bg-cabinet-bone/5 transition-all duration-300 group-hover:w-full"></span>
      <span class="relative">{$t('figurineErrorBack')}</span>
    </a>
  </div>
{:else if figurine}
  <FigurineDetailView {figurine} {id} />
{/if}

<style>
  .bg-noise {
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  }

  .detail-backdrop {
    background:
      radial-gradient(ellipse 70% 55% at 72% 38%, rgba(198,95,60,0.07) 0%, transparent 65%),
      radial-gradient(ellipse 50% 70% at 18% 72%, rgba(201,168,117,0.06) 0%, transparent 60%),
      #f8f1e7;
  }
</style>
