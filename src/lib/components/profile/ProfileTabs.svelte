<script lang="ts">
  /**
   * Рейка разделов профиля.
   *
   * Разделы — настоящие ссылки, а не значения переменной `view`: у каждого есть
   * адрес, поэтому «назад» в браузере возвращает на уровень выше, а не уводит с
   * профиля целиком, перезагрузка оставляет там же, где были, и на переписку
   * можно дать ссылку.
   *
   * Число печатается рядом с названием, потому что ради него раздел и
   * открывают. Красится только непрочитанное: оно значит «от вас ждут ответа»,
   * а не «здесь столько-то строк».
   */
  import { page } from '$app/state';
  import { t } from '$lib/i18n';
  import { userDesk } from '$lib/stores/user-desk.svelte';

  interface Tab {
    href: string;
    label: string;
    count: number;
    unread?: number;
  }

  let tabs = $derived<Tab[]>([
    { href: '/profile', label: $t('profileTabLatest'), count: 0 },
    { href: '/profile/dealings', label: $t('profileHubDealings'), count: userDesk.counts.deals },
    { href: '/profile/messages', label: $t('profileMessages'), count: userDesk.counts.threads, unread: userDesk.counts.unread },
    { href: '/profile/wishlist', label: $t('profileWishlist'), count: userDesk.counts.wishlist },
    { href: '/profile/watches', label: $t('profileWatches'), count: userDesk.counts.watches },
    { href: '/profile/account', label: $t('profileTabAccount'), count: 0 },
  ]);

  /** Подраздел светит свой раздел: ветка письма — «Письма», карточка — «Дела». */
  function isHere(href: string): boolean {
    const path = page.url.pathname.replace(/\/$/, '') || '/profile';
    return href === '/profile' ? path === '/profile' : path === href || path.startsWith(href + '/');
  }
</script>

<nav class="rail" aria-label={$t('profileTitle')}>
  {#each tabs as tab (tab.href)}
    {@const here = isHere(tab.href)}
    <a href={tab.href} class="tab" class:here aria-current={here ? 'page' : undefined}>
      <span class="tab-name">{tab.label}</span>
      {#if tab.unread}
        <span class="pf-count pf-count--new">{tab.unread}</span>
      {:else if tab.count > 0}
        <span class="pf-count">{tab.count}</span>
      {/if}
    </a>
  {/each}
</nav>

<style>
  .rail {
    display: flex;
    align-items: stretch;
    gap: 0.2rem;
    margin-top: 1.5rem;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 12%, transparent);
    overflow-x: auto;
    scrollbar-width: none;
  }
  .rail::-webkit-scrollbar { display: none; }

  .tab {
    display: flex;
    align-items: baseline;
    gap: 0.42rem;
    padding: 0.62rem 0.7rem;
    white-space: nowrap;
    text-decoration: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    transition: color 0.15s, border-color 0.15s;
  }

  .tab-name {
    font-family: var(--font-body);
    font-size: 0.84rem;
    letter-spacing: 0.02em;
    color: var(--color-ink-tertiary);
    transition: color 0.15s;
  }

  .tab:hover .tab-name { color: var(--color-ink-primary); }

  .tab.here { border-bottom-color: var(--color-ember); }
  .tab.here .tab-name { color: var(--color-ink-primary); }

  .tab:focus-visible {
    outline: 2px solid var(--color-ember);
    outline-offset: -2px;
  }

  @media (max-width: 680px) {
    .tab { padding: 0.6rem 0.55rem; }
    .tab-name { font-size: 0.8rem; }
    /* Шесть разделов в 375 px не помещаются никогда, а полоса прокрутки у
       рейки снята: без растворяющегося края последний раздел выглядел бы
       обрезанным по случайности, а не продолжающимся. */
    .rail {
      mask-image: linear-gradient(to right, #000 calc(100% - 28px), transparent);
    }
  }
</style>
