<script lang="ts">
  // Что эта карта (или раса) показывает на каждом поводе.
  //
  // Шесть строк, и ни одна из них не обязательна: не названный повод — это
  // умолчание дома, то есть ровно то, что комната делала до движка. Поэтому
  // «—» здесь не «ничего не показывать», а «как у всех», и подписано именно
  // так: пустой выбор, читаемый как «выключено», однажды заставит хранителя
  // заводить движение «обычный удар», которое у дома уже есть.
  import { t, lang } from '$lib/i18n';
  import {
    MOTION_OCCASIONS,
    motionTitle,
    parseMotionWear,
    stringifyMotionWear,
  } from '$lib/battles';
  import type { Motion, MotionOccasion, MotionWear } from '$lib/types/api';
  import type { TranslationKey } from '$lib/i18n';

  let {
    wear = null,
    motions = [],
    /** Наследуемое — наряд расы под картой. Показывается серым: хранитель
     *  должен видеть, что он перебивает, а не гадать, почему карта уже
     *  что-то делает. */
    inherited = null,
    onChange,
  }: {
    wear?: string | null;
    motions?: Motion[];
    inherited?: string | null;
    onChange: (raw: string | null) => void;
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
</script>

<div>
  <p class="text-[10px] uppercase tracking-[0.16em] text-[#6f3b24]">
    {$t('adminMotionsWear')}
  </p>
  <p class="mt-0.5 text-[10px] leading-snug text-[#34251c]/55">
    {$t('adminMotionsWearNote')}
  </p>
  <div class="mt-1.5 space-y-1">
    {#each MOTION_OCCASIONS as occasion (occasion)}
      {@const kinId = kin[occasion]}
      {@const fromKin = !mine[occasion] && kinId}
      <label class="flex items-center gap-2 text-[11px]">
        <span class="w-24 shrink-0 text-[#34251c]/70">{$t(OCCASION_KEY[occasion])}</span>
        <select
          value={mine[occasion] ?? ''}
          onchange={(e) => put(occasion, e.currentTarget.value)}
          class="flex-1 min-w-0 border border-[#34251c]/20 bg-transparent px-1 py-0.5 text-[11px]"
        >
          <option value=""
            >{fromKin
              ? `${$t('adminMotionsFromRace')} — ${titleOf(kinId)}`
              : $t('adminMotionsHouseDefault')}</option
          >
          {#each motions.filter((m) => m.occasion === occasion) as motion (motion.id)}
            <option value={motion.id}>{motionTitle(motion, $lang)}</option>
          {/each}
        </select>
      </label>
    {/each}
  </div>
</div>
