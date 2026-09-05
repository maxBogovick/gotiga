<script lang="ts">
  // Что эта карта (или раса) показывает на каждом поводе.
  //
  // Шесть строк, и ни одна не обязательна. Повод — когда надето, не что
  // надето: в списке все виды, не только помеченные этим поводом. Иначе
  // выстрел нельзя повесить на способность, не отобрав его у удара.
  //
  // Пустой выбор у способности, если на удар уже что-то надето, сыграет
  // тем ударом (`motionFor`). Это говорится вслух, иначе сюрприз в этюде.
  //
  // И внизу — чего в ящике ещё нет. Заготовки дома (сглаз, чара, секира,
  // выстрел) в списке не показываются и показываться не должны: надетым
  // считается ИМЯ записи в ящике, и повод, названный заготовкой, которую в
  // ящик не клали, молча уступил бы умолчанию — то есть выбор был бы сделан и
  // не сыгран. Но и молчать нельзя: хранитель видит короткий список и решает,
  // что сглаза в доме нет вовсе, — а он есть, одной вкладкой левее.
  import { t, lang } from '$lib/i18n';
  import {
    MOTION_OCCASIONS,
    STOCK_MOTIONS,
    motionTitle,
    parseMotionWear,
    stringifyMotionWear,
  } from '$lib/battles';
  import type { Motion, MotionOccasion, MotionWear } from '$lib/types/api';
  import type { TranslationKey } from '$lib/i18n';

  let {
    wear = null,
    motions = [],
    inherited = null,
    onChange,
    onOpenBox,
  }: {
    wear?: string | null;
    motions?: Motion[];
    inherited?: string | null;
    onChange: (raw: string | null) => void;
    /** Уводит на вкладку движений. Без него строка просто ничего не предлагает. */
    onOpenBox?: () => void;
  } = $props();

  let mine = $derived(parseMotionWear(wear));
  let kin = $derived(parseMotionWear(inherited));

  const OCCASION_KEY: Record<MotionOccasion, TranslationKey> = {
    blow: 'adminMotionsOccasionBlow',
    spell: 'adminMotionsOccasionSpell',
    mend: 'adminMotionsOccasionMend',
    arrive: 'adminMotionsOccasionArrive',
    fall: 'adminMotionsOccasionFall',
    unseen: 'adminMotionsOccasionUnseen',
  };

  function put(occasion: MotionOccasion, id: string) {
    const next: MotionWear = { ...mine };
    if (id) next[occasion] = id;
    else delete next[occasion];
    onChange(stringifyMotionWear(next));
  }

  const titleOf = (id: string | undefined) => {
    const found = id ? motions.find((m) => m.id === id) : null;
    return found ? motionTitle(found, $lang) : '';
  };

  const blowId = $derived(mine.blow || kin.blow);

  /** Заготовки, которых в ящике ещё нет. Сверяются по имени: `takeStock`
   *  кладёт копию с тем же именем, а переименованная в ящике заготовка —
   *  уже своя запись, и звать её обратно в заготовки незачем. */
  const notTaken = $derived(
    STOCK_MOTIONS.filter(
      (stock) =>
        !motions.some(
          (m) => m.nameRu === stock.nameRu || m.nameEn === stock.nameEn,
        ),
    ),
  );
</script>

<div>
  <p class="text-[10px] uppercase tracking-[0.16em] text-[#6f3b24]">
    {$t('adminMotionsWear')}
  </p>
  <details class="mt-0.5">
    <summary
      class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
      >{$t('adminBattlesHintOpen')}</summary
    >
    <p class="mt-1 text-[10px] leading-snug text-[#34251c]/55">
      {$t('adminMotionsWearNote')}
    </p>
  </details>
  <div class="mt-1.5 space-y-1">
    {#each MOTION_OCCASIONS as occasion (occasion)}
      {@const kinId = kin[occasion]}
      {@const fromKin = !mine[occasion] && kinId}
      {@const spellPlaysBlow =
        occasion === 'spell' && !mine.spell && !kin.spell && blowId}
      <label class="flex items-center gap-2 text-[11px]">
        <span class="w-24 shrink-0 text-[#34251c]/70">{$t(OCCASION_KEY[occasion])}</span>
        <select
          value={mine[occasion] ?? ''}
          onchange={(e) => put(occasion, e.currentTarget.value)}
          class="flex-1 min-w-0 border border-[#34251c]/20 bg-transparent px-1 py-0.5 text-[11px]"
        >
          <option value=""
            >{spellPlaysBlow
              ? `${$t('adminMotionsSpellFallsBack')} — ${titleOf(blowId)}`
              : fromKin
                ? `${$t('adminMotionsFromRace')} — ${titleOf(kinId)}`
                : $t('adminMotionsHouseDefault')}</option
          >
          {#each motions as motion (motion.id)}
            <option value={motion.id}
              >{motionTitle(motion, $lang)}{motion.occasion !== occasion
                ? ` · ${$t(OCCASION_KEY[motion.occasion])}`
                : ''}</option
            >
          {/each}
        </select>
      </label>
    {/each}
  </div>

  {#if notTaken.length}
    <p class="mt-2 text-[10px] leading-snug text-[#34251c]/55">
      {$t('adminMotionsWearStock')}
      <span class="text-[#34251c]/75"
        >{notTaken
          .map((s) => ($lang === 'ru' ? s.nameRu : s.nameEn))
          .join(', ')}.</span
      >
      {#if onOpenBox}
        <button
          type="button"
          onclick={onOpenBox}
          class="underline decoration-dotted hover:text-[#c65f3c]"
          >{$t('adminMotionsWearOpenBox')}</button
        >
      {/if}
    </p>
  {/if}
</div>
