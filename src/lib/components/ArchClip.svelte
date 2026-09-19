<script lang="ts">
  /**
   * Стрельчатая арка — одна на весь дом.
   *
   * Обтравка объявлена в долях коробки (`objectBoundingBox`), поэтому один и
   * тот же путь годится и на арку полки шириной 320 px, и на арочку в 26 px у
   * нити: путь в пикселях пришлось бы держать в двух вариантах и править
   * парой. Маршрутов, которым нужна арка, стало два, а форма у неё одна —
   * значит и объявление одно.
   *
   * Элемент нулевого размера, но не `display: none`: скрытый владелец уносит
   * обтравку с собой.
   */
</script>

<svg class="arch-defs" aria-hidden="true" focusable="false">
  <defs>
    <clipPath id="tale-arch" clipPathUnits="objectBoundingBox">
      <path d="M0,1 L0,0.46 C0,0.17 0.40,0.11 0.5,0 C0.60,0.11 1,0.17 1,0.46 L1,1 Z" />
    </clipPath>
  </defs>
</svg>

<style>
  .arch-defs {
    position: absolute;
    width: 0;
    height: 0;
    overflow: hidden;
  }
</style>
