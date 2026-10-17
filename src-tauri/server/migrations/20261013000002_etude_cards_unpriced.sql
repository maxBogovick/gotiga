-- Снаряжение этюда («только для этюдов», 20261013000001) не продаётся, и цена
-- ему не нужна. Прочие карты по-прежнему обязаны иметь цену хотя бы в одной
-- монете: карта на полке без цены — это карта, которую нельзя взять.
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_has_price;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_has_price
    CHECK (status = 'etude' OR price_dust IS NOT NULL OR price_feed IS NOT NULL);
