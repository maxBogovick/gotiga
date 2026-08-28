<script lang="ts">
  // Сургучная печать. Один оттиск на весь дом.
  //
  // Приём уже был у заказа работы — и это же причина вынести его: язык дома
  // узнаётся только тогда, когда он один и тот же. Второй сургуч, нарисованный
  // рядом, через полгода отличается от первого на оттенок и на градус поворота,
  // и человек читает эту разницу как разные дома.
  //
  // Форма — не круг: настоящий оттиск растекается. Неровность даёт
  // `border-radius` четырьмя парами, а не картинкой, чтобы печать оставалась
  // резкой на любом экране и красилась переменными.
  let {
    letter = 'G',
    size = '10rem',
    dim = false,
  }: {
    letter?: string;
    /** Любая длина CSS: печать квадратная и масштабируется целиком. */
    size?: string;
    /** Тусклая, без поворота — когда оттиск не в радость. */
    dim?: boolean;
  } = $props();
</script>

<div class="seal" class:seal--dim={dim} style="--seal-size:{size}" aria-hidden="true">
  <div class="wax">
    <div class="ring">
      <span class="letter">{letter}</span>
    </div>
  </div>
  <div class="shine"></div>
</div>

<style>
  .seal {
    position: relative;
    width: var(--seal-size);
    height: var(--seal-size);
    filter: drop-shadow(0 8px 18px rgba(52, 37, 28, 0.35));
  }

  .wax {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, #c65f3c, #a86124 55%, #9e452d);
    border: 4px solid rgba(111, 59, 36, 0.2);
    box-sizing: border-box;
    border-radius: 45% 55% 48% 52% / 51% 46% 54% 49%;
    box-shadow:
      inset 2px 2px 15px rgba(111, 59, 36, 0.16),
      inset -2px -2px 10px rgba(255, 255, 255, 0.1);
    transform: rotate(12deg);
    animation: press 600ms cubic-bezier(0.175, 0.885, 0.32, 1.275) both;
  }

  /* Не в радость — значит без размаха: тусклее и прямо. */
  .seal--dim .wax {
    filter: saturate(0.55);
    transform: rotate(0deg);
    animation: press-flat 600ms ease both;
  }

  .ring {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 70%;
    height: 70%;
    border: 2px solid rgba(111, 59, 36, 0.2);
    border-radius: 50%;
    box-shadow: inset 0 2px 6px rgba(111, 59, 36, 0.25);
  }

  .letter {
    margin: 6% 0 0 3%;
    font-family: 'Fraunces', Georgia, serif;
    font-size: calc(var(--seal-size) * 0.38);
    line-height: 1;
    color: #6f3b24;
    opacity: 0.7;
  }

  /* Блик — там же, где он был бы на настоящем воске: сверху слева. */
  .shine {
    position: absolute;
    top: 20%;
    left: 25%;
    width: 20%;
    height: 10%;
    background: #fff9f0;
    opacity: 0.25;
    filter: blur(3px);
    border-radius: 50%;
    transform: rotate(45deg);
  }

  @keyframes press {
    from { transform: scale(1.12) rotate(12deg); opacity: 0; }
    to { transform: scale(1) rotate(12deg); opacity: 1; }
  }

  @keyframes press-flat {
    from { transform: scale(1.08); opacity: 0; }
    to { transform: scale(1); opacity: 1; }
  }

  /* Оттиск остаётся оттиском, но не прижимается на глазах. */
  @media (prefers-reduced-motion: reduce) {
    .wax {
      animation: none;
    }
  }
</style>
