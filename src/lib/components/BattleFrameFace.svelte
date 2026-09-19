<script lang="ts">
  // Лицо рамки: её картинка на её же бумаге и в её же кайме.
  //
  // Один отрисовщик на всех, потому что лицо рисуют уже в трёх местах — строка
  // ящика нарядов, гардероб и лента чинов, — и считалось оно в каждом порознь.
  // Лицо — это ответ на «какая это рамка», и три ответа на один вопрос рано
  // или поздно разойдутся: у собранной из частей узнают угол, у прочих целую
  // фотографию, и стоит одному месту забыть про притолоку, как в нём половина
  // рамок станет пустыми квадратиками.
  //
  // Рамка без единой картинки узнаётся по цвету бумаги и каймы, поэтому
  // квадратик рисуется ВСЕГДА, а не только когда есть что показать.
  import type { BattleFrame } from '$lib/types/api';

  let {
    frame,
    dashed = false,
    class: extra = '',
  } = $props<{
    /** Не дана — пустое место: пунктир вместо лица. */
    frame?: BattleFrame | null;
    /** Пунктирная кромка у пустого места. */
    dashed?: boolean;
    /** Величина и всё прочее — от того, кто ставит: у ленты своя, у
     *  гардероба своя, и размер лица не свойство лица. */
    class?: string;
  }>();

  /** Та часть рамки, по которой её узнают. У собранной из частей единой
   *  фотографии нет вовсе — берётся угол, а если его не нарисовали, то
   *  притолока или боковина. */
  function face(one: BattleFrame): string {
    const art =
      one.frameMode === 'sliced'
        ? one.cornerImage?.trim() ||
          one.sideImageH?.trim() ||
          one.sideImageV?.trim()
        : one.frameImage?.trim();
    return art ? `url("${art}")` : 'none';
  }
</script>

{#if frame}
  <span
    class="flex-shrink-0 border bg-center bg-contain bg-no-repeat {extra}"
    style="background-color:{frame.paper}; border-color:{frame.border}; background-image:{face(
      frame,
    )}"
  ></span>
{:else}
  <span
    class="flex-shrink-0 border border-[#34251c]/25 {dashed
      ? 'border-dashed'
      : ''} {extra}"
  ></span>
{/if}
