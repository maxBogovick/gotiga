<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import type { BookingRules } from '$lib/types/api';

  let rules   = $state<BookingRules>({ minDays: 1, maxDays: 30, advanceDays: 0 });
  let loading = $state(true);
  let saving  = $state(false);
  let saved   = $state(false);
  let saveErr = $state('');

  onMount(async () => {
    rules = await api.getBookingRules().catch(() => ({ minDays: 1, maxDays: 30, advanceDays: 0 }));
    loading = false;
  });

  async function save() {
    saving = true; saved = false; saveErr = '';
    try {
      await api.saveBookingRules(rules);
      saved = true;
      setTimeout(() => { saved = false; }, 2500);
    } catch { saveErr = $t('adminBookingRulesSaveError'); }
    finally { saving = false; }
  }
</script>

<div class="max-w-sm px-6 py-5">
  <h3 class="font-['Fraunces'] text-lg text-[#34251c] mb-1">{$t('adminBookingRulesHeading')}</h3>
  <p class="text-xs text-[#5f4636]/70 mb-5 leading-relaxed">
    {$t('adminBookingRulesHint')}
  </p>

  {#if loading}
    <p class="text-sm text-[#5f4636]/60 italic">{$t('adminLoading')}</p>
  {:else}
    <div class="space-y-5">

      <div>
        <label for="br-min-days" class="block text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase mb-1">
          {$t('adminBookingRulesMinDays')}
        </label>
        <input id="br-min-days" type="number" min="1" max="365"
          bind:value={rules.minDays}
          class="w-full border border-[#d8c6b1] bg-[#fff9f0] px-3 py-2 text-sm text-[#34251c] focus:outline-none focus:border-[#34251c]/50 transition-colors" />
        <p class="text-[10px] text-[#5f4636]/50 mt-1">{$t('adminBookingRulesMinDaysHint')}</p>
      </div>

      <div>
        <label for="br-max-days" class="block text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase mb-1">
          {$t('adminBookingRulesMaxDays')}
        </label>
        <input id="br-max-days" type="number" min="1" max="365"
          bind:value={rules.maxDays}
          class="w-full border border-[#d8c6b1] bg-[#fff9f0] px-3 py-2 text-sm text-[#34251c] focus:outline-none focus:border-[#34251c]/50 transition-colors" />
        <p class="text-[10px] text-[#5f4636]/50 mt-1">{$t('adminBookingRulesMaxDaysHint')}</p>
      </div>

      <div>
        <label for="br-advance-days" class="block text-[10px] font-['Inter'] font-bold tracking-[0.08em] text-[#5f4636] uppercase mb-1">
          {$t('adminBookingRulesAdvanceDays')}
        </label>
        <input id="br-advance-days" type="number" min="0" max="90"
          bind:value={rules.advanceDays}
          class="w-full border border-[#d8c6b1] bg-[#fff9f0] px-3 py-2 text-sm text-[#34251c] focus:outline-none focus:border-[#34251c]/50 transition-colors" />
        <p class="text-[10px] text-[#5f4636]/50 mt-1">{$t('adminBookingRulesAdvanceDaysHint')}</p>
      </div>

      <div class="pt-2 flex items-center gap-3">
        <button onclick={save} disabled={saving}
          class="px-5 py-2 bg-[#34251c] text-[#fff9f0] text-xs font-['Inter'] uppercase tracking-wide hover:bg-[#6f3b24] transition-colors disabled:opacity-50 disabled:cursor-not-allowed">
          {saving ? $t('adminSaving') : $t('adminSave')}
        </button>
        {#if saved}
          <span class="text-xs text-green-700 font-['Inter']">{$t('adminBookingRulesSaved')}</span>
        {/if}
        {#if saveErr}
          <span class="text-xs text-red-700 font-['Inter']">{saveErr}</span>
        {/if}
      </div>
    </div>
  {/if}
</div>
