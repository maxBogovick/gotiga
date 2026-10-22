<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import QRCode from 'qrcode';
  import { api } from '$lib/api';
  import type { PublicCertificateDto } from '$lib/types/api';
  import { t, lang, LANG_META } from '$lib/i18n';
  import { figurineNameIn } from '$lib/workText';

  let token = $derived(page.params.token ?? '');
  let certificate = $state<PublicCertificateDto | null>(null);
  let loading = $state(true);
  let error = $state('');
  let verifyUrl = $state('');
  let qrDataUrl = $state('');

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString(LANG_META[$lang].locale, {
      day: '2-digit',
      month: 'long',
      year: 'numeric',
    });
  }

  onMount(async () => {
    verifyUrl = `${window.location.origin}/certificate/${token}`;
    try {
      certificate = await api.getPublicCertificate(token);
      qrDataUrl = await QRCode.toDataURL(verifyUrl, {
        errorCorrectionLevel: 'M',
        margin: 2,
        scale: 8,
        color: {
          dark: '#34251c',
          light: '#ffffff',
        },
      });
    } catch {
      error = 'not-found';
    } finally {
      loading = false;
    }
  });
</script>

<svelte:head>
  <title>{$t('certPageTitle')}</title>
  <meta name="description" content={$t('certPageDescription')} />
</svelte:head>

<main class="min-h-screen bg-[#fff9f0] px-5 py-10 text-[#34251c]">
  <section class="mx-auto max-w-3xl">
    <a href="/" class="text-xs uppercase tracking-[0.18em] text-[#6f3b24] hover:underline">Ritunia</a>

    {#if loading}
      <div class="mt-12 border border-[#34251c]/10 bg-white p-8">
        <p class="font-['Fraunces'] text-2xl">{$t('certChecking')}</p>
      </div>
    {:else if error || !certificate}
      <div class="mt-12 border border-red-900/15 bg-white p-8">
        <p class="text-xs uppercase tracking-[0.18em] text-red-700">{$t('certNotVerified')}</p>
        <h1 class="mt-2 font-['Fraunces'] text-3xl">{$t('certNotFound')}</h1>
        <p class="mt-3 text-sm text-[#5f4636]">
          {$t('certNotFoundText')}
        </p>
      </div>
    {:else}
      <div class="mt-8 grid gap-6 md:grid-cols-[minmax(0,1fr)_260px]">
        <article class="border border-[#34251c]/10 bg-white p-8">
          <p class="text-xs uppercase tracking-[0.18em] {certificate.revoked ? 'text-red-700' : 'text-emerald-800'}">
            {certificate.revoked ? $t('certRevoked') : $t('certVerified')}
          </p>
          <h1 class="mt-3 font-['Fraunces'] text-4xl leading-tight">{$t('certPageTitle')}</h1>
          <dl class="mt-8 space-y-5 text-sm">
            <div>
              <dt class="text-[10px] uppercase tracking-[0.16em] text-[#5f4636]/70">{$t('certNumber')}</dt>
              <dd class="mt-1 font-['Fraunces'] text-2xl">{certificate.certificateNumber}</dd>
            </div>
            <div>
              <dt class="text-[10px] uppercase tracking-[0.16em] text-[#5f4636]/70">{$t('certWork')}</dt>
              <dd class="mt-1">
                {#if certificate.figurineId}
                  <a class="text-[#6f3b24] hover:underline" href="/figurines/{certificate.figurineId}">
                    {figurineNameIn(certificate, $lang)}
                  </a>
                {:else}
                  <span class="text-[#34251c]">{figurineNameIn(certificate, $lang)}</span>
                {/if}
              </dd>
            </div>
            <div>
              <dt class="text-[10px] uppercase tracking-[0.16em] text-[#5f4636]/70">{$t('certIssued')}</dt>
              <dd class="mt-1">{formatDate(certificate.issuedAt)}</dd>
            </div>
            <div>
              <dt class="text-[10px] uppercase tracking-[0.16em] text-[#5f4636]/70">{$t('certStatus')}</dt>
              <dd class="mt-1">
                {certificate.revoked
                  ? $t('certStatusRevoked')
                  : $t('certStatusActive')}
              </dd>
            </div>
          </dl>
        </article>

        <aside class="border border-[#34251c]/10 bg-white p-5">
          <p class="text-xs uppercase tracking-[0.16em] text-[#5f4636]/70">{$t('certQr')}</p>
          {#if qrDataUrl}
            <img class="mt-4 h-[220px] w-[220px]" src={qrDataUrl} alt={$t('certQrAlt')} />
          {/if}
          <p class="mt-4 break-all text-xs text-[#5f4636]">{verifyUrl}</p>
        </aside>
      </div>
    {/if}
  </section>
</main>
