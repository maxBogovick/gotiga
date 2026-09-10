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
  import StudioAsk from '$lib/components/studio/StudioAsk.svelte';
  import '$lib/components/studio/studio-room.css';
  import type { StudioState } from '$lib/types/api';

  let studio = $state<StudioState | null>(null);
  let mine = $state<local.LocalFrame[]>([]);
  let workbench = $state(true);
  let localUsed = $state(0);
  let loading = $state(true);
  let complaint = $state<string | null>(null);
  /** Какую раму выбрасывают. Своим окном, а не системным: необратимое дом
   *  спрашивает сам, и спрашивает на пергаменте. */
  let dropping = $state<local.LocalFrame | null>(null);

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

<div class="studio-room">
  <div class="page" style="max-width: 62rem">
    <p class="eyebrow">
      <span>{$t('studioEyebrow')}</span>
      <span class="eyebrow-rule"></span>
      <a href="/battles">{$t('studioToShelf')}</a>
    </p>
    <h1 class="room-title">{$t('studioTitle')}</h1>
    <p class="room-lead">{$t('studioLead')}</p>

    {#if loading}
      <p class="empty">{$t('studioLoading')}</p>
    {:else if !signedIn}
      <div class="slip">
        <p class="hint" style="font-style:normal">{$t('studioNeedsName')}</p>
        <div class="doors"><a class="btn" href="/login">{$t('studioSignIn')}</a></div>
      </div>
    {:else if studio && !studio.open}
      <!-- Заперто — это дверь, а не ошибка: комната есть, сегодня закрыта. -->
      <div class="slip">
        <p class="hint" style="font-style:normal">
          {studio.gate === 'closed' ? $t('studioShut') : $t('studioForOwners')}
        </p>
        {#if studio.gate === 'owners'}
          <div class="doors"><a class="btn" href="/battles">{$t('studioToShelf')}</a></div>
        {/if}
      </div>
    {:else}
      {#if !workbench}
        <!-- Молча терять работу нельзя. Про закрытый ящик браузера говорится
             прямо, до того как человек начнёт. -->
        <p class="warn">{$t('studioNoWorkbench')}</p>
      {/if}

      <div class="doors">
        <button class="btn btn--lit" onclick={() => beginFrame()}>{$t('studioNewFrame')}</button>
        <a class="btn" href="/studio/cards">{$t('studioToCards')}</a>
        <a class="btn" href="/studio/motions">{$t('studioToMotions')}</a>
        <a class="btn" href="/studio/races">{$t('studioToRaces')}</a>
        <a class="btn" href="/studio/assets">{$t('studioStore')}</a>
      </div>

      <!-- Комнаты, где смотрят на чужое и своё меняют, — второй дверью: они не
           про работу за столом, и мешать их с «новой рамой» значит ставить
           девять одинаковых кнопок в ряд. -->
      <div class="doors">
        <a class="quiet" href="/studio/season">{$t('studioToSeason')}</a>
        <a class="quiet" href="/studio/gallery">{$t('studioToGallery')}</a>
        <a class="quiet" href="/studio/market">{$t('studioToMarket')}</a>
        <button class="quiet" onclick={() => beginFrame(true)}>{$t('studioNewSimple')}</button>
      </div>

      <p class="mark">
        {#if studio}
          {$t('studioBoxUsed')
            .replace('{used}', weigh(studio.box.used))
            .replace('{limit}', weigh(studio.box.limit))}
        {/if}
        {#if workbench && localUsed > 0}
          · {$t('studioLocalUsed').replace('{used}', weigh(localUsed))}
        {/if}
      </p>

      {#if complaint}<p class="said">{complaint}</p>{/if}

      <!-- Верстак: то, что лежит только в этом браузере. -->
      <h2 class="shelf-title">{$t('studioWorkbench')}</h2>
      {#if mine.length === 0}
        <p class="empty" style="margin-top:1rem">{$t('studioWorkbenchEmpty')}</p>
      {:else}
        <ul class="rows">
          {#each mine as frame (frame.id)}
            <li class="row">
              <a class="row-name" href="/studio/frames/{frame.id}">{frame.name}</a>
              <span class="mark" style="margin:0">
                {frame.remoteId ? $t('studioInBox') : $t('studioLocalOnly')}
              </span>
              <button class="quiet" onclick={() => copyFrame(frame)}>{$t('studioCopy')}</button>
              <button class="quiet quiet--danger" onclick={() => (dropping = frame)}
                >{$t('studioDrop')}</button
              >
            </li>
          {/each}
        </ul>
      {/if}

      <!-- Ящик: то, что уже отдано дому и переживёт чистку браузера. -->
      {#if studio}
        <h2 class="shelf-title">{$t('studioBox')}</h2>
        {#if studio.frames.length === 0}
          <p class="empty" style="margin-top:1rem">{$t('studioBoxEmpty')}</p>
        {:else}
          <ul class="rows">
            {#each studio.frames as frame (frame.id)}
              <li class="row">
                <span class="row-name">{frame.name}</span>
                <span class="mark" style="margin:0">
                  <!-- «Выложена» и «допущена» — разные вещи, и путь работы виден
                       только если их назвать порознь. -->
                  {#if frame.status === 'shown' && frame.admittedAt}
                    {$t('studioStatus_admitted')}
                  {:else}
                    {$t(`studioStatus_${frame.status}` as never)}
                  {/if}
                </span>
                {#if frame.status === 'shown' && frame.admittedAt}
                  <a class="quiet" style="color:#c65f3c" href="/studio/season"
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
</div>

{#if dropping}
  <StudioAsk
    title={$t('studioDrop')}
    lead={$t('studioDropSure').replace('{name}', dropping.name)}
    yes={$t('studioDrop')}
    danger
    onyes={() => {
      const one = dropping;
      dropping = null;
      if (one) void dropFrame(one);
    }}
    onclose={() => (dropping = null)}
  />
{/if}

<style>
  .warn {
    margin: 1.5rem 0 0;
    padding: 0.8rem 1rem;
    border-left: 2px solid #c65f3c;
    background: #fdf3ee;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.9rem;
    line-height: 1.55;
    color: #6f3b24;
  }

  .rows {
    margin: 1rem 0 0;
    padding: 0;
    list-style: none;
    border-top: 1px solid rgba(216, 198, 177, 0.6);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.5rem 1rem;
    padding: 0.7rem 0;
    border-bottom: 1px solid rgba(216, 198, 177, 0.6);
  }

  .row-name {
    flex: 1 1 12rem;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1rem;
    color: #6f3b24;
    text-decoration: none;
  }

  a.row-name:hover {
    color: #c65f3c;
  }
</style>
