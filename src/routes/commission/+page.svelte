<script lang="ts">
  import { page } from '$app/state';
  import { api } from '$lib/api';
  import { t, lang, brandName } from '$lib/i18n';
  import { workName } from '$lib/workText';
  import RequestWizard, { type WizardSource } from '$lib/components/RequestWizard.svelte';
  import type { Figurine } from '$lib/types/api';

  // ?source=<id или слаг> выбирает работу, на которую должна быть похожа новая.
  let sourceId = $derived(
    (page.url.searchParams.get('source') ?? page.url.searchParams.get('sourceFigurineId') ?? '').trim(),
  );
  let work = $state<Figurine | null>(null);
  let loadedFor = $state('');

  $effect(() => {
    const id = sourceId;
    work = null;
    loadedFor = '';
    if (!id) return;
    let cancelled = false;
    api.getFigurine(id)
      .then((f) => { if (!cancelled) work = f; })
      .catch(() => { /* работа не нашлась: заявка просто без основы */ })
      .finally(() => { if (!cancelled) loadedFor = id; });
    return () => { cancelled = true; };
  });

  let source = $derived.by<WizardSource | undefined>(() => {
    if (!work) return undefined;
    const img = work.images.find((i) => i.imageType === 'face') ?? work.images[0];
    return {
      id: work.id,
      name: work.name,
      displayName: workName(work, $lang) || work.name,
      photo: img?.thumbUrl ?? img?.url ?? null,
    };
  });

  // Мастер читает основу один раз при создании, поэтому ждём ответа.
  let waiting = $derived(sourceId !== '' && loadedFor !== sourceId);
</script>

<svelte:head>
  <title>{$t('commissionTitle')} — {$brandName}</title>
  <meta name="description" content={$t('commissionSubtitle')} />
</svelte:head>

<div class="page">
  <header class="head">
    <h1>{$t('wizPageTitle')}</h1>
    <p>{$t('wizPageLead')}</p>
  </header>

  {#if !waiting}
    {#key source?.id ?? 'new'}
      <RequestWizard mode="page" {source} />
    {/key}
  {/if}
</div>

<style>
  .page {
    max-width: 46rem;
    margin: 0 auto;
    padding: 2.5rem 1rem 4rem;
  }
  .head { margin: 0 0 1.4rem; text-align: center; }
  h1 {
    margin: 0 0 0.5rem;
    font-family: Georgia, 'Fraunces', serif;
    font-size: clamp(1.8rem, 5vw, 2.4rem);
    font-weight: 500;
    color: var(--color-ink-primary, #2c1710);
  }
  p {
    margin: 0;
    color: var(--color-ink-secondary, #5a3420);
    line-height: 1.5;
  }
</style>
