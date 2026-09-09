/** Две привычки числового поля, которые дом правит везде одинаково.
 *
 *  Лежали в трёх местах порознь (`BattlesPanel`, `sheet/StatCell`, и с выносом
 *  стола рамок стало бы четыре), а поведение поля — не свойство вкладки.
 */

/** Число целиком перепечатывают чаще, чем правят по цифре: щелчок и ввод
 *  ведут себя так же, как везде. */
export function selectOnFocus(e: FocusEvent & { currentTarget: HTMLInputElement }) {
	e.currentTarget.select();
}

/** Поле `type="number"` под фокусом крутит своё значение обычным колесом
 *  страницы в Chrome и Firefox. Здесь это неожиданно: поле стоит в колонке,
 *  которую прокручивают. Снятый фокус возвращает прокрутку странице вместо
 *  того, чтобы молча изменить цену, силу или врезку. */
export function blurOnWheel(e: WheelEvent & { currentTarget: HTMLInputElement }) {
	e.currentTarget.blur();
}
