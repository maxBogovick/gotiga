-- Уведомления в браузере: четвёртая тема «Игра» (новые карты и новые битвы).
--
-- Только расширяет список допустимых тем. Подписки не трогаются: у тех, кто подписался
-- раньше, новой темы нет, и она включается только их собственным выбором в панели
-- уведомлений. Карты и битвы, что к выкладке уже на сайте, сервер при первом проходе
-- отмечает ушедшими (настройка `push_game_since`), так что о старом не придёт ничего.
DO $$
DECLARE
    old_check TEXT;
BEGIN
    FOR old_check IN
        SELECT c.conname
          FROM pg_constraint c
         WHERE c.conrelid = 'push_subscriptions'::regclass
           AND c.contype = 'c'
           AND pg_get_constraintdef(c.oid) LIKE '%topics%'
           AND pg_get_constraintdef(c.oid) LIKE '%gazette%'
    LOOP
        EXECUTE format('ALTER TABLE push_subscriptions DROP CONSTRAINT %I', old_check);
    END LOOP;
END $$;

ALTER TABLE push_subscriptions
    ADD CONSTRAINT push_subscriptions_topics_check
    CHECK (cardinality(topics) > 0 AND topics <@ ARRAY['works', 'tales', 'gazette', 'game']);
