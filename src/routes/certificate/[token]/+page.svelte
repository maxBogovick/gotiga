<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import QRCode from 'qrcode';
  import { api } from '$lib/api';
  import type { PublicCertificateDto } from '$lib/types/api';

  let token = $derived(page.params.token ?? '');
  let certificate = $state<PublicCertificateDto | null>(null);
  let loading = $state(true);
  let error = $state('');
  let verifyUrl = $state('');
  let qrDataUrl = $state('');

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString(undefined, {
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
      error = 'Certificate was not found.';
    } finally {
      loading = false;
    }
  });
</script>

<svelte:head>
  <title>Collector Certificate</title>
  <meta
    name="description"
    content="Public collector certificate verification for an authenticated Ritunia reserve."
  />
</svelte:head>

<main class="min-h-screen bg-[#fff9f0] px-5 py-10 text-[#34251c]">
  <section class="mx-auto max-w-3xl">
    <a href="/" class="text-xs uppercase tracking-[0.18em] text-[#6f3b24] hover:underline">Ritunia</a>

    {#if loading}
      <div class="mt-12 border border-[#34251c]/10 bg-white p-8">
        <p class="font-['Fraunces'] text-2xl">Checking certificate…</p>
      </div>
    {:else if error || !certificate}
      <div class="mt-12 border border-red-900/15 bg-white p-8">
        <p class="text-xs uppercase tracking-[0.18em] text-red-700">Not verified</p>
        <h1 class="mt-2 font-['Fraunces'] text-3xl">Certificate not found</h1>
        <p class="mt-3 text-sm text-[#5f4636]">
          This verification link does not match an issued collector certificate.
        </p>
      </div>
    {:else}
      <div class="mt-8 grid gap-6 md:grid-cols-[minmax(0,1fr)_260px]">
        <article class="border border-[#34251c]/10 bg-white p-8">
          <p class="text-xs uppercase tracking-[0.18em] {certificate.revoked ? 'text-red-700' : 'text-emerald-800'}">
            {certificate.revoked ? 'Revoked certificate' : 'Verified certificate'}
          </p>
          <h1 class="mt-3 font-['Fraunces'] text-4xl leading-tight">Collector Certificate</h1>
          <dl class="mt-8 space-y-5 text-sm">
            <div>
              <dt class="text-[10px] uppercase tracking-[0.16em] text-[#5f4636]/70">Certificate number</dt>
              <dd class="mt-1 font-['Fraunces'] text-2xl">{certificate.certificateNumber}</dd>
            </div>
            <div>
              <dt class="text-[10px] uppercase tracking-[0.16em] text-[#5f4636]/70">Work</dt>
              <dd class="mt-1">
                {#if certificate.figurineId}
                  <a class="text-[#6f3b24] hover:underline" href="/figurines/{certificate.figurineId}">
                    {certificate.figurineName}
                  </a>
                {:else}
                  <span class="text-[#34251c]">{certificate.figurineName}</span>
                {/if}
              </dd>
            </div>
            <div>
              <dt class="text-[10px] uppercase tracking-[0.16em] text-[#5f4636]/70">Issued</dt>
              <dd class="mt-1">{formatDate(certificate.issuedAt)}</dd>
            </div>
            <div>
              <dt class="text-[10px] uppercase tracking-[0.16em] text-[#5f4636]/70">Public status</dt>
              <dd class="mt-1">
                {certificate.revoked
                  ? 'This certificate has been revoked and should not be treated as active.'
                  : 'This certificate is active and matches an issued reserve certificate.'}
              </dd>
            </div>
          </dl>
        </article>

        <aside class="border border-[#34251c]/10 bg-white p-5">
          <p class="text-xs uppercase tracking-[0.16em] text-[#5f4636]/70">Verification QR</p>
          {#if qrDataUrl}
            <img class="mt-4 h-[220px] w-[220px]" src={qrDataUrl} alt="Certificate verification QR code" />
          {/if}
          <p class="mt-4 break-all text-xs text-[#5f4636]">{verifyUrl}</p>
        </aside>
      </div>
    {/if}
  </section>
</main>
