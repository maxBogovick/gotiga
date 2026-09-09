<script lang="ts">
  // Стол студии: та же `FrameDesk`, что у хозяина, с другим транспортом.
  //
  // Разница ровно одна и она вся здесь: загрузка кладёт картинку в ЯЩИК
  // БРАУЗЕРА, а не на склад дома. Стол об этом не знает — ему дают функцию,
  // возвращающую адрес, который браузер умеет показать.
  //
  // Ящика нарядов у гостя нет, и прячется он не флагом, а тем, что `presets`
  // сюда не переданы вовсе.
  import { onDestroy } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { t, lang } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { DEFAULT_FRAMES, emptyBattleCard } from '$lib/battles';
  import FrameDesk from '$lib/components/studio/FrameDesk.svelte';
  import FrameBuilder from '$lib/components/studio/FrameBuilder.svelte';
  import * as local from '$lib/studio/local';
  import { toLive, toStored, rememberLive, forgetLive, hasAlpha, liveUrl } from '$lib/studio/live';
  import StudioAssetPicker from '$lib/components/studio/StudioAssetPicker.svelte';
  import type { BattleCard, BattleFrame } from '$lib/types/api';

  let id = $derived($page.params.id ?? '');

  let frames = $state<BattleFrame[]>([structuredClone(DEFAULT_FRAMES[0])]);
  let frameIndex = $state(0);
  let name = $state('');
  let ready = $state(false);
  let missing = $state(false);
  let said = $state<string | null>(null);
  let uploading = $state(false);
  let saving = $state(false);
  let remoteId = $state<string | null>(null);
  let status = $state<string>('draft');
  /** Не согласился с нынешней редакцией. Спрашивается на выкладке, а не на
   *  входе: до выкладки работу не видит никто, и спрашивать не о чем. */
  let needsAgreement = $state(true);
  let asking = $state(false);
  /** Все рамы верстака — для выбора наверху. Стол правит одну, но человек
   *  держит несколько, и уходить за другой на витрину — лишняя дорога. */
  let bench = $state<local.LocalFrame[]>([]);
  /** Простой сборщик или полный стол. Дорога односторонняя. */
  let advanced = $state(false);
  /** Пришли пальцем. Стол резьбы держится на перетаскивании, и пальцем оно не
   *  работает — не «работает хуже», а не работает вовсе. Сказать об этом надо
   *  словом и сразу: человек, который десять минут пытается сдвинуть уголок,
   *  уходит думая, что сломано у него. Смотреть и править числа с телефона
   *  можно, и стол за это не запирается — он предупреждает.
   *
   *  Спрашивается ГРУБОСТЬ УКАЗАТЕЛЯ, а не ширина: на планшете с пером ширина
   *  телефонная, а перетаскивание работает; на широком сенсорном экране
   *  наоборот. */
  let byFinger = $state(false);

  // Что стол правит на карте: деталь в руке, строка описи, панель.
  let sliceHeld = $state<{ id: string; side: string } | null>(null) as never;
  let rowHeld = $state(null) as never;
  let barPin = $state<{ x: number; y: number } | null>(null);
  let pokedAt = $state<{ x: number; y: number } | null>(null);

  // Отмена — своя, как у хозяина: перетаскивание пишет в раму сразу, и без
  // отмены одно движение не по той детали стоит человеку всех чисел.
  let history = $state<string[]>([]);
  let ahead = $state<string[]>([]);
  let stored = $state('');
  let dirty = $derived(JSON.stringify(frames[0]) !== stored);

  // Манекен: карта дома, на которой примеряют раму. Своих карт у человека на
  // этом этапе нет, и показывать раму не на чем было бы вовсе.
  let sample = $state<BattleCard>({
    ...emptyBattleCard(),
    tier: 1,
    titleEn: 'The Keeper of the Key',
    titleRu: 'Хранительница Ключа',
    effectRu: 'Вихрь Души: каждое третье заклинание создаёт копию эффекта.',
    effectEn: 'Wind of Soul: every third spell makes a copy of its effect.',
    cost: 5,
    power: 10,
  });

  let picker = $state<{ role: string; apply: (url: string) => void } | null>(null);

  /**
   * Открыть раму — и открывать её ЗАНОВО при каждой смене адреса.
   *
   * Было `onMount`, и это молча ломало переключение: SvelteKit оставляет тот же
   * компонент, когда меняется только `[id]`, — `onMount` не повторяется, и стол
   * продолжал показывать прежнюю раму под новым адресом. Правка одной рамы
   * писалась бы в другую.
   *
   * Поэтому чтение висит на `id`, а всё, что относится к прежней раме — рука,
   * отмена, её ящик, — сбрасывается вместе с ней.
   */
  let opened = $state('');
  $effect(() => {
    const which = id;
    if (!which || which === opened) return;
    opened = which;
    void open(which);
  });

  async function open(which: string) {
    ready = false;
    missing = false;
    sliceHeld = null as never;
    history = [];
    ahead = [];
    forgetLive();
    const found = await local.getFrame(which);
    if (!found) {
      missing = true;
      return;
    }
    name = found.name;
    remoteId = found.remoteId ?? null;
    advanced = found.advanced ?? false;
    frames = [await toLive(found.body)];
    stored = JSON.stringify(frames[0]);
    status = 'draft';
    ready = true;
    bench = await local.listFrames();
    const token = authStore.token;
    if (token) {
      const state = await api.getStudio(token).catch(() => null);
      needsAgreement = !state?.agreed;
      const known = state?.frames.find((f) => f.id === remoteId);
      if (known) status = known.status;
    }
  }

  onDestroy(() => forgetLive());

  $effect(() => {
    if (typeof window === 'undefined') return;
    byFinger = window.matchMedia('(pointer: coarse)').matches;
  });

  function mark() {
    history = [...history.slice(-49), JSON.stringify(frames[0])];
    ahead = [];
  }

  function stepBack() {
    const last = history.at(-1);
    if (!last) return;
    ahead = [...ahead, JSON.stringify(frames[0])];
    history = history.slice(0, -1);
    frames = [JSON.parse(last)];
  }

  function stepOn() {
    const next = ahead.at(-1);
    if (!next) return;
    history = [...history, JSON.stringify(frames[0])];
    ahead = ahead.slice(0, -1);
    frames = [JSON.parse(next)];
  }

  function flash(text: string, ms = 4000) {
    said = text;
    setTimeout(() => (said = null), ms);
  }

  /** Картинка кладётся в ящик браузера и сразу получает живой адрес: стол
   *  показывает её той же секундой, а сеть при этом не трогается вовсе. */
  async function keepLocally(file: File): Promise<string> {
    const key = local.localKey();
    await local.putBlob(key, file);
    const url = URL.createObjectURL(file);
    rememberLive(key, url);
    return url;
  }

  async function uploadArt(file: File) {
    uploading = true;
    try {
      return { url: await keepLocally(file), hasAlpha: await hasAlpha(file) };
    } finally {
      uploading = false;
    }
  }

  async function importMedia(file: File) {
    uploading = true;
    try {
      return { url: await keepLocally(file) };
    } finally {
      uploading = false;
    }
  }

  /** Работа записывается сама, без надписи «сохранено»: она просто не теряется. */
  async function keep() {
    if (!ready) return;
    await local.putFrame({
      id: opened,
      name,
      body: await toStored(frames[0]),
      updatedAt: Date.now(),
      remoteId,
      advanced,
    });
    stored = JSON.stringify(frames[0]);
  }

  /**
   * Отдать рамку на сервер — одним пакетом.
   *
   * Тело и все картинки уходят за один запрос: сервер сам меняет ключи
   * `local:…` на адреса склада и отвечает готовой рамой. Клеить тело из
   * двадцати ответов клиенту не приходится, а значит и разъезжаться нечему.
   *
   * `publish` — та же дорога, но с просьбой показать хозяину: выкладка это не
   * второе сохранение, а второе НАМЕРЕНИЕ, и делать её отдельным путём значило
   * бы иметь два способа отправить одну и ту же работу.
   */
  async function send(publish: boolean) {
    const token = authStore.token;
    if (!token) return;
    if (publish && needsAgreement) {
      asking = true;
      return;
    }
    saving = true;
    try {
      const body = (await toStored(frames[0])) as BattleFrame;
      const files = new Map<string, Blob>();
      for (const key of collectLocal(body)) {
        const blob = await local.getBlob(key);
        if (blob) files.set(key, blob);
      }
      const saved = await api.packageStudioFrame(token, {
        id: remoteId,
        name,
        body,
        files,
        publish,
        lang: $lang,
      });
      remoteId = saved.id;
      status = saved.status;
      await local.putFrame({
        id: opened,
        name,
        body: saved.body,
        updatedAt: Date.now(),
        remoteId,
        advanced,
      });
      frames = [await toLive(saved.body)];
      stored = JSON.stringify(frames[0]);
      flash(publish ? $t('studioPublishedDone') : $t('studioPutInBoxDone'));
    } catch (e) {
      // «Уже на людях» — не поломка, а ответ по делу: работа отдана, и второй
      // раз её не отдают. Всё прочее показывается как есть.
      flash(/alreadyOut/.test(String(e)) ? $t('studioAlreadyOut') : String(e), 8000);
    } finally {
      saving = false;
    }
  }

  /** Согласиться и сразу выложить: человек нажал «выложить», а не «согласиться». */
  async function agreeAndPublish() {
    const token = authStore.token;
    if (!token) return;
    try {
      await api.acceptStudioAgreement(token);
      needsAgreement = false;
      asking = false;
      await send(true);
    } catch (e) {
      flash(String(e), 8000);
    }
  }

  function collectLocal(value: unknown, found = new Set<string>()): Set<string> {
    if (typeof value === 'string') {
      if (local.isLocal(value)) found.add(value);
    } else if (Array.isArray(value)) {
      for (const one of value) collectLocal(one, found);
    } else if (value && typeof value === 'object') {
      for (const one of Object.values(value)) collectLocal(one, found);
    }
    return found;
  }

  /** Уйти на полный стол — навсегда для этой рамы. Спрашивается один раз,
   *  потому что обратной дороги нет и молча её не должно быть. */
  async function goAdvanced() {
    advanced = true;
    await keep();
  }

  async function rename(next: string) {
    name = next;
    await keep();
  }

  /** Новая рама — прямо со стола: чаще всего следующую заводят сразу после
   *  того, как кончили с этой, и уходить за ней на витрину незачем. */
  async function beginFrame() {
    const fresh = crypto.randomUUID();
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
      id: fresh,
      name: $t('studioFrameUntitled'),
      body,
      updatedAt: Date.now(),
      advanced: true,
    });
    goto(`/studio/frames/${fresh}`);
  }

  /** Копия. Ключи картинок общие с исходной — уборщик держит картинку, пока на
   *  неё ссылается хоть одна рама, поэтому копия ничего не весит до правки. */
  async function copyFrame() {
    await keep();
    const fresh = crypto.randomUUID();
    await local.putFrame({
      id: fresh,
      name: `${name} ${$t('studioCopySuffix')}`,
      body: await toStored(frames[0]),
      updatedAt: Date.now(),
      remoteId: null,
      advanced,
    });
    goto(`/studio/frames/${fresh}`);
  }

  /** Выбросить. Из ящика — только пока хозяин её не видел. */
  async function dropFrame() {
    if (!confirm($t('studioDropSure').replace('{name}', name))) return;
    const token = authStore.token;
    if (remoteId && token) await api.deleteStudioFrame(token, remoteId).catch(() => {});
    await local.dropFrame(opened);
    await local.sweep();
    goto('/studio');
  }
</script>

<svelte:head><title>{name || $t('studioTitle')}</title></svelte:head>

{#if missing}
  <div class="mx-auto max-w-2xl px-5 py-16">
    <p class="text-sm text-[#6f3b24]">{$t('studioFrameGone')}</p>
    <a href="/studio" class="mt-4 inline-block text-xs uppercase tracking-[0.16em] text-[#c65f3c] hover:underline"
      >{$t('studioBack')}</a
    >
  </div>
{:else if ready}
  <div class="flex h-[100dvh] flex-col bg-[#f8f1e7]">
    {#if byFinger}
      <p class="border-b border-[#d8c6b1] bg-[#fdf3e6] px-4 py-2 text-[11px] leading-relaxed text-[#6f3b24]">
        {$t('studioByFinger')}
      </p>
    {/if}
    <header class="flex flex-wrap items-center gap-3 border-b border-[#d8c6b1] px-4 py-2">
      <a href="/studio" class="text-xs uppercase tracking-[0.16em] text-[#8a6a55] hover:text-[#c65f3c]"
        >← {$t('studioBack')}</a
      >
      <a href="/studio/assets" class="text-xs uppercase tracking-[0.16em] text-[#8a6a55] hover:text-[#c65f3c]"
        >{$t('studioStore')}</a
      >
      <input
        value={name}
        oninput={(e) => rename(e.currentTarget.value)}
        maxlength="60"
        class="w-56 border border-[#34251c]/15 bg-transparent px-2 py-1 text-sm outline-none focus:border-[#34251c]/35"
      />
      <!-- Выбор рамы. Обычный список, а не своё изобретение: он открывается с
           клавиатуры, ищется набором и знаком человеку до того, как он сюда
           зашёл. -->
      {#if bench.length > 1}
        <select
          value={id}
          onchange={(e) => goto(`/studio/frames/${e.currentTarget.value}`)}
          aria-label={$t('studioSwitch')}
          class="max-w-[16rem] border border-[#34251c]/15 bg-transparent px-2 py-1 text-xs outline-none focus:border-[#34251c]/35"
        >
          {#each bench as one (one.id)}
            <option value={one.id}>{one.name}</option>
          {/each}
        </select>
      {/if}
      <button
        onclick={beginFrame}
        class="border border-[#34251c]/20 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
        >{$t('studioNewHere')}</button
      >
      <button
        onclick={copyFrame}
        class="border border-[#34251c]/20 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
        >{$t('studioCopy')}</button
      >
      <button
        onclick={dropFrame}
        class="px-2 py-1.5 text-[10px] uppercase tracking-[0.16em] text-[#8f2f22]/70 hover:text-[#8f2f22]"
        >{$t('studioDrop')}</button
      >
      <button
        onclick={keep}
        class="border border-[#34251c]/20 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
        >{$t('studioKeep')}</button
      >
      <button
        onclick={() => send(false)}
        disabled={saving || status !== 'draft'}
        class="border border-[#34251c]/25 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] hover:bg-[#34251c]/5 disabled:opacity-40"
        >{saving ? $t('studioSaving') : $t('studioPutInBox')}</button
      >
      <button
        onclick={() => send(true)}
        disabled={saving || status !== 'draft'}
        class="border border-[#c65f3c]/50 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] text-[#c65f3c] hover:bg-[#c65f3c]/8 disabled:opacity-40"
        >{$t('studioPublish')}</button
      >
      {#if status !== 'draft'}
        <span class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]"
          >{$t(`studioStatus_${status}` as never)}</span
        >
      {/if}
      {#if dirty}<span class="text-[10px] uppercase tracking-[0.14em] text-[#c65f3c]">{$t('studioUnsaved')}</span>{/if}
      {#if said}<span class="text-[11px] text-[#6f3b24]">{said}</span>{/if}
    </header>

    <div class="min-h-0 flex-1 overflow-hidden">
      {#if !advanced}
        <FrameBuilder bind:frame={frames[0]} {sample} {mark} onadvanced={goAdvanced} />
      {:else}
      <FrameDesk
        bind:frames
        bind:frameIndex
        bind:sliceHeld
        bind:rowHeld
        bind:barPin
        bind:pokedAt
        bind:uploading
        {sample}
        {saving}
        {dirty}
        {history}
        {ahead}
        {mark}
        {flash}
        titleOf={(card) => card.titleRu || card.titleEn}
        moveRow={() => {}}
        {stepBack}
        {stepOn}
        pickFromStore={(role, apply) => (picker = { role, apply })}
        {uploadArt}
        {importMedia}
      />
      {/if}
    </div>
  </div>

  <!-- Соглашение спрашивается ровно в тот миг, когда работа впервые выходит
       на люди, и одной кнопкой: человек нажал «показать», а не «согласиться». -->
  {#if asking}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-[#34251c]/40 p-4">
      <div class="max-w-lg border border-[#d8c6b1] bg-[#f8f1e7] p-6">
        <h2 class="font-serif text-xl text-[#34251c]">{$t('studioAgreementTitle')}</h2>
        <p class="mt-3 text-sm leading-relaxed text-[#6f3b24]">{$t('studioAgreementBody')}</p>
        <div class="mt-5 flex flex-wrap gap-3">
          <button
            onclick={agreeAndPublish}
            class="border border-[#c65f3c]/60 px-4 py-2 text-xs uppercase tracking-[0.16em] text-[#c65f3c] hover:bg-[#c65f3c]/8"
            >{$t('studioAgreementAgree')}</button
          >
          <button
            onclick={() => (asking = false)}
            class="border border-[#34251c]/20 px-4 py-2 text-xs uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
            >{$t('studioAgreementNo')}</button
          >
        </div>
      </div>
    </div>
  {/if}

  {#if picker}
    <StudioAssetPicker
      role={picker.role}
      onpick={async (picked: string) => {
        // С верстака приходит КЛЮЧ, со склада — адрес. На карту кладётся то,
        // что браузер умеет показать, а обратно в ключ его превращает `toStored`
        // при записи: обе замены живут в одном месте (`live.ts`), и ключ в раму
        // никогда не попадает напрямую.
        const url = local.isLocal(picked) ? ((await liveUrl(picked)) ?? picked) : picked;
        picker?.apply(url);
        picker = null;
      }}
      onclose={() => (picker = null)}
    />
  {/if}
{/if}
