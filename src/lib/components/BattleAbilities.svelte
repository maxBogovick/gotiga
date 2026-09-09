<script lang="ts">
  // Способности карты — ОДИН редактор на весь дом.
  //
  // Жил внутри стола хозяина и был бы переписан заново на столе человека в
  // студии; два таких редактора — это два списка глаголов, две таблицы значков
  // и два способа завести способность, которые однажды разойдутся. Разойдутся
  // молча: сервер отбрасывает неизвестный глагол без единого слова.
  //
  // Мебель листа (медальон, поле, плашка чисел) переехала вместе с ним из
  // папки стола в общую по той же причине, по которой там живёт `BattleIcon`:
  // набор, которым рисуют оба стола, не может лежать в папке одного из них.
  //
  // Числа справа считает СЕРВЕР той же формулой, что и при сохранении:
  // браузер не знает ни одного курса. Сюда они приходят готовыми — `pointsOf`.
  import { t, lang } from '$lib/i18n';
  import BattleIcon from '$lib/components/BattleIcon.svelte';
  import SheetField from '$lib/components/sheet/SheetField.svelte';
  import Medallion from '$lib/components/sheet/Medallion.svelte';
  import StatPlate from '$lib/components/sheet/StatPlate.svelte';
  import StatCell from '$lib/components/sheet/StatCell.svelte';
  import {
    ABILITIES_MAX,
    CHANNELS,
    CHANNEL_ICON,
    CHANNEL_LABELS,
    SHAPES,
    SHAPE_LABELS,
    TRIGGERS,
    TRIGGER_LABELS,
    VERBS,
    VERB_ICON,
    VERB_LABELS,
    shapeCarriesNumber,
  } from '$lib/battles';
  import type { CardAbility } from '$lib/types/api';

  let {
    abilities = $bindable([]),
    pointsOf = () => null,
    editLang = null,
  }: {
    abilities: CardAbility[];
    /** Сколько весит способность — числом от сервера или ничего. */
    pointsOf?: (id: string) => number | null;
    /** На каком языке подписывать ленту. Пусто — язык страницы: у стола
     *  хозяина свой переключатель RU/EN, у человека его нет. */
    editLang?: 'ru' | 'en' | null;
  } = $props();

  let shownLang = $derived(editLang ?? $lang);

  function addAbility() {
    if (abilities.length >= ABILITIES_MAX) return;
    abilities = [
      ...abilities,
      {
        // Собственный id внутри карты: по нему весы кладут число к нужной строке.
        id: `a${Date.now().toString(36)}`,
        nameEn: '',
        nameRu: '',
        verb: 'damage',
        channel: 'physical',
        amount: 1,
        shape: 'one',
        radius: 1,
        range: 1,
        duration: 0,
        trigger: 'active',
        manaCost: 0,
        cooldown: 0,
        keywords: [],
      },
    ];
    // Заведённое берётся в руку сразу: его затем и заводили, а лента без этого
    // отрастила бы кружок, настройки которого лежат под другим кружком.
    abilityHeld = abilities[abilities.length - 1].id;
  }

  /**
   * Какое умение в руке.
   *
   * Держим ИМЯ строки, а не её номер — по той же причине, по которой деталь
   * рамы держат за `id`: номер меняется от всякого перемещения по списку, и
   * поднятое вверх умение выпадало бы из рук на каждый щелчок. Стёртое
   * уступает первому: пустых рук у панели с умениями не бывает.
   */
  let abilityHeld = $state<string | null>(null);
  let abilityAt = $derived.by(() => {
    if (!abilities.length) return -1;
    const at = abilities.findIndex((a) => a.id === abilityHeld);
    return at >= 0 ? at : 0;
  });
  let abilityInHand = $derived(abilityAt >= 0 ? abilities[abilityAt] : null);

  /** Чем умение подписано в ленте: своим именем, а если его нет — глаголом.
   *  Безымянных умений на карте много, и «без названия» пять раз подряд не
   *  говорит ничего, а глагол говорит всё. */
  function abilityName(ability: CardAbility): string {
    const own = (shownLang === 'en' ? ability.nameEn : ability.nameRu)?.trim();
    return own || $t(VERB_LABELS[ability.verb]);
  }

  function removeAbility(at: number) {
    abilities = abilities.filter((_, i) => i !== at);
  }

  function moveAbility(at: number, by: number) {
    const to = at + by;
    if (to < 0 || to >= abilities.length) return;
    const next = [...abilities];
    [next[at], next[to]] = [next[to], next[at]];
    abilities = next;
  }

  function keywordsInput(at: number, raw: string) {
    const list = [...abilities];
    list[at] = {
      ...list[at],
      keywords: raw
        .split(',')
        .map((k) => k.trim())
        .filter(Boolean)
        .slice(0, 4),
    };
    abilities = list;
  }

  const abilityPoints = (id: string) => pointsOf(id);
</script>

  <!-- Лента умений: по медальону на каждое, лицом глагола и своим
       именем. Настройки — ТОЛЬКО того, что в руке.

       Панель разворачивала все поля всех умений разом: четыре
       умения это стена в пол-экрана из одинаковых выпадающих
       списков, и найти в ней нужное можно было только чтением
       подряд. Дом этот приём уже знает — стол рам показывает
       список деталей и настройки одной, взятой; здесь тот же
       жест и та же мебель. -->
  <div class="flex flex-wrap items-start gap-x-5 gap-y-4">
    {#each abilities as ability (ability.id)}
      <Medallion
        icon={VERB_ICON[ability.verb]}
        caption={abilityName(ability)}
        note={abilityPoints(ability.id) != null
          ? `${abilityPoints(ability.id)?.toFixed(1)} ${$t("cardAbilPoints")}`
          : undefined}
        selected={ability.id === abilityInHand?.id}
        onclick={() => (abilityHeld = ability.id)}
      />
    {/each}
    {#if abilities.length < ABILITIES_MAX}
      <Medallion
        icon="plus"
        hollow
        caption={$t("cardAbilAdd")}
        onclick={addAbility}
      />
    {/if}
  </div>

  {#if !(abilities).length}
    <p class="mt-4 text-[11px] leading-relaxed italic text-[#8a6a55]">
      {$t("cardAbilEmpty")}
    </p>
  {:else if abilityInHand}
    {@const i = abilityAt}
    {@const ability = abilityInHand}
    <div
      class="mt-5 pt-4 border-t border-dashed border-[#34251c]/15"
    >
      <!-- Чьи это настройки — сказано вслух. Без подписи полоса
           полей под лентой читается настройками ленты целиком. -->
      <div class="flex items-baseline gap-2 mb-3">
        <span
          class="text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
          >{$t("cardAbilChosen")}</span
        >
        <span class="text-xs text-[#6f3b24]"
          >{abilityName(ability)}</span
        >
        <span class="ml-auto flex items-center gap-1">
          <button
            type="button"
            onclick={() => moveAbility(i, -1)}
            disabled={i === 0}
            title={$t("cardAbilRaise")}
            class="w-6 h-6 flex items-center justify-center border border-[#34251c]/20 text-[#5f4636] disabled:opacity-30 hover:bg-[#34251c]/5"
            ><BattleIcon name="up" /></button
          >
          <button
            type="button"
            onclick={() => moveAbility(i, 1)}
            disabled={i === abilities.length - 1}
            title={$t("cardAbilLower")}
            class="w-6 h-6 flex items-center justify-center border border-[#34251c]/20 text-[#5f4636] disabled:opacity-30 hover:bg-[#34251c]/5"
            ><BattleIcon name="down" /></button
          >
          <button
            type="button"
            onclick={() => removeAbility(i)}
            title={$t("cardAbilDrop")}
            class="w-6 h-6 flex items-center justify-center border border-[#34251c]/20 text-[#8f2f22] hover:bg-[#c65f3c]/10"
            ><BattleIcon name="trash" /></button
          >
        </span>
      </div>

      <div class="grid grid-cols-2 gap-x-4 gap-y-3">
        <SheetField label={$t("cardAbilVerb")}>
          <span class="flex items-center gap-1.5">
            <span class="flex-shrink-0 text-[#6f3b24]"
              ><BattleIcon
                name={VERB_ICON[ability.verb]}
                size={15}
              /></span
            >
            <select bind:value={abilities[i].verb}>
              {#each VERBS as verb (verb)}
                <option value={verb}
                  >{$t(VERB_LABELS[verb])}</option
                >
              {/each}
            </select>
          </span>
        </SheetField>

        <!-- Значок стоит У списка, а не вместо него: форм восемь
             и поводов восемь, медальонами это два ряда кружков на
             каждое умение. Слово в списке уже написано. -->
        <SheetField label={$t("cardAbilShape")}>
          <span class="flex items-center gap-1.5">
            <span class="flex-shrink-0 text-[#6f3b24]"
              ><BattleIcon name={ability.shape} size={15} /></span
            >
            <select bind:value={abilities[i].shape}>
              {#each SHAPES as shape (shape)}
                <option value={shape}
                  >{$t(SHAPE_LABELS[shape])}</option
                >
              {/each}
            </select>
          </span>
        </SheetField>

        <SheetField label={$t("cardAbilTrigger")}>
          <span class="flex items-center gap-1.5">
            <span class="flex-shrink-0 text-[#6f3b24]"
              ><BattleIcon
                name={ability.trigger}
                size={15}
              /></span
            >
            <select bind:value={abilities[i].trigger}>
              {#each TRIGGERS as trigger (trigger)}
                <option value={trigger}
                  >{$t(TRIGGER_LABELS[trigger])}</option
                >
              {/each}
            </select>
          </span>
        </SheetField>

        <SheetField label={$t("cardAbilKeywords")}>
          <input
            value={(ability.keywords ?? []).join(", ")}
            oninput={(e) =>
              keywordsInput(i, e.currentTarget.value)}
          />
        </SheetField>

        <!-- Числа умения. Той же плашкой, что тело карты: это
             тоже числа, которые читает только движок, и второй
             вид для них означал бы, что они другого рода. -->
        <div class="col-span-2">
          <StatPlate min="6rem">
            <StatCell
              icon="sword"
              label={$t("cardAbilAmount")}
              min={0}
              max={99}
              bind:value={abilities[i].amount}
            />
            <StatCell
              icon={ability.shape}
              label={$t("cardAbilRadius")}
              tone={shapeCarriesNumber(ability.shape)
                ? "plain"
                : "quiet"}
              readonly={!shapeCarriesNumber(ability.shape)}
              min={0}
              max={3}
              bind:value={abilities[i].radius}
            />
            <StatCell
              icon="reach"
              label={$t("cardAbilRange")}
              min={0}
              max={5}
              bind:value={abilities[i].range}
            />
            <StatCell
              icon="turnStart"
              label={$t("cardAbilDuration")}
              min={0}
              max={5}
              bind:value={abilities[i].duration}
            />
            <StatCell
              icon="drop"
              label={$t("cardAbilMana")}
              min={0}
              max={20}
              bind:value={abilities[i].manaCost}
            />
            <StatCell
              icon="once"
              label={$t("cardAbilCooldown")}
              min={0}
              max={5}
              bind:value={abilities[i].cooldown}
            />
          </StatPlate>
          <!-- Оговорки о радиусе здесь нет и не нужно: ячейка
               приглушена и не правится ровно у тех форм, которые
               числа не несут, и это видно без слов. -->
        </div>

        <!-- Канал медальонами: их четыре, и это тот же выбор
             теми же кружками, что у карты целиком в «Ударе». -->
        <div class="col-span-2">
          <span
            class="block mb-1.5 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
            >{$t("cardAbilChannel")}</span
          >
          <div class="flex items-start gap-4">
            {#each CHANNELS as channel (channel)}
              <Medallion
                icon={CHANNEL_ICON[channel]}
                caption={$t(CHANNEL_LABELS[channel])}
                size={30}
                selected={ability.channel === channel}
                onclick={() =>
                  (abilities[i].channel = channel)}
              />
            {/each}
          </div>
        </div>

        <SheetField
          label={`${$t("cardAbilName")} · RU`}
        >
          <input
            bind:value={abilities[i].nameRu}
            maxlength="60"
          />
        </SheetField>
        <SheetField
          label={`${$t("cardAbilName")} · EN`}
        >
          <input
            bind:value={abilities[i].nameEn}
            maxlength="60"
          />
        </SheetField>
      </div>
    </div>
  {/if}
