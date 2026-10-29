<script lang="ts">
  // Что человек указал в мастере заявки: способы связи, пожелание, выбор, размер, срок,
  // бюджет, фото — и готовое сообщение со ссылкой на Etsy. Тексты админки русские.
  import { resolveMediaUrl } from '$lib/api';
  import { isLang, textInLang, type Lang } from '$lib/i18n';
  import { contactHref } from '$lib/requestChoices';
  import type { CommissionDto } from '$lib/types/api';

  let { c }: { c: CommissionDto } = $props();

  const CHANNEL: Record<string, string> = {
    email: 'Почта', telegram: 'Telegram', whatsapp: 'WhatsApp', viber: 'Viber', signal: 'Signal',
    instagram: 'Instagram', facebook: 'Facebook', x: 'X', phone: 'Телефон'
  };
  const PARTS: Record<string, string> = {
    look: 'образ и лицо', pose: 'поза', clothes: 'одежда', colors: 'цвета', props: 'предметы в руках', size: 'размер'
  };
  const WHO: Record<string, string> = {
    witch: 'ведьма', granny: 'бабушка', gnome: 'гном', girl: 'девочка', old_man: 'старик', creature: 'существо', other: 'другое'
  };
  const MOOD: Record<string, string> = { kind: 'добрая', sly: 'хитрая', dark: 'мрачная', funny: 'смешная', calm: 'спокойная' };
  const PROPS: Record<string, string> = {
    broom: 'метла', cat: 'кот', pumpkin: 'тыква', book: 'книга', lantern: 'фонарь', herbs: 'травы', bird: 'птица', other: 'другое'
  };
  const SIZE: Record<string, string> = {
    xs: 'очень маленькая', s: 'маленькая', m: 'средняя', l: 'крупная', xl: 'очень крупная', unsure: 'не знаю'
  };
  const BUDGET: Record<string, string> = {
    lt100: 'до 100', '100_200': '100–200', '200_400': '200–400', gt400: 'больше 400', discuss: 'обсудить'
  };

  const join = (keys: string[] | undefined, names: Record<string, string>) =>
    (keys ?? []).map((k) => names[k] ?? k).join(', ');

  let wish = $derived(c.wish === 'similar' ? 'похожую на готовую работу' : c.wish === 'new' ? 'новую' : '');
  let rows = $derived(
    [
      ['Хочет', wish],
      ['Оставить', join(c.choices?.keep, PARTS)],
      ['Изменить', join(c.choices?.change, PARTS)],
      ['Кто', join(c.choices?.who, WHO)],
      ['Настроение', join(c.choices?.mood, MOOD)],
      ['Предметы', join(c.choices?.props, PROPS)],
      ['Размер', c.sizeKey ? (SIZE[c.sizeKey] ?? c.sizeKey) : ''],
      [
        'Срок',
        c.deadlineKind === 'none' ? 'не спешит'
          : c.deadlineKind === 'asap' ? 'как можно скорее'
          : c.deadlineKind === 'date' ? (c.deadline ?? 'дата не указана') : ''
      ],
      [
        'Бюджет',
        c.budgetKey
          ? (BUDGET[c.budgetKey] ?? c.budgetKey) +
            (c.budgetKey !== 'discuss' && c.budgetCurrency ? ` ${c.budgetCurrency === 'EUR' ? '€' : '$'}` : '')
          : ''
      ]
    ].filter(([, v]) => v)
  );

  // Способы связи; у старой заявки без списка — почта.
  let contacts = $derived(
    c.contacts?.length
      ? c.contacts
      : c.requesterEmail
        ? [{ channel: 'email' as const, value: c.requesterEmail }]
        : []
  );

  let copied = $state('');
  async function copy(key: string, text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = key;
      setTimeout(() => { if (copied === key) copied = ''; }, 1500);
    } catch {
      copied = '';
    }
  }

  let etsyUrl = $state('');
  const ETSY = /^https:\/\/((www\.)?etsy\.com|etsy\.me)\/\S+$/i;
  let etsyOk = $derived(ETSY.test(etsyUrl.trim()));
  let etsyTouched = $state(false);

  async function copyMessage() {
    if (!etsyOk) { etsyTouched = true; return; }
    const l: Lang = isLang(c.lang) ? c.lang : 'en';
    const tpl = (await textInLang(l, 'etsyMessage')) || (await textInLang('en', 'etsyMessage'));
    const text = tpl.replace('{name}', c.requesterName || '').replace('{link}', etsyUrl.trim());
    await copy('etsy', text);
  }
</script>

<div class="mt-2 space-y-2">
  {#if contacts.length}
    <div class="flex flex-wrap gap-2">
      {#each contacts as k (k.channel + k.value)}
        <span class="inline-flex items-stretch border border-[#34251c]/15 text-xs bg-[#fff9f0]">
          <a href={contactHref(k.channel, k.value)} target="_blank" rel="noopener" class="px-2 py-1 text-[#c65f3c] hover:underline">
            {CHANNEL[k.channel] ?? k.channel}: {k.value}
          </a>
          <button
            type="button"
            onclick={() => copy(k.channel + k.value, k.value)}
            class="px-2 border-l border-[#34251c]/15 text-[#5f4636] hover:bg-[#34251c]/5"
            title="Скопировать"
          >{copied === k.channel + k.value ? '✓' : 'Копировать'}</button>
        </span>
      {/each}
    </div>
  {/if}

  {#if rows.length}
    <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 text-xs text-[#5f4636]">
      {#each rows as [label, value] (label)}
        <dt class="font-semibold text-[#34251c]">{label}</dt>
        <dd class="m-0">{value}</dd>
      {/each}
    </dl>
  {/if}

  <div class="pt-1 border-t border-[#34251c]/5">
    <div class="flex flex-wrap items-center gap-2">
      <input
        type="url"
        bind:value={etsyUrl}
        placeholder="Ссылка на лот в Etsy (https://www.etsy.com/…)"
        class="flex-1 min-w-[220px] text-xs border border-[#34251c]/15 px-2 py-1 bg-[#fff9f0]"
        oninput={() => (etsyTouched = false)}
      />
      <button
        type="button"
        onclick={copyMessage}
        class="text-[10px] px-2 py-1 border border-[#6f3b24]/40 text-[#6f3b24] hover:border-[#6f3b24] hover:bg-[#6f3b24]/5 transition-colors"
      >{copied === 'etsy' ? '✓ Скопировано' : 'Скопировать сообщение со ссылкой на Etsy'}</button>
    </div>
    {#if etsyTouched && !etsyOk}
      <p class="text-[10px] text-[#a3361d] mt-1">Нужна ссылка вида https://www.etsy.com/… или https://etsy.me/…</p>
    {:else}
      <p class="text-[10px] text-[#5f4636]/60 mt-1">Сообщение будет на языке заявки: {c.lang || 'en'}.</p>
    {/if}
  </div>
</div>
