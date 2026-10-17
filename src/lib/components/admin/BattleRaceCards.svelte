<!--
  Карты расы на столе рас.

  Два режима одной панели:
  - `raceId` задан — карты этой расы (снять расу с отмеченных) и остальные
    карты (поставить эту расу отмеченным);
  - `raceId` пуст — карты без расы, и у каждой строки выбор расы, который
    ставится сразу: так разбирают собрание по расам за один проход.

  Расу ставит одна ручка (`POST /admin/battles/cards/race`), а не сохранение
  каждой карты: сохранение несёт карту целиком, и двадцать снимков подряд
  могли бы затереть чужую правку.
-->
<script lang="ts">
  import { t, lang, localized } from "$lib/i18n";
  import type { BattleRace } from "$lib/types/api";
  import type { BattleCard as BattleCardDto } from "$lib/types/api";

  let {
    cards,
    races,
    raceId,
    onAssign,
    onOpen,
  }: {
    cards: BattleCardDto[];
    races: BattleRace[];
    /** Раса, чьи карты показаны; `null` — разбор карт без расы. */
    raceId: string | null;
    onAssign: (cardIds: string[], raceId: string | null) => Promise<void>;
    onOpen: (card: BattleCardDto) => void;
  } = $props();

  const STATUS_TONE: Record<string, string> = {
    draft: "bg-[#b9a68f]",
    published: "bg-emerald-600",
    retired: "bg-[#b0a08e]",
    etude: "bg-[#6f3b24]",
  };
  const STATUS_KEY = {
    draft: "adminBattlesStatusDraft",
    published: "adminBattlesStatusPublished",
    retired: "adminBattlesStatusRetired",
    etude: "adminBattlesStatusEtude",
  } as const;

  let busy = $state(false);
  let query = $state("");
  let onlyFree = $state(true);
  /** Отмеченные карты — одно множество на обе части панели: карта стоит
   *  либо в одной, либо в другой, и перепутать их нечем. */
  let marked = $state<Set<string>>(new Set());
  /** Раса для отмеченных в разборе карт без расы. */
  let bulkRace = $state("");

  const titleOf = (c: BattleCardDto) => localized(c, "title", $lang) || "—";
  const raceName = (id: string | null) => {
    const r = races.find((one) => one.id === id);
    return r ? localized(r, "name", $lang) : "";
  };

  const matches = (c: BattleCardDto) => {
    const q = query.trim().toLowerCase();
    if (!q) return true;
    return `${c.titleRu} ${c.titleEn} ${c.titleUk ?? ""} ${c.figurineName ?? ""}`
      .toLowerCase()
      .includes(q);
  };

  let members = $derived(cards.filter((c) => (c.raceId ?? null) === raceId));
  let candidates = $derived(
    raceId === null
      ? []
      : cards.filter(
          (c) => c.raceId !== raceId && (!onlyFree || !c.raceId) && matches(c),
        ),
  );
  let shownMembers = $derived(raceId === null ? members.filter(matches) : members);

  /** Сколько карт расы в каком состоянии — ответ на «готова ли раса к полке». */
  let tally = $derived.by(() => {
    const out: Record<string, number> = {};
    for (const c of members) out[c.status] = (out[c.status] ?? 0) + 1;
    return Object.entries(out);
  });

  let markedMembers = $derived(shownMembers.filter((c) => marked.has(c.id)));
  let markedCandidates = $derived(candidates.filter((c) => marked.has(c.id)));

  // Сменили расу — отметки прежней ничего не значат.
  $effect(() => {
    raceId;
    marked = new Set();
    query = "";
    bulkRace = "";
  });

  function toggle(id: string) {
    const next = new Set(marked);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    marked = next;
  }

  function markAll(list: BattleCardDto[], on: boolean) {
    const next = new Set(marked);
    for (const c of list) {
      if (on) next.add(c.id);
      else next.delete(c.id);
    }
    marked = next;
  }

  async function assign(ids: string[], to: string | null) {
    if (!ids.length || busy) return;
    busy = true;
    try {
      await onAssign(ids, to);
      const next = new Set(marked);
      for (const id of ids) next.delete(id);
      marked = next;
    } finally {
      busy = false;
    }
  }

  const btn =
    "px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40 disabled:pointer-events-none";
</script>

{#snippet row(card: BattleCardDto, extra: "none" | "race" | "pick")}
  <li
    class="flex items-center gap-2.5 px-2 py-1.5 border-b border-[#34251c]/5 {marked.has(
      card.id,
    )
      ? 'bg-[#c65f3c]/[0.07]'
      : 'hover:bg-[#34251c]/[0.03]'}"
  >
    <input
      type="checkbox"
      checked={marked.has(card.id)}
      onchange={() => toggle(card.id)}
      aria-label={titleOf(card)}
      class="flex-shrink-0"
    />
    <span
      class="w-7 flex-shrink-0 bg-[#34251c]/[0.06] bg-center bg-cover border border-[#34251c]/10"
      style="aspect-ratio: 5 / 7; {card.artUrl
        ? `background-image:url('${card.artUrl}')`
        : ''}"
    ></span>
    <span class="min-w-0 flex-1">
      <button
        onclick={() => onOpen(card)}
        title={$t("adminRaceCardsOpen")}
        class="block max-w-full text-left text-[13px] leading-snug truncate hover:text-[#c65f3c] hover:underline"
        style="font-family: 'Cormorant Garamond', Georgia, serif;"
        >{titleOf(card)}</button
      >
      <span class="flex items-center gap-1.5 text-[10px] text-[#8a6a55] min-w-0">
        <span
          class="w-1.5 h-1.5 rounded-full flex-shrink-0 {STATUS_TONE[card.status] ??
            ''}"
        ></span>
        <span class="truncate"
          >{$t(STATUS_KEY[card.status as keyof typeof STATUS_KEY] ?? "adminBattlesStatusDraft")}
          · {$t("adminBattlesTier")} {card.tier}{#if card.figurineName}
            · {card.figurineName}{/if}</span
        >
      </span>
    </span>
    {#if extra === "race" && card.raceId}
      <span
        class="flex-shrink-0 max-w-[9rem] truncate px-1.5 py-0.5 text-[10px] border border-[#34251c]/15 text-[#6f3b24]"
        title={$t("adminRaceCardsNow")}>{raceName(card.raceId)}</span
      >
    {:else if extra === "pick"}
      <select
        value=""
        disabled={busy || !races.length}
        onchange={(e) => {
          const to = e.currentTarget.value;
          if (to) assign([card.id], to);
        }}
        class="flex-shrink-0 max-w-[10rem] px-1.5 py-1 text-[11px] bg-transparent border border-[#34251c]/20"
        aria-label={$t("adminRaceCardsPut")}
      >
        <option value="">{$t("adminRaceCardsPick")}</option>
        {#each races as r (r.id)}
          <option value={r.id}>{localized(r, "name", $lang)}</option>
        {/each}
      </select>
    {/if}
  </li>
{/snippet}

{#if raceId === null}
  <!-- ── Разбор карт без расы ── -->
  <div class="space-y-3">
    <div class="flex flex-wrap items-center gap-2">
      <input
        bind:value={query}
        placeholder={$t("adminBattlesSearch")}
        class="flex-1 min-w-[12rem] px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
      />
      <button class={btn} onclick={() => markAll(shownMembers, true)} disabled={!shownMembers.length}
        >{$t("adminRaceCardsAll")}</button
      >
      {#if markedMembers.length}
        <button class={btn} onclick={() => markAll(shownMembers, false)}
          >{$t("adminRaceCardsClear")}</button
        >
      {/if}
    </div>

    {#if markedMembers.length}
      <div
        class="flex flex-wrap items-center gap-2 px-3 py-2 border border-[#c65f3c]/30 bg-[#c65f3c]/[0.05]"
      >
        <span class="text-[11px] text-[#6f3b24]"
          >{$t("adminRaceCardsMarked")}: {markedMembers.length}</span
        >
        <select
          bind:value={bulkRace}
          class="px-1.5 py-1 text-[11px] bg-transparent border border-[#34251c]/20"
        >
          <option value="">{$t("adminRaceCardsPick")}</option>
          {#each races as r (r.id)}
            <option value={r.id}>{localized(r, "name", $lang)}</option>
          {/each}
        </select>
        <button
          class={btn}
          disabled={!bulkRace || busy}
          onclick={() => assign(markedMembers.map((c) => c.id), bulkRace)}
          >{$t("adminRaceCardsPut")}</button
        >
      </div>
    {/if}

    {#if !members.length}
      <p class="text-xs italic text-[#5f4636]">{$t("adminRacesWithoutEmpty")}</p>
    {:else if !shownMembers.length}
      <p class="text-xs italic text-[#5f4636]">{$t("adminRaceCardsNoMatch")}</p>
    {:else}
      <ul class="border-t border-[#34251c]/10">
        {#each shownMembers as card (card.id)}
          {@render row(card, "pick")}
        {/each}
      </ul>
    {/if}
  </div>
{:else}
  <!-- ── Карты одной расы ── -->
  <div class="space-y-6">
    <section>
      <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1 mb-2">
        <h4 class="text-[10px] uppercase tracking-[0.16em] text-[#6f3b24]">
          {$t("adminRaceCardsMembers")} · {members.length}
        </h4>
        {#if tally.length}
          <span class="text-[10px] text-[#8a6a55]">
            {#each tally as [status, n], i (status)}
              {#if i}<span class="mx-1">·</span>{/if}<span
                class="inline-block w-1.5 h-1.5 mr-1 rounded-full align-middle {STATUS_TONE[
                  status
                ] ?? ''}"
              ></span>{$t(STATUS_KEY[status as keyof typeof STATUS_KEY] ?? "adminBattlesStatusDraft")}: {n}
            {/each}
          </span>
        {/if}
        {#if members.length}
          <span class="ml-auto flex gap-2">
            {#if markedMembers.length}
              <button
                class={btn}
                disabled={busy}
                onclick={() => assign(markedMembers.map((c) => c.id), null)}
                >{$t("adminRaceCardsTake")} · {markedMembers.length}</button
              >
            {/if}
            <button
              class={btn}
              onclick={() => markAll(members, markedMembers.length < members.length)}
              >{markedMembers.length < members.length
                ? $t("adminRaceCardsAll")
                : $t("adminRaceCardsClear")}</button
            >
          </span>
        {/if}
      </div>
      {#if !members.length}
        <p class="text-xs italic text-[#5f4636]">{$t("adminRaceCardsNoneYet")}</p>
      {:else}
        <ul class="border-t border-[#34251c]/10 max-h-[26rem] overflow-y-auto">
          {#each members as card (card.id)}
            {@render row(card, "none")}
          {/each}
        </ul>
      {/if}
    </section>

    <section>
      <div class="flex flex-wrap items-center gap-2 mb-2">
        <h4 class="text-[10px] uppercase tracking-[0.16em] text-[#6f3b24] mr-auto">
          {$t("adminRaceCardsAdd")}
        </h4>
        <label
          class="flex items-center gap-1.5 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
        >
          <input type="checkbox" bind:checked={onlyFree} />
          {$t("adminRaceCardsOnlyFree")}
        </label>
      </div>
      <div class="flex flex-wrap items-center gap-2 mb-2">
        <input
          bind:value={query}
          placeholder={$t("adminBattlesSearch")}
          class="flex-1 min-w-[12rem] px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
        />
        <button
          class={btn}
          disabled={!markedCandidates.length || busy}
          onclick={() => assign(markedCandidates.map((c) => c.id), raceId)}
          >{$t("adminRaceCardsPutHere")} · {markedCandidates.length}</button
        >
      </div>
      {#if !candidates.length}
        <p class="text-xs italic text-[#5f4636]">{$t("adminRaceCardsNoMatch")}</p>
      {:else}
        <ul class="border-t border-[#34251c]/10 max-h-[26rem] overflow-y-auto">
          {#each candidates as card (card.id)}
            {@render row(card, "race")}
          {/each}
        </ul>
      {/if}
    </section>
  </div>
{/if}
