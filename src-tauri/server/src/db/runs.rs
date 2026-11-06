//! Забег: маршруты, волны, забеги игроков (`runs.rs`, миграция `battle_runs`).

use super::Repository;
use crate::error::{AppError, Result};
use crate::models::BattleMatch;
use crate::runs::{BattleRoute, BattleRouteWave, BattleRun, BattleRunTrophy};
use uuid::Uuid;

impl Repository {
    pub async fn list_published_battle_routes(&self) -> Result<Vec<(BattleRoute, i64)>> {
        let routes = sqlx::query_as::<_, BattleRoute>(
            "SELECT id, slug, title_en, title_ru, title_uk, title_es, title_de, title_it,
                    guest_waves, status, sort_order
               FROM battle_routes WHERE status = 'published'
              ORDER BY sort_order NULLS LAST, created_at",
        )
        .fetch_all(&self.pg_pool)
        .await?;
        let counts: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT route_id, COUNT(*) FROM battle_route_waves GROUP BY route_id",
        )
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(routes
            .into_iter()
            .map(|r| {
                let n = counts.iter().find(|(id, _)| *id == r.id).map(|c| c.1).unwrap_or(0);
                (r, n)
            })
            .collect())
    }

    pub async fn list_all_battle_routes(&self) -> Result<Vec<(BattleRoute, Vec<BattleRouteWave>)>> {
        let routes = sqlx::query_as::<_, BattleRoute>(
            "SELECT id, slug, title_en, title_ru, title_uk, title_es, title_de, title_it,
                    guest_waves, status, sort_order
               FROM battle_routes ORDER BY sort_order NULLS LAST, created_at",
        )
        .fetch_all(&self.pg_pool)
        .await?;
        let waves = sqlx::query_as::<_, BattleRouteWave>(
            "SELECT id, route_id, wave_no, challenge_id, reinforcements, goal,
                    min_cards, max_cards, reward_dust, race_hint
               FROM battle_route_waves ORDER BY route_id, wave_no",
        )
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(routes
            .into_iter()
            .map(|r| {
                let own = waves.iter().filter(|w| w.route_id == r.id).cloned().collect();
                (r, own)
            })
            .collect())
    }

    /// Маршрут целиком одной транзакцией: строка маршрута и все его волны
    /// (старые заменяются). Забеги, уже идущие по маршруту, остаются: они
    /// читают волны по номеру, и правка волны меняет те, что ещё не начаты.
    pub async fn save_battle_route(
        &self,
        id: Option<Uuid>,
        route: &crate::runs::AdminRoute,
        waves: &[(i16, Uuid, String, Option<String>, i16, i16, i32, Option<String>)],
    ) -> Result<Uuid> {
        let mut tx = self.pg_pool.begin().await?;
        let route_id: Uuid = match id {
            Some(id) => {
                let found: Option<(Uuid,)> = sqlx::query_as(
                    "UPDATE battle_routes
                        SET slug=$2, title_en=$3, title_ru=$4, title_uk=$5, title_es=$6,
                            title_de=$7, title_it=$8, guest_waves=$9, status=$10,
                            sort_order=$11, updated_at=NOW()
                      WHERE id=$1 RETURNING id",
                )
                .bind(id)
                .bind(&route.slug)
                .bind(&route.title_en)
                .bind(&route.title_ru)
                .bind(&route.title_uk)
                .bind(&route.title_es)
                .bind(&route.title_de)
                .bind(&route.title_it)
                .bind(route.guest_waves)
                .bind(&route.status)
                .bind(route.sort_order)
                .fetch_optional(&mut *tx)
                .await?;
                found.map(|f| f.0).ok_or_else(|| AppError::NotFound("No such route".into()))?
            }
            None => {
                let (id,): (Uuid,) = sqlx::query_as(
                    "INSERT INTO battle_routes
                         (slug, title_en, title_ru, title_uk, title_es, title_de, title_it,
                          guest_waves, status, sort_order)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING id",
                )
                .bind(&route.slug)
                .bind(&route.title_en)
                .bind(&route.title_ru)
                .bind(&route.title_uk)
                .bind(&route.title_es)
                .bind(&route.title_de)
                .bind(&route.title_it)
                .bind(route.guest_waves)
                .bind(&route.status)
                .bind(route.sort_order)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| match &e {
                    sqlx::Error::Database(d) if d.is_unique_violation() => {
                        AppError::BadRequest("A route with this slug exists".into())
                    }
                    _ => e.into(),
                })?;
                id
            }
        };
        sqlx::query("DELETE FROM battle_route_waves WHERE route_id = $1")
            .bind(route_id)
            .execute(&mut *tx)
            .await?;
        for (no, challenge, reinforcements, goal, min, max, dust, race) in waves {
            sqlx::query(
                "INSERT INTO battle_route_waves
                     (route_id, wave_no, challenge_id, reinforcements, goal,
                      min_cards, max_cards, reward_dust, race_hint)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
            )
            .bind(route_id)
            .bind(no)
            .bind(challenge)
            .bind(reinforcements)
            .bind(goal)
            .bind(min)
            .bind(max)
            .bind(dust)
            .bind(race)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(route_id)
    }

    /// Удалить маршрут. Пока по нему идут или шли забеги, он удаляется только
    /// снятием с публикации: забеги ссылаются на него.
    pub async fn delete_battle_route(&self, id: Uuid) -> Result<()> {
        let (runs,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM battle_runs WHERE route_id = $1")
            .bind(id)
            .fetch_one(&self.pg_pool)
            .await?;
        if runs > 0 {
            return Err(AppError::BadRequest("The route has runs: retire it instead".into()));
        }
        sqlx::query("DELETE FROM battle_routes WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn get_battle_route(&self, id: Uuid) -> Result<BattleRoute> {
        sqlx::query_as::<_, BattleRoute>(
            "SELECT id, slug, title_en, title_ru, title_uk, title_es, title_de, title_it,
                    guest_waves, status, sort_order
               FROM battle_routes WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound("No such route".into()))
    }

    pub async fn get_battle_route_by_slug(&self, slug: &str) -> Result<BattleRoute> {
        sqlx::query_as::<_, BattleRoute>(
            "SELECT id, slug, title_en, title_ru, title_uk, title_es, title_de, title_it,
                    guest_waves, status, sort_order
               FROM battle_routes WHERE slug = $1 AND status = 'published'",
        )
        .bind(slug)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound("No such route".into()))
    }

    pub async fn count_battle_route_waves(&self, route_id: Uuid) -> Result<i64> {
        Ok(sqlx::query_scalar("SELECT COUNT(*) FROM battle_route_waves WHERE route_id = $1")
            .bind(route_id)
            .fetch_one(&self.pg_pool)
            .await?)
    }

    pub async fn get_battle_route_wave(&self, route_id: Uuid, wave_no: i16) -> Result<BattleRouteWave> {
        sqlx::query_as::<_, BattleRouteWave>(
            "SELECT id, route_id, wave_no, challenge_id, reinforcements, goal,
                    min_cards, max_cards, reward_dust, race_hint
               FROM battle_route_waves WHERE route_id = $1 AND wave_no = $2",
        )
        .bind(route_id)
        .bind(wave_no)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound("No such wave".into()))
    }

    pub async fn get_battle_run(&self, id: Uuid, user_id: Uuid) -> Result<BattleRun> {
        sqlx::query_as::<_, BattleRun>(
            "SELECT id, user_id, route_id, wave_no, waves_total, state, squad, swaps, seed, phase, reward_options, free_swaps
               FROM battle_runs WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound("No such run".into()))
    }

    /// Забег без проверки хозяина: для расчёта итога партии, где хозяин уже
    /// проверен по партии.
    pub async fn get_battle_run_by_id(&self, id: Uuid) -> Result<Option<BattleRun>> {
        Ok(sqlx::query_as::<_, BattleRun>(
            "SELECT id, user_id, route_id, wave_no, waves_total, state, squad, swaps, seed, phase, reward_options, free_swaps
               FROM battle_runs WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn find_active_battle_run(&self, user_id: Uuid) -> Result<Option<BattleRun>> {
        Ok(sqlx::query_as::<_, BattleRun>(
            "SELECT id, user_id, route_id, wave_no, waves_total, state, squad, swaps, seed, phase, reward_options, free_swaps
               FROM battle_runs WHERE user_id = $1 AND state = 'active'",
        )
        .bind(user_id)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Последний забег человека, каким бы он ни кончился: по нему страница
    /// показывает итог и трофеи, пока человек не начнёт новый.
    pub async fn find_latest_battle_run(&self, user_id: Uuid) -> Result<Option<BattleRun>> {
        Ok(sqlx::query_as::<_, BattleRun>(
            "SELECT id, user_id, route_id, wave_no, waves_total, state, squad, swaps, seed, phase, reward_options, free_swaps
               FROM battle_runs WHERE user_id = $1
              ORDER BY created_at DESC LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Сколько незаконченных забегов идёт по маршруту.
    pub async fn count_active_route_runs(&self, route_id: Uuid) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*) FROM battle_runs WHERE route_id = $1 AND state = 'active'",
        )
        .bind(route_id)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    /// Новый забег. Второй незаконченный у того же человека не вставится
    /// (`battle_runs_active_idx`): тогда возвращается `None`.
    pub async fn insert_battle_run(
        &self,
        user_id: Uuid,
        route_id: Uuid,
        waves_total: i16,
        squad: &str,
        seed: i64,
        daily: Option<chrono::NaiveDate>,
    ) -> Result<Option<BattleRun>> {
        Ok(sqlx::query_as::<_, BattleRun>(
            "INSERT INTO battle_runs (user_id, route_id, waves_total, squad, seed, daily_date)
             VALUES ($1,$2,$3,$4,$5,$6)
             ON CONFLICT DO NOTHING
             RETURNING id, user_id, route_id, wave_no, waves_total, state, squad, swaps, seed, phase, reward_options, free_swaps",
        )
        .bind(user_id)
        .bind(route_id)
        .bind(waves_total)
        .bind(squad)
        .bind(seed)
        .bind(daily)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Законченные партии забега по порядку: номер волны, итог, раунды.
    pub async fn list_run_finished_matches(
        &self,
        run_id: Uuid,
    ) -> Result<Vec<(Uuid, i16, String, Option<i16>)>> {
        Ok(sqlx::query_as::<_, (Uuid, i16, String, Option<i16>)>(
            "SELECT id, run_wave, outcome, rounds FROM battle_matches
              WHERE run_id = $1 AND run_wave IS NOT NULL AND outcome IS NOT NULL
              ORDER BY created_at",
        )
        .bind(run_id)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Забег дня этого человека за эти сутки, если он уже был.
    pub async fn find_daily_battle_run(&self, user_id: Uuid, day: chrono::NaiveDate) -> Result<Option<BattleRun>> {
        Ok(sqlx::query_as::<_, BattleRun>(
            "SELECT id, user_id, route_id, wave_no, waves_total, state, squad, swaps, seed, phase, reward_options, free_swaps
               FROM battle_runs WHERE user_id = $1 AND daily_date = $2",
        )
        .bind(user_id)
        .bind(day)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Сколько человек начали забег дня и сколько прошли маршрут до конца.
    pub async fn daily_battle_run_counts(&self, day: chrono::NaiveDate) -> Result<(i64, i64)> {
        Ok(sqlx::query_as::<_, (i64, i64)>(
            "SELECT COUNT(*), COUNT(*) FILTER (WHERE state = 'won') FROM battle_runs WHERE daily_date = $1",
        )
        .bind(day)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    /// Итог волны: отряд, следующая волна и состояние. Двигает забег только с
    /// той волны, которую назвал вызывающий, — повтор ничего не меняет.
    pub async fn settle_battle_run(
        &self,
        id: Uuid,
        from_wave: i16,
        next_wave: i16,
        state: &str,
        squad: &str,
        phase: &str,
        reward_options: Option<&str>,
    ) -> Result<Option<BattleRun>> {
        Ok(sqlx::query_as::<_, BattleRun>(
            "UPDATE battle_runs
                SET wave_no = $3, state = $4, squad = $5, updated_at = NOW(),
                    phase = $6, reward_options = $7,
                    finished_at = CASE WHEN $4 = 'active' THEN NULL ELSE NOW() END
              WHERE id = $1 AND wave_no = $2 AND state = 'active'
              RETURNING id, user_id, route_id, wave_no, waves_total, state, squad, swaps, seed, phase, reward_options, free_swaps",
        )
        .bind(id)
        .bind(from_wave)
        .bind(next_wave)
        .bind(state)
        .bind(squad)
        .bind(phase)
        .bind(reward_options)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn end_battle_run(&self, id: Uuid, user_id: Uuid, state: &str) -> Result<()> {
        sqlx::query(
            "UPDATE battle_runs SET state = $3, updated_at = NOW(), finished_at = NOW()
              WHERE id = $1 AND user_id = $2 AND state = 'active'",
        )
        .bind(id)
        .bind(user_id)
        .bind(state)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn find_open_run_match(&self, run_id: Uuid) -> Result<Option<BattleMatch>> {
        Ok(sqlx::query_as::<_, BattleMatch>(
            "SELECT * FROM battle_matches WHERE run_id = $1 AND outcome IS NULL
              ORDER BY created_at DESC LIMIT 1",
        )
        .bind(run_id)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Последняя законченная партия забега, итог которой ещё не вернулся в
    /// забег (ход записан, а перенос не дошёл — сбой между двумя записями).
    pub async fn find_unsettled_run_match(&self, run_id: Uuid, wave_no: i16) -> Result<Option<BattleMatch>> {
        Ok(sqlx::query_as::<_, BattleMatch>(
            "SELECT * FROM battle_matches
              WHERE run_id = $1 AND run_wave = $2 AND outcome IS NOT NULL
              ORDER BY finished_at DESC NULLS LAST LIMIT 1",
        )
        .bind(run_id)
        .bind(wave_no)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_run_match(
        &self,
        user_id: Uuid,
        run_id: Uuid,
        run_wave: i16,
        run_cards: &str,
        setup: &str,
        rules_version: i32,
        board_cache: &str,
        rules: &str,
    ) -> Result<Option<BattleMatch>> {
        Ok(sqlx::query_as::<_, BattleMatch>(
            "INSERT INTO battle_matches
                 (user_id, challenge_id, run_id, run_wave, run_cards,
                  setup, rules_version, board_cache, rules)
             VALUES ($1, NULL, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT DO NOTHING RETURNING *",
        )
        .bind(user_id)
        .bind(run_id)
        .bind(run_wave)
        .bind(run_cards)
        .bind(setup)
        .bind(rules_version)
        .bind(board_cache)
        .bind(rules)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Пыль за забег — не больше, чем осталось от суточного предела.
    ///
    /// Возвращает, сколько записано. Кошелёк — тот же `battle_wallet_entries`:
    /// строка со своим ключом, повтор с тем же ключом ничего не пишет. Предел
    /// считается внутри замка кошелька, чтобы две волны, кончившиеся в одно
    /// мгновение, не прошли мимо него обе.
    pub async fn credit_run_dust(
        &self,
        user_id: Uuid,
        amount: i32,
        reason: &str,
        ref_id: Uuid,
        idem_key: &str,
        daily_cap: i32,
    ) -> Result<i32> {
        if amount <= 0 || daily_cap <= 0 {
            return Ok(0);
        }
        let mut tx = self.pg_pool.begin().await?;
        lock_wallet(&mut tx, user_id).await?;
        let given = credit_dust_locked(&mut tx, user_id, amount, reason, ref_id, idem_key, daily_cap).await?;
        tx.commit().await?;
        Ok(given)
    }

    /// Сколько пыли забег ещё может дать человеку сегодня (UTC).
    pub async fn run_dust_room(&self, user_id: Uuid, daily_cap: i32) -> Result<i32> {
        let (paid,): (Option<i64>,) = sqlx::query_as(RUN_DUST_TODAY)
            .bind(user_id)
            .fetch_one(&self.pg_pool)
            .await?;
        Ok((i64::from(daily_cap) - paid.unwrap_or(0)).clamp(0, i64::from(i32::MAX)) as i32)
    }

    /// Выбор награды: отряд и бесплатные замены меняются, фаза возвращается в
    /// `wave` — только из `reward`, и только если отряд тот, по которому награда
    /// посчитана (замена между чтением и записью его меняет). Пыль награды
    /// пишется той же транзакцией: два выбора разом не дадут и пыль, и лечение.
    /// Повтор находит фазу `wave` и ничего не делает.
    #[allow(clippy::too_many_arguments)]
    pub async fn finish_run_reward(
        &self,
        run_id: Uuid,
        user_id: Uuid,
        old_squad: &str,
        squad: &str,
        add_free_swaps: i32,
        add_heal_charges: i32,
        dust: Option<(i32, i32)>,
        pick: (&str, i32),
    ) -> Result<Option<BattleRun>> {
        let mut tx = self.pg_pool.begin().await?;
        lock_wallet(&mut tx, user_id).await?;
        let run = sqlx::query_as::<_, BattleRun>(
            "UPDATE battle_runs
                SET squad = $4, free_swaps = free_swaps + $5, heal_charges = heal_charges + $6,
                    phase = 'wave', reward_options = NULL, updated_at = NOW()
              WHERE id = $1 AND user_id = $2 AND squad = $3
                AND phase = 'reward' AND state = 'active'
              RETURNING id, user_id, route_id, wave_no, waves_total, state, squad, swaps, seed,
                        phase, reward_options, free_swaps",
        )
        .bind(run_id)
        .bind(user_id)
        .bind(old_squad)
        .bind(squad)
        .bind(add_free_swaps)
        .bind(add_heal_charges)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(run) = run else {
            tx.rollback().await?;
            return Ok(None);
        };
        sqlx::query(
            "INSERT INTO battle_run_picks (run_id, wave_no, kind, amount) VALUES ($1, $2, $3, $4)
             ON CONFLICT (run_id, wave_no) DO NOTHING",
        )
        .bind(run.id)
        .bind(run.wave_no)
        .bind(pick.0)
        .bind(pick.1)
        .execute(&mut *tx)
        .await?;
        if let Some((amount, daily_cap)) = dust {
            credit_dust_locked(
                &mut tx,
                user_id,
                amount,
                "run_reward",
                run.id,
                &format!("run:{}:reward:{}", run.id, run.wave_no),
                daily_cap,
            )
            .await?;
        }
        tx.commit().await?;
        Ok(Some(run))
    }

    /// Сколько зарядов лечения у забега.
    pub async fn run_heal_charges(&self, run_id: Uuid) -> Result<i32> {
        Ok(sqlx::query_scalar::<_, i32>("SELECT heal_charges FROM battle_runs WHERE id = $1")
            .bind(run_id)
            .fetch_optional(&self.pg_pool)
            .await?
            .unwrap_or(0))
    }

    /// Потратить заряд лечения: отряд меняется только если он тот, по которому
    /// лечение посчитано, и заряд есть. `None` — отряд уже другой или зарядов нет
    /// (повтор или гонка).
    pub async fn heal_run_card(
        &self,
        run_id: Uuid,
        user_id: Uuid,
        old_squad: &str,
        squad: &str,
    ) -> Result<Option<BattleRun>> {
        Ok(sqlx::query_as::<_, BattleRun>(
            "UPDATE battle_runs
                SET squad = $4, heal_charges = heal_charges - 1, updated_at = NOW()
              WHERE id = $1 AND user_id = $2 AND squad = $3
                AND heal_charges > 0 AND state = 'active'
              RETURNING id, user_id, route_id, wave_no, waves_total, state, squad, swaps, seed,
                        phase, reward_options, free_swaps",
        )
        .bind(run_id)
        .bind(user_id)
        .bind(old_squad)
        .bind(squad)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Замена: одна транзакция под замком кошелька. Забег двигается только с
    /// того числа замен, которое назвал вызывающий; цена списывается строкой с
    /// ключом по номеру замены. `None` — забег уже ушёл дальше (повтор).
    pub async fn swap_run_card(
        &self,
        run_id: Uuid,
        user_id: Uuid,
        expected_swaps: i32,
        old_squad: &str,
        squad: &str,
        cost: i32,
        use_free: bool,
    ) -> Result<Option<BattleRun>> {
        let mut tx = self.pg_pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
            .bind(user_id.to_string())
            .execute(&mut *tx)
            .await?;
        let locked: Option<(i32, String)> = sqlx::query_as(
            "SELECT swaps, squad FROM battle_runs
              WHERE id = $1 AND user_id = $2 AND state = 'active' FOR UPDATE",
        )
        .bind(run_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;
        // Отряд мог смениться между чтением и записью (выбор награды): замена,
        // посчитанная по старому отряду, затёрла бы его.
        if locked.as_ref().map(|l| (l.0, l.1.as_str())) != Some((expected_swaps, old_squad)) {
            tx.rollback().await?;
            return Ok(None);
        }
        if cost > 0 {
            let (balance,): (Option<i64>,) = sqlx::query_as(
                "SELECT SUM(amount)::bigint FROM battle_wallet_entries
                  WHERE user_id = $1 AND currency = 'dust'",
            )
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;
            if balance.unwrap_or(0) < i64::from(cost) {
                tx.rollback().await?;
                return Err(AppError::BadRequest("Not enough in the wallet".into()));
            }
            sqlx::query(
                "INSERT INTO battle_wallet_entries (user_id, currency, amount, reason, ref_id, idem_key)
                 VALUES ($1, 'dust', $2, 'run_swap', $3, $4)
                 ON CONFLICT (user_id, idem_key) DO NOTHING",
            )
            .bind(user_id)
            .bind(-cost)
            .bind(run_id)
            .bind(format!("run:{run_id}:swap:{expected_swaps}"))
            .execute(&mut *tx)
            .await?;
        }
        let run = sqlx::query_as::<_, BattleRun>(
            "UPDATE battle_runs
                SET squad = $3, swaps = swaps + 1,
                    free_swaps = free_swaps - CASE WHEN $4 THEN 1 ELSE 0 END,
                    updated_at = NOW()
              WHERE id = $1 AND user_id = $2
              RETURNING id, user_id, route_id, wave_no, waves_total, state, squad, swaps, seed,
                        phase, reward_options, free_swaps",
        )
        .bind(run_id)
        .bind(user_id)
        .bind(squad)
        .bind(use_free)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(run)
    }

    pub async fn insert_run_trophies(&self, run_id: Uuid, wave_no: i16, trophies: &[(Uuid, i32)]) -> Result<()> {
        for (card_id, price) in trophies {
            sqlx::query(
                "INSERT INTO battle_run_trophies (run_id, card_id, wave_no, price_dust)
                 VALUES ($1,$2,$3,$4) ON CONFLICT (run_id, card_id) DO NOTHING",
            )
            .bind(run_id)
            .bind(card_id)
            .bind(wave_no)
            .bind(price)
            .execute(&self.pg_pool)
            .await?;
        }
        Ok(())
    }

    /// Выбранные награды забега по порядку волн.
    pub async fn list_run_picks(&self, run_id: Uuid) -> Result<Vec<(i16, String, i32)>> {
        Ok(sqlx::query_as("SELECT wave_no, kind, amount FROM battle_run_picks WHERE run_id = $1 ORDER BY wave_no")
            .bind(run_id)
            .fetch_all(&self.pg_pool)
            .await?)
    }

    pub async fn list_run_trophies(&self, run_id: Uuid) -> Result<Vec<BattleRunTrophy>> {
        Ok(sqlx::query_as::<_, BattleRunTrophy>(
            "SELECT run_id, card_id, wave_no, price_dust, taken
               FROM battle_run_trophies WHERE run_id = $1 ORDER BY wave_no, created_at, card_id",
        )
        .bind(run_id)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Взять трофей на полку: одна транзакция под замком кошелька, тем же
    /// порядком, что и покупка (`buy_battle_card`): владение, баланс, номер
    /// экземпляра (тираж может кончиться), списание, запись экземпляра.
    /// `Ok(false)` — карта уже у человека или трофей уже взят.
    /// Какие из этих карт уже лежат на полке у человека.
    pub async fn owned_card_ids(&self, user_id: Uuid, ids: &[Uuid]) -> Result<std::collections::HashSet<Uuid>> {
        let rows: Vec<(Uuid,)> =
            sqlx::query_as("SELECT DISTINCT card_id FROM card_copies WHERE owner_id = $1 AND card_id = ANY($2)")
                .bind(user_id)
                .bind(ids)
                .fetch_all(&self.pg_pool)
                .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// `join` получает отряд забега строкой и возвращает новый, если купленной
    /// карте в нём есть место; запись идёт в той же транзакции, что и покупка.
    pub async fn take_run_trophy(
        &self,
        run_id: Uuid,
        user_id: Uuid,
        card_id: Uuid,
        join: impl FnOnce(&str) -> Option<String>,
    ) -> Result<bool> {
        let mut tx = self.pg_pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
            .bind(user_id.to_string())
            .execute(&mut *tx)
            .await?;
        let trophy: Option<(i32, bool)> = sqlx::query_as(
            "SELECT t.price_dust, t.taken
               FROM battle_run_trophies t JOIN battle_runs r ON r.id = t.run_id
              WHERE t.run_id = $1 AND t.card_id = $2 AND r.user_id = $3 FOR UPDATE OF t",
        )
        .bind(run_id)
        .bind(card_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((price, taken)) = trophy else {
            tx.rollback().await?;
            return Err(AppError::NotFound("No such trophy".into()));
        };
        if taken {
            tx.rollback().await?;
            return Ok(false);
        }
        if price <= 0 {
            tx.rollback().await?;
            return Err(AppError::BadRequest("run:notForSale".into()));
        }
        let (owned,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM card_copies WHERE owner_id = $1 AND card_id = $2)",
        )
        .bind(user_id)
        .bind(card_id)
        .fetch_one(&mut *tx)
        .await?;
        if owned {
            tx.rollback().await?;
            return Ok(false);
        }
        let (balance,): (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(amount)::bigint FROM battle_wallet_entries
              WHERE user_id = $1 AND currency = 'dust'",
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
        if balance.unwrap_or(0) < i64::from(price) {
            tx.rollback().await?;
            return Err(AppError::BadRequest("Not enough in the wallet".into()));
        }
        // Трофей берётся только с опубликованной карты: этюдная карта на полку
        // не попадает, пока хранитель сам её не опубликует.
        let minted: Option<(i32,)> = sqlx::query_as(
            "UPDATE battle_cards SET minted = minted + 1
              WHERE id = $1 AND status = 'published'
                AND (edition_size IS NULL OR minted < edition_size)
          RETURNING minted",
        )
        .bind(card_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((serial,)) = minted else {
            tx.rollback().await?;
            return Err(AppError::BadRequest("run:notOnTheShelf".into()));
        };
        sqlx::query(
            "INSERT INTO battle_wallet_entries (user_id, currency, amount, reason, ref_id, idem_key)
             VALUES ($1, 'dust', $2, 'run_trophy', $3, $4)
             ON CONFLICT (user_id, idem_key) DO NOTHING",
        )
        .bind(user_id)
        .bind(-price)
        .bind(card_id)
        .bind(format!("run:{run_id}:trophy:{card_id}"))
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO card_copies (card_id, owner_id, serial, level, origin)
             VALUES ($1, $2, $3, 1, 'prize')",
        )
        .bind(card_id)
        .bind(user_id)
        .bind(serial)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE battle_run_trophies SET taken = TRUE WHERE run_id = $1 AND card_id = $2")
            .bind(run_id)
            .bind(card_id)
            .execute(&mut *tx)
            .await?;
        let seated: Option<(String,)> =
            sqlx::query_as("SELECT squad FROM battle_runs WHERE id = $1 AND state = 'active' FOR UPDATE")
                .bind(run_id)
                .fetch_optional(&mut *tx)
                .await?;
        if let Some(next) = seated.and_then(|(old,)| join(&old)) {
            sqlx::query("UPDATE battle_runs SET squad = $2, updated_at = NOW() WHERE id = $1")
                .bind(run_id)
                .bind(next)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(true)
    }
}

const RUN_DUST_TODAY: &str = "SELECT SUM(amount)::bigint FROM battle_wallet_entries
      WHERE user_id = $1 AND currency = 'dust'
        AND reason IN ('run_wave', 'run_reward')
        AND created_at >= date_trunc('day', NOW() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC'";

type WalletTx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

/// Замок кошелька человека: тот же, что у покупки карты.
async fn lock_wallet(tx: &mut WalletTx<'_>, user_id: Uuid) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
        .bind(user_id.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Пыль за забег под замком кошелька: не больше остатка суточного предела.
/// Возвращает, сколько записано; повтор с тем же ключом пишет ноль.
async fn credit_dust_locked(
    tx: &mut WalletTx<'_>,
    user_id: Uuid,
    amount: i32,
    reason: &str,
    ref_id: Uuid,
    idem_key: &str,
    daily_cap: i32,
) -> Result<i32> {
    if amount <= 0 || daily_cap <= 0 {
        return Ok(0);
    }
    let (paid,): (Option<i64>,) = sqlx::query_as(RUN_DUST_TODAY)
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await?;
    let room = (i64::from(daily_cap) - paid.unwrap_or(0)).max(0);
    let give = i64::from(amount).min(room) as i32;
    if give <= 0 {
        return Ok(0);
    }
    let res = sqlx::query(
        "INSERT INTO battle_wallet_entries (user_id, currency, amount, reason, ref_id, idem_key)
         VALUES ($1, 'dust', $2, $3, $4, $5)
         ON CONFLICT (user_id, idem_key) DO NOTHING",
    )
    .bind(user_id)
    .bind(give)
    .bind(reason)
    .bind(ref_id)
    .bind(idem_key)
    .execute(&mut **tx)
    .await?;
    Ok(if res.rows_affected() > 0 { give } else { 0 })
}
