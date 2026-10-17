<!--
  Урон по расам на листе расы.

  Две строки галочек — «сильнее против» и «защищена от». Числа не выбирают:
  одна отметка даёт ровно `raceDamageStep` (его называет сервер), потому что
  уже +2 решает встречу двух рас до первого хода (замер — `RACE_DAMAGE_MAX`
  в `battles.rs`). Своей расы в списках нет: «сильнее против себя» в цикле рас
  ничего не значит.

  Под галочками сводная таблица: строка — кто бьёт, столбец — по кому, в
  клетке итог для пары. Строку и столбец открытой расы можно править нажатием
  на клетку; чужие строки только показаны — сохраняется одна раса за раз.
  Расы без карт в таблице не стоят: их строка ничего бы не сообщила.
-->
<script lang="ts">
  import { t, lang, localized } from "$lib/i18n";
  import type { BattleRace, RaceDamage } from "$lib/types/api";

  let {
    races,
    raceId,
    value = $bindable(),
  }: {
    races: BattleRace[];
    /** Открытая раса; `null` — новая, ещё не сохранённая. */
    raceId: string | null;
    value: RaceDamage;
  } = $props();

  /** Сколько даёт одна отметка. Число одно на справочник и приходит с ним. */
  let step = $derived(races.find((r) => r.raceDamageStep)?.raceDamageStep ?? 1);

  const nameOf = (r: BattleRace) => localized(r, "name", $lang);
  let others = $derived(races.filter((r) => r.id !== raceId));

  /** Таблица расы `r`: у открытой — то, что правится сейчас, а не сохранённое. */
  function damageOf(r: BattleRace): RaceDamage {
    if (r.id === raceId) return value;
    return r.raceDamage ?? { attack: 0, against: [], defence: 0, from: [] };
  }

  /** Итог пары: прибавка бьющего против расы цели минус защита цели от него. */
  function pair(a: BattleRace, b: BattleRace): number {
    const da = damageOf(a);
    const db = damageOf(b);
    const plus = da.against.includes(b.id) ? da.attack : 0;
    const minus = db.from.includes(a.id) ? db.defence : 0;
    return plus - minus;
  }

  function toggle(list: "against" | "from", id: string) {
    if (id === raceId) return;
    const next = value[list].includes(id)
      ? value[list].filter((x) => x !== id)
      : [...value[list], id];
    const amount = list === "against" ? "attack" : "defence";
    value = { ...value, [list]: next, [amount]: next.length ? step : 0 };
  }

  /** В таблице — расы с картами и открытая: пустая раса не участвует в бою. */
  let shown = $derived(races.filter((r) => r.cardCount > 0 || r.id === raceId));
  let hidden = $derived(races.filter((r) => r.cardCount === 0 && r.id !== raceId));

  const strongAgainst = (r: BattleRace) =>
    races.some((o) => o.id !== r.id && pair(r, o) > 0);
  const answered = (r: BattleRace) =>
    races.some((o) => o.id !== r.id && pair(o, r) > 0);

  /** Сильнее кого-то, но против неё не сильнее никто: выигрывает без ответа. */
  let unanswered = $derived(races.filter((r) => strongAgainst(r) && !answered(r)));
  /** Слабее кого-то, но сама не сильнее никого: проигрывает без ответа. */
  let helpless = $derived(races.filter((r) => answered(r) && !strongAgainst(r)));

  const cellTone = (n: number) =>
    n > 0 ? "bg-[#c65f3c]/15 text-[#8f2f22]" : n < 0 ? "bg-[#34251c]/10 text-[#34251c]" : "";
  const signed = (n: number) => (n > 0 ? `+${n}` : n < 0 ? `−${-n}` : "");
</script>

{#snippet line(list: "against" | "from", title: string, hint: string)}
  <div class="py-3 border-b border-dashed border-[#34251c]/12 last:border-b-0">
    <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
      <span class="text-[10px] uppercase tracking-[0.16em] text-[#6f3b24]">{title}</span>
      <span class="text-[11px] italic text-[#5f4636]">{hint}</span>
    </div>
    {#if others.length}
      <div class="mt-2.5 flex flex-wrap gap-x-4 gap-y-1.5">
        {#each others as r (r.id)}
          <label class="flex items-center gap-1.5 text-[12px]">
            <input
              type="checkbox"
              checked={value[list].includes(r.id)}
              onchange={() => toggle(list, r.id)}
            />
            {nameOf(r)}
          </label>
        {/each}
      </div>
    {:else}
      <p class="mt-2 text-xs italic text-[#5f4636]">{$t("adminRaceDamageAlone")}</p>
    {/if}
  </div>
{/snippet}

{#if !raceId}
  <p class="text-xs italic text-[#5f4636]">{$t("adminRaceCardsSaveFirst")}</p>
{:else}
  {@render line(
    "against",
    `${$t("adminRaceDamageAttack")} · +${step}`,
    $t("adminRaceDamageAttackHint"),
  )}
  {@render line(
    "from",
    `${$t("adminRaceDamageDefence")} · −${step}`,
    $t("adminRaceDamageDefenceHint"),
  )}

  {#if shown.length > 1}
    <div class="mt-5">
      <p class="mb-2 text-[10px] uppercase tracking-[0.16em] text-[#6f3b24]">
        {$t("adminRaceDamageTable")}
      </p>
      <div class="overflow-x-auto">
        <table class="text-[11px] border-collapse">
          <thead>
            <tr>
              <th
                class="px-2 py-1 text-left font-normal text-[9px] uppercase tracking-[0.12em] text-[#5f4636]"
                >{$t("adminRaceDamageWho")} ↓ · {$t("adminRaceDamageWhom")} →</th
              >
              {#each shown as col (col.id)}
                <th
                  class="px-2 py-1 font-normal max-w-[6rem] truncate {col.id === raceId
                    ? 'text-[#c65f3c]'
                    : 'text-[#6f3b24]'}"
                  title={nameOf(col)}>{nameOf(col)}</th
                >
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each shown as row (row.id)}
              <tr class={row.id === raceId ? "outline outline-1 outline-[#c65f3c]/40" : ""}>
                <th
                  class="px-2 py-1 text-left font-normal whitespace-nowrap {row.id === raceId
                    ? 'text-[#c65f3c]'
                    : 'text-[#6f3b24]'}">{nameOf(row)}</th
                >
                {#each shown as col (col.id)}
                  {@const n = pair(row, col)}
                  {@const label = `${nameOf(row)} → ${nameOf(col)}: ${n > 0 ? "+" : ""}${n}`}
                  {#if row.id === col.id}
                    <!-- Раса против самой себя не настраивается. -->
                    <td class="w-12 border border-[#34251c]/8 bg-[#34251c]/[0.04]" aria-hidden="true"></td>
                  {:else if row.id === raceId || col.id === raceId}
                    <!-- Строка открытой расы — «сильнее против этой», столбец —
                         «защищена от этой». -->
                    <td class="p-0 border border-[#34251c]/8">
                      <button
                        onclick={() =>
                          row.id === raceId ? toggle("against", col.id) : toggle("from", row.id)}
                        title={label}
                        aria-label={label}
                        class="w-12 h-7 tabular-nums hover:ring-1 hover:ring-[#c65f3c]/60 {cellTone(n)}"
                        >{signed(n)}</button
                      >
                    </td>
                  {:else}
                    <td
                      class="w-12 h-7 text-center tabular-nums border border-[#34251c]/8 {cellTone(n)}"
                      title={label}>{signed(n)}</td
                    >
                  {/if}
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <p class="mt-2 max-w-[70ch] text-[11px] leading-relaxed italic text-[#5f4636]">
        {$t("adminRaceDamageTableHint")}
      </p>
      {#if hidden.length}
        <p class="mt-1 text-[11px] italic text-[#5f4636]">
          {$t("adminRaceDamageHidden")}: {hidden.map(nameOf).join(", ")}
        </p>
      {/if}
    </div>
  {/if}

  {#if unanswered.length || helpless.length}
    <div class="mt-3 space-y-1 max-w-[70ch] text-[11px] leading-relaxed text-[#8f2f22]">
      {#if unanswered.length}
        <p>{$t("adminRaceDamageUnanswered")}: {unanswered.map(nameOf).join(", ")}</p>
      {/if}
      {#if helpless.length}
        <p>{$t("adminRaceDamageHelpless")}: {helpless.map(nameOf).join(", ")}</p>
      {/if}
    </div>
  {/if}
{/if}
