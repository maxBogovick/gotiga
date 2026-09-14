<script lang="ts">
  // ВРЕМЕННАЯ страница: мерит переполнение полосы свойств на НАСТОЯЩИХ картах.
  import { onMount } from 'svelte';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import { lang } from '$lib/i18n';
  import type { BattleCard as CardDto, BattleFrame } from '$lib/types/api';

  let presets = $state<{ id: string; name: string; frame: BattleFrame }[]>([]);
  let cards = $state<CardDto[]>([]);
  const SIZES = [640];

  onMount(async () => {
    lang.set('ru');
    const [mine, base, cs] = await Promise.all([
      fetch('/_frames-preview.json').then((r) => r.json()),
      fetch('/_frames-baseline.json').then((r) => r.json()),
      fetch('/_frames-cards.json').then((r) => r.json())
    ]);
    presets = [...base, ...mine];
    cards = cs;
  });
</script>

<div style="display:flex;flex-wrap:wrap;gap:14px;padding:14px;background:#f8f1e7">
  {#each presets as p (p.id)}
    {#each cards as c (c.id)}
      {#each SIZES as px}
        <div style="width:{px}px" data-label="{p.name} · {c.titleRu} · {px}">
          <div style="font:11px Georgia,serif;color:#34251c">{p.name} · {c.titleRu} · {px}</div>
          <BattleCard
            card={{ ...c, tier: 1, frameOverride: null, raceLevelFrames: null }}
            frames={[{ ...p.frame, tier: 1 }]}
            owned={true}
            transition={false}
            interactive={false}
            frameEditable={true}
            rowsEditable={true}
            wearSeed={7}
          />
        </div>
      {/each}
    {/each}
  {/each}
</div>
