<script lang="ts">
  // Вход в студию: свои рамки на верстаке, ящик, дверь к столу.
  //
  // Верстак — местный (IndexedDB), ящик — на сервере. Это две разные полки, и
  // они показаны двумя полками, а не слиты в одну: человек должен видеть, что
  // лежит только у него в браузере, а что уже отдано дому. Слитый список
  // однажды соврал бы ровно в тот день, когда браузер почистили.
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { DEFAULT_FRAMES } from '$lib/battles';
  import * as local from '$lib/studio/local';
  import type { StudioState } from '$lib/types/api';

  let studio = $state<StudioState | null>(null);
  let mine = $state<local.LocalFrame[]>([]);
  let workbench = $state(true);
  let localUsed = $state(0);
  let loading = $state(true);
  let complaint = $state<string | null>(null);

  let signedIn = $derived(authStore.isLoggedIn);

  onMount(async () => {
    workbench = await local.available();
    if (workbench) {
      await local.sweep();
      mine = await local.listFrames();
      localUsed = await local.localUsed();
    }
    const token = authStore.token;
    if (token) {
      try {
        studio = await api.getStudio(token);
      } catch (e) {
        complaint = String(e);
      }
    }
    loading = false;
  });

  /** Новая рамка начинается с домашней рамы её чина, а не с пустоты: пустая
   *  рама — это не «чистый лист», а карта без бумаги и каймы, и первое, что
   *  человек увидел бы, — что всё сломано.
   *
   *  И открывается СРАЗУ ПОЛНЫМ столом. Простой сборщик остаётся второй
   *  дверью для тех, кто её выберет, но встречать им всякого — значит прятать
   *  от человека то, ради чего он пришёл. */
  async function beginFrame(simple = false) {
    const id = crypto.randomUUID();
    // Новая рама — ПУСТАЯ и собранная из частей. Своя работа начинается с
    // чистого листа, а не с чужой рамы, которую надо разбирать: деталь берут в
    // руку из списка сбоку и дают ей картинку — на карте её пока нет, потому
    // что её пока нет и в раме.
    //
    // Врезки — те же десять процентов, которыми полный стол открывает `sliced`:
    // полоса нулевой ширины это деталь нулевого размера, и загруженной картинки
    // не было бы видно.
    const body = structuredClone(DEFAULT_FRAMES[0]);
    body.frameMode = 'sliced';
    body.cornerImage = '';
    body.sideImageH = '';
    body.sideImageV = '';
    body.cornerExtra = '';
    body.sideMidH = '';
    body.sideMidV = '';
    body.ornaments = [];
    body.insetTop = 10;
    body.insetRight = 10;
    body.insetBottom = 10;
    body.insetLeft = 10;
    await local.putFrame({
      id,
      name: $t('studioFrameUntitled'),
      body,
      updatedAt: Date.now(),
      advanced: !simple,
    });
    goto(`/studio/frames/${id}`);
  }

  /** Копия рамы. Ключи картинок общие с исходной — это не оплошность:
   *  уборщик (`sweep`) держит картинку, пока на неё ссылается хоть одна рама,
   *  поэтому копия ничего не весит, пока её не начали менять. */
  async function copyFrame(frame: local.LocalFrame) {
    const id = crypto.randomUUID();
    await local.putFrame({
      ...frame,
      id,
      name: `${frame.name} ${$t('studioCopySuffix')}`,
      updatedAt: Date.now(),
      // Копия — своя работа: в ящике дома её нет и статуса у неё нет.
      remoteId: null,
    });
    goto(`/studio/frames/${id}`);
  }

  /** Выбросить раму. С верстака — всегда; из ящика — если хозяин её ещё не
   *  видел: выложенное принадлежит уже не только автору. */
  async function dropFrame(frame: local.LocalFrame) {
    if (!confirm($t('studioDropSure').replace('{name}', frame.name))) return;
    const token = authStore.token;
    if (frame.remoteId && token) {
      await api.deleteStudioFrame(token, frame.remoteId).catch(() => {});
    }
    await local.dropFrame(frame.id);
    await local.sweep();
    mine = await local.listFrames();
    localUsed = await local.localUsed();
    if (token) studio = await api.getStudio(token).catch(() => studio);
  }

  /** Человеческий вес. «0.0 МБ» при шести килобайтах — это не число, а
   *  сообщение «ничего нет», и оно врёт: деталь лежит. */
  function weigh(bytes: number): string {
    if (bytes < 1024) return `${bytes} ${$t('studioBytes')}`;
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} ${$t('studioKb')}`;
    return `${(bytes / 1024 / 1024).toFixed(1)} ${$t('studioMb')}`;
  }
</script>

<svelte:head><title>{$t('studioTitle')}</title></svelte:head>

<div class="mx-auto max-w-4xl px-5 py-12">
  <h1 class="font-serif text-3xl text-[#34251c]">{$t('studioTitle')}</h1>
  <p class="mt-2 max-w-xl text-sm leading-relaxed text-[#6f3b24]">{$t('studioLead')}</p>

  {#if loading}
    <p class="mt-10 text-sm text-[#8a6a55]">{$t('studioLoading')}</p>
  {:else if !signedIn}
    <div class="mt-10 border border-[#d8c6b1] bg-[#fdf9f3] p-6">
      <p class="text-sm text-[#6f3b24]">{$t('studioNeedsName')}</p>
      <a href="/login" class="mt-4 inline-block border border-[#34251c]/25 px-4 py-2 text-xs uppercase tracking-[0.16em] hover:bg-[#34251c]/5">{$t('studioSignIn')}</a>
    </div>
  {:else if studio && !studio.open}
    <!-- Заперто — это дверь, а не ошибка: комната есть, сегодня закрыта. -->
    <div class="mt-10 border border-[#d8c6b1] bg-[#fdf9f3] p-6">
      <p class="text-sm text-[#6f3b24]">
        {studio.gate === 'closed' ? $t('studioShut') : $t('studioForOwners')}
      </p>
      {#if studio.gate === 'owners'}
        <a href="/battles" class="mt-4 inline-block border border-[#34251c]/25 px-4 py-2 text-xs uppercase tracking-[0.16em] hover:bg-[#34251c]/5">{$t('studioToShelf')}</a>
      {/if}
    </div>
  {:else}
    {#if !workbench}
      <!-- Молча терять работу нельзя. Про закрытый ящик браузера говорится
           прямо, до того как человек начнёт. -->
      <p class="mt-6 border-l-2 border-[#c65f3c] bg-[#fdf3ee] px-4 py-3 text-xs leading-relaxed text-[#6f3b24]">
        {$t('studioNoWorkbench')}
      </p>
    {/if}

    <div class="mt-8 flex flex-wrap items-center gap-3">
      <button
        onclick={() => beginFrame()}
        class="border border-[#34251c]/25 px-4 py-2 text-xs uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
        >{$t('studioNewFrame')}</button
      >
      <button
        onclick={() => beginFrame(true)}
        class="text-xs uppercase tracking-[0.16em] text-[#8a6a55] underline-offset-4 hover:text-[#c65f3c] hover:underline"
        >{$t('studioNewSimple')}</button
      >
      <a
        href="/studio/cards"
        class="border border-[#34251c]/25 px-4 py-2 text-xs uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
        >{$t('studioToCards')}</a
      >
      <a
        href="/studio/races"
        class="border border-[#34251c]/25 px-4 py-2 text-xs uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
        >{$t('studioToRaces')}</a
      >
      <a
        href="/studio/assets"
        class="border border-[#34251c]/25 px-4 py-2 text-xs uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
        >{$t('studioStore')}</a
      >
      <a
        href="/studio/season"
        class="border border-[#c65f3c]/40 px-4 py-2 text-xs uppercase tracking-[0.16em] text-[#c65f3c] hover:bg-[#c65f3c]/8"
        >{$t('studioToSeason')}</a
      >
      <a
        href="/studio/gallery"
        class="border border-[#34251c]/25 px-4 py-2 text-xs uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
        >{$t('studioToGallery')}</a
      >
      <a
        href="/studio/market"
        class="border border-[#34251c]/25 px-4 py-2 text-xs uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
        >{$t('studioToMarket')}</a
      >
      {#if studio}
        <span class="text-[11px] text-[#8a6a55]">
          {$t('studioBoxUsed')
            .replace('{used}', weigh(studio.box.used))
            .replace('{limit}', weigh(studio.box.limit))}
        </span>
      {/if}
      {#if workbench && localUsed > 0}
        <span class="text-[11px] text-[#8a6a55]"
          >{$t('studioLocalUsed').replace('{used}', weigh(localUsed))}</span
        >
      {/if}
    </div>

    {#if complaint}
      <p class="mt-4 text-xs text-[#c65f3c]">{complaint}</p>
    {/if}

    <!-- Верстак: то, что лежит только в этом браузере. -->
    <h2 class="mt-10 text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
      {$t('studioWorkbench')}
    </h2>
    {#if mine.length === 0}
      <p class="mt-3 text-sm text-[#8a6a55]">{$t('studioWorkbenchEmpty')}</p>
    {:else}
      <ul class="mt-3 divide-y divide-[#d8c6b1]/60 border-y border-[#d8c6b1]/60">
        {#each mine as frame (frame.id)}
          <li class="flex items-center justify-between gap-4 py-3">
            <a href="/studio/frames/{frame.id}" class="text-sm text-[#6f3b24] hover:underline"
              >{frame.name}</a
            >
            <span class="flex items-center gap-3">
              <span class="text-[10px] uppercase tracking-[0.14em] text-[#b0a08e]">
                {frame.remoteId ? $t('studioInBox') : $t('studioLocalOnly')}
              </span>
              <button
                onclick={() => copyFrame(frame)}
                class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55] hover:text-[#c65f3c]"
                >{$t('studioCopy')}</button
              >
              <button
                onclick={() => dropFrame(frame)}
                class="text-[10px] uppercase tracking-[0.14em] text-[#8f2f22]/70 hover:text-[#8f2f22]"
                >{$t('studioDrop')}</button
              >
            </span>
          </li>
        {/each}
      </ul>
    {/if}

    <!-- Ящик: то, что уже отдано дому и переживёт чистку браузера. -->
    {#if studio}
      <h2 class="mt-10 text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
        {$t('studioBox')}
      </h2>
      {#if studio.frames.length === 0}
        <p class="mt-3 text-sm text-[#8a6a55]">{$t('studioBoxEmpty')}</p>
      {:else}
        <ul class="mt-3 divide-y divide-[#d8c6b1]/60 border-y border-[#d8c6b1]/60">
          {#each studio.frames as frame (frame.id)}
            <li class="flex items-center justify-between gap-4 py-3">
              <span class="text-sm text-[#6f3b24]">{frame.name}</span>
              <span class="text-[10px] uppercase tracking-[0.14em] text-[#b0a08e]">
                <!-- «Выложена» и «допущена» — разные вещи, и путь работы виден
                     только если их назвать порознь. -->
                {#if frame.status === 'shown' && frame.admittedAt}
                  {$t('studioStatus_admitted')}
                {:else}
                  {$t(`studioStatus_${frame.status}` as never)}
                {/if}
              </span>
              {#if frame.status === 'shown' && frame.admittedAt}
                <a
                  href="/studio/season"
                  class="text-[10px] uppercase tracking-[0.14em] text-[#c65f3c] hover:underline"
                  >{$t('studioPutInSeason')}</a
                >
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  {/if}
</div>
