<script lang="ts">
  // Стенд забега: настоящая страница `/battles/run` в рамке на подставных данных.
  // Список сценариев слева, рамка справа, сверху ширина экрана. Забеги, награды и
  // кошелёк подставные и живут в памяти рамки: база и сервер забега не трогаются.
  import { t } from '$lib/i18n';
  import { SCENARIOS } from '$lib/runStand/scenarios';

  const SIZES = [
    { id: 'phone', label: 'adminStandPhone', w: 375, h: 812 },
    { id: 'tablet', label: 'adminStandTablet', w: 768, h: 1024 },
    { id: 'desktop', label: 'adminStandDesktop', w: 1280, h: 800 },
  ] as const;

  let scenarioId = $state(SCENARIOS[2].id);
  let size = $state<(typeof SIZES)[number]['id']>('phone');
  /** Меняется при «Сбросить»: рамка открывается заново с чистым сценарием. */
  let round = $state(0);

  const current = $derived(SCENARIOS.find((s) => s.id === scenarioId)!);
  const frame = $derived(SIZES.find((s) => s.id === size)!);
  const src = $derived(`/battles/run?stand=${scenarioId}`);
</script>

<div class="grid gap-4 lg:grid-cols-[18rem_1fr]">
  <aside class="space-y-3">
    <p class="text-[11px] leading-relaxed text-[#6f5240]">{$t('adminStandNote')}</p>
    <ul class="space-y-1">
      {#each SCENARIOS as s (s.id)}
        <li>
          <button
            type="button"
            onclick={() => ((scenarioId = s.id), round++)}
            class="w-full border px-3 py-2 text-left text-sm {scenarioId === s.id
              ? 'border-[#34251c] bg-[#34251c] text-[#f8f1e7]'
              : 'border-[#34251c]/20 hover:bg-[#34251c]/5'}"
          >
            {s.title}
          </button>
        </li>
      {/each}
    </ul>
  </aside>

  <section class="min-w-0 space-y-3">
    <div class="flex flex-wrap items-center gap-2">
      <div class="flex gap-1 text-xs">
        {#each SIZES as s (s.id)}
          <button
            type="button"
            onclick={() => (size = s.id)}
            class="border px-3 py-1 {size === s.id ? 'border-[#34251c] bg-[#34251c] text-[#f8f1e7]' : 'border-[#34251c]/20'}"
            >{$t(s.label)} · {s.w}</button
          >
        {/each}
      </div>
      <button type="button" onclick={() => round++} class="border border-[#34251c]/20 px-3 py-1 text-xs">{$t('adminStandReset')}</button>
      <a href={src} target="_blank" rel="noreferrer" class="text-xs text-[#6f3b24] underline">{$t('adminStandOpen')}</a>
    </div>
    <p class="text-xs text-[#5f4636]"><b>{current.title}.</b> {current.about}</p>

    <div class="overflow-auto border border-[#34251c]/15 bg-[#34251c]/[0.03] p-3">
      {#key `${scenarioId}:${round}`}
        <iframe
          title={current.title}
          {src}
          width={frame.w}
          height={frame.h}
          class="mx-auto block border border-[#34251c]/20 bg-white"
        ></iframe>
      {/key}
    </div>
  </section>
</div>
