<script lang="ts">
  /**
   * Дополнительные двери под полем почты: Google, Apple, Telegram — одним
   * блоком и под одной линией «или».
   *
   * Линия одна, потому что двери равны и ни одна не объявлена главной. Рисует
   * её этот блок, а не каждая дверь: прежде её печатал Telegram сам, и с
   * появлением второй двери рядом вышли бы две линии подряд. Видна она, только
   * если внутри есть хоть одна дверь (`:has`) — какая из них включена на этом
   * сервере, знает каждая сама.
   */
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import { interact } from '$lib/analytics';
  import { beginOauth } from '$lib/oauth';
  import TelegramDoor from './TelegramDoor.svelte';
  import type { OauthConfig, OauthProvider, UserDto } from '$lib/types/api';

  interface Props {
    /** Что делать, когда дверь отдала сессию (Telegram отдаёт её здесь же). */
    ondone: (sessionToken: string, user: UserDto | null) => void;
    /** Куда вернуться после входа через провайдера. По умолчанию — `?from=`. */
    from?: string | null;
    /**
     * Страница регистрации: двери закрыты, пока не отмечено «мне 16 или
     * больше». На странице входа этого нет — вернувшегося вопрос о возрасте
     * не касается, а нового имени там не заводят (сервер вернёт отказ `age`).
     */
    needsAge?: boolean;
    ageConfirmed?: boolean;
  }
  let { ondone, from = null, needsAge = false, ageConfirmed = false }: Props = $props();
  let gated = $derived(needsAge && !ageConfirmed);

  let conf = $state<OauthConfig>({ google: false, apple: false });
  let busy = $state<OauthProvider | null>(null);
  let failed = $state(false);

  onMount(async () => {
    try {
      conf = await api.oauthConfig();
    } catch {
      conf = { google: false, apple: false }; // сервер старой сборки — дверей просто нет
    }
  });

  async function go(provider: OauthProvider) {
    if (busy) return;
    busy = provider;
    failed = false;
    interact('auth_oauth', `${provider}:login:start`);
    try {
      await beginOauth(provider, {
        mode: 'login',
        from: from ?? page.url.searchParams.get('from'),
        ageConfirmed: needsAge && ageConfirmed
      });
      // Дальше страницу меняет переход; кнопка остаётся занятой до самого ухода.
    } catch {
      failed = true;
      busy = null;
    }
  }
</script>

<div class="auth-doors">
  <div class="auth-doors-rule"><span>{$t('authTelegramOr')}</span></div>

  <!-- Пока возраст не отмечен, двери закрыты целиком (`inert`): ни кнопкой, ни
       с клавиатуры. Тот же закон держит и сервер — у Google, Apple и Telegram
       новое имя без отметки не заводится. -->
  <div class="gate" class:gate--shut={gated} inert={gated}>
  {#if conf.google || conf.apple}
    <div class="oauth">
      {#if conf.google}
        <button class="oauth-btn" onclick={() => go('google')} disabled={busy !== null}>
          <!-- Знак Google в его настоящих цветах: правила бренда не разрешают
               перекрашивать ни «G», ни кнопку целиком в чужую палитру. -->
          <svg width="18" height="18" viewBox="0 0 48 48" aria-hidden="true">
            <path fill="#EA4335" d="M24 9.5c3.54 0 6.71 1.22 9.21 3.6l6.85-6.85C35.9 2.38 30.47 0 24 0 14.62 0 6.51 5.38 2.56 13.22l7.98 6.19C12.43 13.72 17.74 9.5 24 9.5z" />
            <path fill="#4285F4" d="M46.98 24.55c0-1.57-.15-3.09-.38-4.55H24v9.02h12.94c-.58 2.96-2.26 5.48-4.78 7.18l7.73 6c4.51-4.18 7.09-10.36 7.09-17.65z" />
            <path fill="#FBBC05" d="M10.53 28.59c-.48-1.45-.76-2.99-.76-4.59s.27-3.14.76-4.59l-7.98-6.19C.92 16.46 0 20.12 0 24c0 3.88.92 7.54 2.56 10.78l7.97-6.19z" />
            <path fill="#34A853" d="M24 48c6.48 0 11.93-2.13 15.89-5.81l-7.73-6c-2.15 1.45-4.92 2.3-8.16 2.3-6.26 0-11.57-4.22-13.47-9.91l-7.98 6.19C6.51 42.62 14.62 48 24 48z" />
          </svg>
          {busy === 'google' ? '…' : $t('authOauthGoogle')}
        </button>
      {/if}
      {#if conf.apple}
        <button class="oauth-btn" onclick={() => go('apple')} disabled={busy !== null}>
          <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
            <path fill="currentColor" d="M12.152 6.896c-.948 0-2.415-1.078-3.96-1.04-2.04.027-3.91 1.183-4.961 3.014-2.117 3.675-.546 9.103 1.519 12.09 1.013 1.454 2.208 3.09 3.792 3.039 1.52-.065 2.09-.987 3.935-.987 1.831 0 2.35.987 3.96.948 1.637-.026 2.676-1.48 3.676-2.948 1.156-1.688 1.636-3.325 1.662-3.415-.039-.013-3.182-1.221-3.22-4.857-.026-3.04 2.48-4.494 2.597-4.559-1.429-2.09-3.623-2.324-4.39-2.376-2-.156-3.675 1.09-4.61 1.09zM15.53 3.83c.843-1.012 1.4-2.427 1.245-3.83-1.207.052-2.662.805-3.532 1.818-.78.896-1.454 2.338-1.273 3.714 1.338.104 2.715-.688 3.559-1.701" />
          </svg>
          {busy === 'apple' ? '…' : $t('authOauthApple')}
        </button>
      {/if}
    </div>
    {#if failed}
      <p class="oauth-note" role="alert">{$t('authOauthFailed')}</p>
    {/if}
  {/if}

  <TelegramDoor {ondone} ageConfirmed={needsAge && ageConfirmed} />
  </div>
  {#if gated}
    <p class="oauth-note oauth-note--quiet" role="status">{$t('authDoorsAgeHint')}</p>
  {/if}
</div>

<style>
  .auth-doors {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-top: 14px;
  }

  /* «или» на линии. Скрыта, пока ни одной двери не нарисовано. */
  .auth-doors-rule {
    display: none;
    align-items: center;
    gap: 10px;
    width: 100%;
    margin: 2px 0 10px;
    color: rgba(95, 70, 54, 0.4);
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 12px;
    font-style: italic;
  }
  :global(.auth-doors:has(.oauth, .door) > .auth-doors-rule) {
    display: flex;
  }
  .auth-doors-rule::before,
  .auth-doors-rule::after {
    content: '';
    flex: 1;
    height: 1px;
    background: #d8c6b1;
  }

  .oauth {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: 100%;
  }

  /* Тот же язык, что у кнопки Telegram: пергамент, тонкая рамка, Georgia. Знаки
     провайдеров сохраняют свои цвета — это условие их бренда, а не вкус. */
  .oauth-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 9px;
    min-width: 15rem;
    padding: 9px 18px;
    font-family: Georgia, serif;
    font-size: 14px;
    letter-spacing: 0.02em;
    color: #34251c;
    background: rgba(255, 255, 255, 0.45);
    border: 1px solid #d8c6b1;
    border-radius: 3px;
    cursor: pointer;
    transition: border-color 0.2s, background 0.2s;
  }
  .oauth-btn:hover:not(:disabled) {
    border-color: #c65f3c;
    background: rgba(255, 255, 255, 0.75);
  }
  .oauth-btn:focus-visible {
    outline: 2px solid #c65f3c;
    outline-offset: 2px;
  }
  .oauth-btn:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .gate {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 100%;
    transition: opacity 0.2s;
  }
  .gate--shut {
    opacity: 0.45;
  }
  .oauth-note--quiet {
    color: rgba(95, 70, 54, 0.7);
  }

  .oauth-note {
    margin: 8px 0 0;
    max-width: 320px;
    text-align: center;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 13px;
    font-style: italic;
    color: #c65f3c;
  }
</style>
