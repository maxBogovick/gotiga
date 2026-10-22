<script lang="ts" module>
  export type TextLang = 'en' | 'ru' | 'uk' | 'es' | 'de' | 'it';
  export const TEXT_LANGS: TextLang[] = ['en', 'ru', 'uk', 'es', 'de', 'it'];
  const cap = (l: string) => l[0].toUpperCase() + l.slice(1);
  /** Имя поля на языке: английское — без суффикса (`title`), прочие — `titleRu`. */
  export function fieldIn(stem: string, l: TextLang): string {
    return l === 'en' ? stem : stem + cap(l);
  }
</script>

<script lang="ts">
  // Переключатель языка текстов в формах админки: EN — поле без суффикса,
  // остальные языки — поля рядом с ним. Пустое поле на сайте заменяется английским.
  import { t } from '$lib/i18n';

  let {
    value = $bindable<TextLang>('en'),
    filled = (_l: TextLang) => false,
  }: { value?: TextLang; filled?: (l: TextLang) => boolean } = $props();
</script>

<div class="flex items-center gap-1" role="group" aria-label={$t('adminFormTextLang')} title={$t('adminFormTextLangHint')}>
  <span class="text-[9px] uppercase tracking-wide text-[#5f4636]/70 mr-1">{$t('adminFormTextLang')}</span>
  {#each TEXT_LANGS as l (l)}
    <button
      type="button"
      onclick={() => (value = l)}
      aria-pressed={value === l}
      class="relative px-1.5 py-1 text-[10px] tracking-wide border transition-colors
        {value === l ? 'border-[#c65f3c] text-[#34251c] bg-[#f2e8da]' : 'border-transparent text-[#5f4636] hover:text-[#34251c]'}"
    >{l.toUpperCase()}{#if l !== 'en' && filled(l)}<span class="absolute -top-0.5 -right-0.5 w-1.5 h-1.5 rounded-full bg-emerald-700" title={$t('adminFormTextLangFilled')}></span>{/if}</button>
  {/each}
</div>
