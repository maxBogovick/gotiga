<script lang="ts">
  // Окно дома вместо системного `prompt()`.
  //
  // Их было семнадцать: цена, ставка, слово отказа, «выбросить работу»,
  // жалоба. Серое системное окно посреди пергамента — самое громкое, что было в
  // студии, и это ровно то, чего дом не делает нигде больше: у него своё окно —
  // рамка, повёрнутая на градус, двойная кайма.
  //
  // И оно не только тише: в системную строку нельзя было написать, из чего
  // выбирают. Коридор цены («от 50 до 5000») человек узнавал ТОЛЬКО из отказа
  // сервера, уже нажав.
  import { t } from '$lib/i18n';

  interface Props {
    /** Заголовок — одной строкой, о чём спрашивают. */
    title: string;
    /** Пояснение под ним: то, чего в системном окне сказать было негде. */
    lead?: string;
    /** Поле ввода. Пусто — окно спрашивает только «да или нет». */
    field?: 'text' | 'number';
    value?: string;
    /** Коридор числа. Назван и в подсказке, и в самом поле. */
    min?: number;
    max?: number;
    /** Слово на кнопке согласия. */
    yes: string;
    /** Опасное ли согласие: необратимое красится домом иначе. */
    danger?: boolean;
    onyes: (value: string) => void;
    onclose: () => void;
  }
  let {
    title,
    lead,
    field,
    value = '',
    min,
    max,
    yes,
    danger = false,
    onyes,
    onclose,
  }: Props = $props();

  let said = $state(value);
  let box: HTMLInputElement | null = $state(null);

  // Поле берётся в руку само: окно открыли, чтобы в него написать.
  $effect(() => {
    box?.focus();
    box?.select();
  });

  function done() {
    if (field && !said.trim()) return;
    onyes(said.trim());
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  class="veil"
  onclick={(e) => e.target === e.currentTarget && onclose()}
  role="dialog"
  aria-modal="true"
>
  <div class="leaf">
    <h2>{title}</h2>
    {#if lead}<p class="lead">{lead}</p>{/if}

    {#if field}
      <input
        bind:this={box}
        bind:value={said}
        type={field}
        {min}
        {max}
        onkeydown={(e) => {
          if (e.key === 'Enter') done();
          if (e.key === 'Escape') onclose();
        }}
      />
    {/if}

    <div class="deeds">
      <button type="button" class="yes" class:yes--danger={danger} onclick={done}>{yes}</button>
      <button type="button" class="no" onclick={onclose}>{$t('studioAskNo')}</button>
    </div>
  </div>
</div>

<style>
  .veil {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    background: rgba(52, 37, 28, 0.42);
  }

  /* Повёрнутый на градус лист с двойной каймой — то же окно, каким дом
     спрашивает везде: в модалках брони, взятия карты, приветствия. */
  .leaf {
    max-width: 30rem;
    padding: 1.6rem 1.7rem 1.4rem;
    border: 1px solid #d8c6b1;
    outline: 1px solid rgba(216, 198, 177, 0.5);
    outline-offset: 5px;
    background: #f8f1e7;
    transform: rotate(-1deg);
    box-shadow: 0 18px 40px rgba(52, 37, 28, 0.18);
  }

  h2 {
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.35rem;
    font-weight: 400;
    color: #34251c;
  }

  .lead {
    margin: 0.6rem 0 0;
    max-width: 40ch;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.95rem;
    line-height: 1.55;
    color: #6f3b24;
    opacity: 0.85;
  }

  input {
    display: block;
    width: 100%;
    margin: 1rem 0 0;
    padding: 0.5rem 0.6rem;
    border: 1px solid #d8c6b1;
    background: #fff;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1rem;
    color: #34251c;
  }

  input:focus {
    outline: none;
    border-color: rgba(198, 95, 60, 0.5);
  }

  .deeds {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    margin: 1.3rem 0 0;
  }

  .yes,
  .no {
    padding: 0.5rem 1.1rem;
    border: 1px solid rgba(198, 95, 60, 0.5);
    background: none;
    font-size: 0.66rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #c65f3c;
    cursor: pointer;
  }

  .yes:hover {
    background: rgba(198, 95, 60, 0.08);
  }

  /* Необратимое красится иначе — и это единственная разница, потому что
     разница в цвете видна и боковым зрением. */
  .yes--danger {
    border-color: rgba(143, 47, 34, 0.5);
    color: #8f2f22;
  }

  .yes--danger:hover {
    background: rgba(143, 47, 34, 0.08);
  }

  .no {
    border-color: rgba(52, 37, 28, 0.2);
    color: #34251c;
  }

  .no:hover {
    background: rgba(52, 37, 28, 0.05);
  }

  @media (prefers-reduced-motion: reduce) {
    .leaf {
      transform: none;
    }
  }
</style>
