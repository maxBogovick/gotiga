//! Забег: начало, волна, перенос состояния (`runs.rs`, `TASKS-BATTLE-RUN.md`).
//!
//! Волна — обычная партия движка. Сервер собирает её расстановку из этюда-основы
//! (поле, местность, правила, враги с начала), расписания подкрепления, условия
//! победы и отряда игрока; движок получает готовое и не знает слова «забег».
//! Исход, здоровье после волны и следующая волна считаются здесь.

use super::*;
use crate::runs::*;

impl AppService {
    /// Настройки режима: рубильник и числа. Нет записи — умолчания (режим выключен).
    pub async fn get_run_settings(&self) -> Result<RunSettings> {
        let saved: RunSettings =
            parse_json_setting("battle_run", self.repo.get_setting("battle_run").await?)?;
        Ok(saved.clamped())
    }

    pub async fn save_run_settings(&self, settings: RunSettings) -> Result<RunSettings> {
        let clean = settings.clamped();
        let json = serde_json::to_string(&clean).map_err(|e| AppError::Internal(e.to_string()))?;
        self.repo.upsert_setting("battle_run", &json).await?;
        Ok(clean)
    }

    /// Выключенный режим не начинает забегов и не открывает волн; начатое
    /// читается по-прежнему.
    async fn assert_runs_are_on(&self) -> Result<RunSettings> {
        let settings = self.get_run_settings().await?;
        if !settings.enabled {
            return Err(AppError::BadRequest("run:off".into()));
        }
        Ok(settings)
    }

    pub async fn list_battle_routes(&self) -> Result<Vec<RouteDto>> {
        // Выключенный режим не показывает маршрутов: вместе с ними страница
        // не показывает и вход в него.
        if !self.get_run_settings().await?.enabled {
            return Ok(Vec::new());
        }
        Ok(self
            .repo
            .list_published_battle_routes()
            .await?
            .into_iter()
            .map(|(r, n)| Self::route_dto(&r, n as i16))
            .collect())
    }

    fn route_dto(r: &BattleRoute, waves: i16) -> RouteDto {
        RouteDto {
            slug: r.slug.clone(),
            title_en: r.title_en.clone(),
            title_ru: r.title_ru.clone(),
            title_uk: r.title_uk.clone(),
            title_es: r.title_es.clone(),
            title_de: r.title_de.clone(),
            title_it: r.title_it.clone(),
            waves,
            guest_waves: r.guest_waves,
        }
    }

    /// Начать забег — или вернуть тот, что человек оставил незаконченным.
    pub async fn start_battle_run(&self, user_id: Uuid, req: StartRunRequest) -> Result<RunDto> {
        if let Some(open) = self.repo.find_active_battle_run(user_id).await? {
            let open = self.reconcile_run(open).await?;
            return self.run_dto(&open).await;
        }
        self.assert_runs_are_on().await?;
        let today = chrono::Utc::now().date_naive();
        let route = if req.daily {
            // Забег дня один: сыгранный сегодня возвращается как есть.
            if let Some(done) = self.repo.find_daily_battle_run(user_id, today).await? {
                return self.run_dto(&done).await;
            }
            let slug = self.daily_route_slug(today).await?;
            self.repo.get_battle_route_by_slug(&slug).await?
        } else {
            self.repo.get_battle_route_by_slug(&req.route).await?
        };
        let total = self.repo.count_battle_route_waves(route.id).await? as i16;
        if total == 0 {
            return Err(AppError::NotFound("No such route".into()));
        }
        let guest = self.repo.user_is_guest(user_id).await?;
        let waves_total = if guest {
            // Гостю — короткая проба, а если маршрут ему не открыт — отказ.
            let trial = route.guest_waves.ok_or(AppError::Unauthorized)?;
            trial.min(total)
        } else {
            total
        };

        let shelf = self.list_battle_cards_public().await?;
        let fieldable: Vec<&BattleCardDto> =
            shelf.iter().filter(|c| crate::battles::can_take_the_field(c)).collect();
        let owned: std::collections::HashSet<Uuid> = self
            .repo
            .list_owned_battle_cards(user_id)
            .await?
            .into_iter()
            .map(|o| o.card_id)
            .collect();
        let mine: Vec<&BattleCardDto> = fieldable
            .iter()
            .copied()
            .filter(|c| c.id.parse().is_ok_and(|id| owned.contains(&id)))
            .collect();

        // Гостю и тому, у кого карт нет, выдаются слабейшие: забег не просит
        // сначала купить карту, и выданные не становятся владением.
        let (picked, loaned) = if guest || mine.is_empty() {
            (weakest(&fieldable, LOAN_CARDS), true)
        } else if req.card_ids.is_empty() {
            (strongest(&mine, SQUAD_MAX), false)
        } else {
            let mut seen = std::collections::HashSet::new();
            let ids: Vec<Uuid> = req.card_ids.iter().copied().filter(|id| seen.insert(*id)).collect();
            if ids.len() > SQUAD_MAX
                || ids.iter().any(|id| !mine.iter().any(|c| c.id.parse().ok() == Some(*id)))
            {
                return Err(AppError::BadRequest("run:badSquad".into()));
            }
            (ids, false)
        };
        if picked.is_empty() {
            return Err(AppError::BadRequest("run:noCards".into()));
        }
        let squad: Vec<SquadCard> = picked
            .into_iter()
            .map(|card_id| SquadCard { card_id, health: None, dead: false, loaned, cell: None })
            .collect();
        let written = serde_json::to_string(&squad)
            .map_err(|_| AppError::Internal("Squad will not serialise".into()))?;
        let seed = if req.daily { daily_seed(today) } else { Uuid::new_v4().as_u128() as i64 };

        let run = match self
            .repo
            .insert_battle_run(user_id, route.id, waves_total, &written, seed, req.daily.then_some(today))
            .await?
        {
            Some(run) => run,
            // Два запроса подряд: второй находит то, что завёл первый.
            None => self
                .repo
                .find_active_battle_run(user_id)
                .await?
                .ok_or_else(|| AppError::Internal("Run will not start".into()))?,
        };
        Self::log_domain_event("battle_run_started", "battle_run", run.id, "ok");
        self.run_dto(&run).await
    }

    /// Законченные партии забега, чтобы игрок мог посмотреть, как прошла волна.
    pub async fn list_run_waves(&self, user_id: Uuid, id: Uuid) -> Result<Vec<WaveLogDto>> {
        self.repo.get_battle_run(id, user_id).await?;
        Ok(wave_log(self.repo.list_run_finished_matches(id).await?))
    }

    /// Слаг маршрута дня; нет опубликованных маршрутов — отказ.
    async fn daily_route_slug(&self, day: chrono::NaiveDate) -> Result<String> {
        let slugs: Vec<String> = self
            .repo
            .list_published_battle_routes()
            .await?
            .into_iter()
            .map(|(r, _)| r.slug)
            .collect();
        daily_pick(slugs, day).ok_or_else(|| AppError::NotFound("No such route".into()))
    }

    /// Маршрут дня и общий счёт; `None` — режим выключен или маршрутов нет.
    pub async fn get_battle_daily(&self, user_id: Uuid) -> Result<Option<DailyDto>> {
        if !self.get_run_settings().await?.enabled {
            return Ok(None);
        }
        let today = chrono::Utc::now().date_naive();
        let Ok(slug) = self.daily_route_slug(today).await else {
            return Ok(None);
        };
        let route = self.repo.get_battle_route_by_slug(&slug).await?;
        let waves = self.repo.count_battle_route_waves(route.id).await? as i16;
        let (started, won) = self.repo.daily_battle_run_counts(today).await?;
        let mine = self.repo.find_daily_battle_run(user_id, today).await?.map(|r| DailyMineDto {
            wave_no: r.wave_no,
            waves_total: r.waves_total,
            state: r.state,
        });
        Ok(Some(DailyDto {
            date: today.to_string(),
            route: Self::route_dto(&route, waves),
            started,
            won,
            mine,
        }))
    }

    pub async fn current_battle_run(&self, user_id: Uuid) -> Result<Option<RunDto>> {
        // Идущий забег, а если его нет — последний законченный: итог и трофеи
        // не пропадают от перезагрузки страницы.
        if let Some(run) = self.repo.find_active_battle_run(user_id).await? {
            let run = self.reconcile_run(run).await?;
            return Ok(Some(self.run_dto(&run).await?));
        }
        match self.repo.find_latest_battle_run(user_id).await? {
            Some(run) => Ok(Some(self.run_dto(&run).await?)),
            None => Ok(None),
        }
    }

    pub async fn read_battle_run(&self, user_id: Uuid, id: Uuid) -> Result<RunDto> {
        let run = self.repo.get_battle_run(id, user_id).await?;
        let run = self.reconcile_run(run).await?;
        self.run_dto(&run).await
    }

    /// Волна кончилась, а перенос итога в забег не дошёл (сбой между двумя
    /// записями): догоняет его. Ход, которым партия кончилась, второй раз не
    /// проходит, поэтому без этого забег застрял бы на сыгранной волне.
    async fn reconcile_run(&self, run: BattleRun) -> Result<BattleRun> {
        if run.state != STATE_ACTIVE || self.repo.find_open_run_match(run.id).await?.is_some() {
            return Ok(run);
        }
        if let Some(rec) = self.repo.find_unsettled_run_match(run.id, run.wave_no).await? {
            let state = Self::match_state(&rec)?;
            if state.outcome.is_some() && self.settle_run_wave(&rec, &state).await?.is_some() {
                return self.repo.get_battle_run(run.id, run.user_id).await;
            }
        }
        Ok(run)
    }

    /// Сдать забег: он кончается проигрышем, и можно начать новый.
    pub async fn abandon_battle_run(&self, user_id: Uuid, id: Uuid) -> Result<RunDto> {
        self.repo.get_battle_run(id, user_id).await?;
        // Открытая волна закрывается вместе с забегом.
        if let Some(open) = self.repo.find_open_run_match(id).await? {
            let mut state = Self::match_state(&open)?;
            state.outcome = Some(battle_core::Outcome::Keeper);
            let cache = serde_json::to_string(&state)
                .map_err(|_| AppError::Internal("Board will not serialise".into()))?;
            self.repo.yield_battle_match(open.id, user_id, &cache).await?;
        }
        self.repo.end_battle_run(id, user_id, STATE_LOST).await?;
        self.read_battle_run(user_id, id).await
    }

    /// Начать волну забега — или вернуть открытую.
    pub async fn begin_run_wave(
        &self,
        user_id: Uuid,
        id: Uuid,
        req: BeginWaveRequest,
    ) -> Result<BattleMatchDto> {
        let run = self.repo.get_battle_run(id, user_id).await?;
        let run = self.reconcile_run(run).await?;
        if run.state != STATE_ACTIVE {
            return Err(AppError::BadRequest("run:over".into()));
        }
        if let Some(open) = self.repo.find_open_run_match(run.id).await? {
            let state = Self::match_state(&open)?;
            return Ok(Self::match_dto(&open, state, Vec::new(), 0));
        }
        if run.phase == PHASE_REWARD {
            return Err(AppError::BadRequest("run:pickReward".into()));
        }
        self.assert_runs_are_on().await?;
        let squad = read_squad(&run.squad);
        self.start_wave_match(user_id, &run, squad, req).await
    }

    /// Тренировка: проигранную волну можно сыграть снова, с другой расстановкой.
    /// Партия привязана к проигранному забегу, поэтому её итог (`settle_run_wave`)
    /// забег не двигает, пыль и трофеи не пишет: забег не меняется.
    pub async fn practice_run_wave(
        &self,
        user_id: Uuid,
        id: Uuid,
        req: BeginWaveRequest,
    ) -> Result<BattleMatchDto> {
        let run = self.repo.get_battle_run(id, user_id).await?;
        if run.state != STATE_LOST {
            return Err(AppError::BadRequest("run:notOver".into()));
        }
        if let Some(open) = self.repo.find_open_run_match(run.id).await? {
            let state = Self::match_state(&open)?;
            return Ok(Self::match_dto(&open, state, Vec::new(), 0));
        }
        self.assert_runs_are_on().await?;
        let squad = self.practice_squad(&run).await?;
        self.start_wave_match(user_id, &run, squad, req).await
    }

    /// Отряд на начало последней сыгранной волны: карты, что выходили, с тем
    /// здоровьем, с каким вышли (оно записано в замороженной расстановке партии);
    /// остальные — как они лежат в забеге.
    async fn practice_squad(&self, run: &BattleRun) -> Result<Vec<SquadCard>> {
        let squad = read_squad(&run.squad);
        let Some(rec) = self.repo.find_unsettled_run_match(run.id, run.wave_no).await? else {
            return Ok(squad);
        };
        let fielded: Vec<Uuid> = rec
            .run_cards
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_default();
        let health = serde_json::from_str::<battle_core::Setup>(&rec.setup)
            .map(|setup| setup.player_health)
            .unwrap_or_default();
        Ok(squad_at_wave_start(squad, &fielded, &health))
    }

    /// Собрать волну и завести партию. Общее для забега и тренировки.
    async fn start_wave_match(
        &self,
        user_id: Uuid,
        run: &BattleRun,
        squad: Vec<SquadCard>,
        req: BeginWaveRequest,
    ) -> Result<BattleMatchDto> {
        let wave = self.repo.get_battle_route_wave(run.route_id, run.wave_no).await?;
        let etude_cards = self.cards_by_slug().await?;
        let shelf = self.list_battle_cards_public().await?;
        let base = self.wave_base(&wave, &etude_cards).await?;
        let mut setup = base.setup;

        // Отряд: живые карты, и выходят те, что названы, а иначе — сколько
        // позволяет волна.
        let alive: Vec<&SquadCard> = squad.iter().filter(|c| !c.dead).collect();
        if alive.is_empty() {
            self.repo.end_battle_run(run.id, user_id, STATE_LOST).await?;
            return Err(AppError::BadRequest("run:over".into()));
        }
        let max = wave.max_cards as usize;
        let min = (wave.min_cards as usize).min(alive.len());
        let chosen: Vec<(Uuid, Option<(u8, u8)>)> = if req.placements.is_empty() {
            alive.iter().take(max).map(|c| (c.card_id, None)).collect()
        } else {
            let mut seen = std::collections::HashSet::new();
            let mut out = Vec::new();
            for p in &req.placements {
                if !seen.insert(p.card_id) || !alive.iter().any(|c| c.card_id == p.card_id) {
                    return Err(AppError::BadRequest("run:badSquad".into()));
                }
                out.push((p.card_id, p.x.zip(p.y)));
            }
            out
        };
        if chosen.len() > max {
            return Err(AppError::BadRequest("run:tooMany".into()));
        }
        if chosen.len() < min {
            return Err(AppError::BadRequest("run:tooFew".into()));
        }

        let laid = lay_out(&base.ground, &chosen).map_err(|w| AppError::BadRequest(format!("run:{w}")))?;

        let by_id: std::collections::HashMap<Uuid, &BattleCardDto> =
            shelf.iter().filter_map(|c| c.id.parse().ok().map(|id| (id, c))).collect();
        let by_slug: std::collections::HashMap<&str, &BattleCardDto> =
            shelf.iter().map(|c| (c.slug.as_str(), c)).collect();
        let mut fielded = Vec::new();
        for (card_id, (x, y)) in &laid {
            let card = by_id
                .get(card_id)
                .ok_or_else(|| AppError::BadRequest("run:badSquad".into()))?;
            let snap = crate::battles::to_snapshot_with(card, |want| by_slug.get(want).copied());
            let carried = squad
                .iter()
                .find(|c| c.card_id == *card_id)
                .and_then(|c| c.health)
                .unwrap_or(snap.health);
            let cell = battle_core::Cell::new(*x, *y)
                .ok_or_else(|| AppError::BadRequest("run:cellBlocked".into()))?;
            setup.player_health.push(carried);
            setup.player_board.push((snap, cell));
            fielded.push(*card_id);
        }

        setup.player_board.extend(base.structures);
        let frozen = serde_json::to_string(&setup)
            .map_err(|_| AppError::Internal("Arrangement will not freeze".into()))?;
        let rules = base.rules;
        let written_rules = serde_json::to_string(&rules)
            .map_err(|_| AppError::Internal("Rules will not freeze".into()))?;
        let state = battle_core::MatchState::begin_with(setup, rules);
        let cache = serde_json::to_string(&state)
            .map_err(|_| AppError::Internal("Board will not serialise".into()))?;
        let cards_written = serde_json::to_string(&fielded)
            .map_err(|_| AppError::Internal("Squad will not serialise".into()))?;
        let version = shelf.iter().map(|c| c.rules_version).max().unwrap_or(1);

        let Some(rec) = self
            .repo
            .insert_run_match(
                user_id,
                run.id,
                run.wave_no,
                &cards_written,
                &frozen,
                version,
                &cache,
                &written_rules,
            )
            .await?
        else {
            // Два нажатия подряд: вторая волна не заводится, возвращается первая.
            let open = self
                .repo
                .find_open_run_match(run.id)
                .await?
                .ok_or_else(|| AppError::Internal("Wave will not start".into()))?;
            let state = Self::match_state(&open)?;
            return Ok(Self::match_dto(&open, state, Vec::new(), 0));
        };
        Self::log_domain_event("battle_wave_started", "battle_match", rec.id, "ok");
        Ok(Self::match_dto(&rec, state, Vec::new(), 0))
    }

    /// Итог волны: здоровье возвращается в отряд, забег идёт дальше или кончается.
    /// Зовётся один раз, в тот ход, которым партия кончилась. `None` — партия не
    /// волна или забег уже ушёл дальше.
    pub(super) async fn settle_run_wave(
        &self,
        rec: &BattleMatch,
        state: &battle_core::MatchState,
    ) -> Result<Option<RunDto>> {
        let (Some(run_id), Some(wave_no)) = (rec.run_id, rec.run_wave) else {
            return Ok(None);
        };
        let Some(outcome) = state.outcome else {
            return Ok(None);
        };
        let Some(run) = self.repo.get_battle_run_by_id(run_id).await? else {
            return Ok(None);
        };
        if run.state != STATE_ACTIVE || run.wave_no != wave_no {
            return Ok(None);
        }

        let fielded: Vec<Uuid> = rec
            .run_cards
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_default();
        let mut squad = read_squad(&run.squad);
        carry(state, &fielded, &mut squad);
        if let Ok(started) = serde_json::from_str::<battle_core::Setup>(&rec.setup) {
            remember_cells(&started, &fielded, &mut squad);
        }

        // Ничья волной не берётся: выигрывает только та сторона, что победила.
        let won_wave = outcome == battle_core::Outcome::Player;
        let (next_wave, next_state) = after_wave(won_wave, wave_no, run.waves_total);
        let settings = self.get_run_settings().await?;
        let guest = self.repo.user_is_guest(run.user_id).await?;
        let wave = self.repo.get_battle_route_wave(run.route_id, wave_no).await?;
        let shelf = self.list_battle_cards_public().await?;

        // Пыль и трофеи пишутся ДО перехода забега: у обеих записей свой ключ,
        // и повтор после сбоя ничего не удвоит, а переход, записанный раньше
        // них, при сбое их бы потерял.
        let mut paid_dust = 0;
        if !guest {
            let dust = settings.wave_dust(wave.reward_dust, won_wave);
            paid_dust = self
                .repo
                .credit_run_dust(
                    run.user_id,
                    dust,
                    "run_wave",
                    run.id,
                    &format!("run:{}:wave:{}", run.id, wave_no),
                    settings.daily_dust_cap,
                )
                .await?;
            let mut trophies: Vec<(Uuid, i32)> = Vec::new();
            for u in state.units.iter().filter(|u| u.owner == battle_core::Side::Keeper) {
                if !u.health.is_dead() || u.card.structure {
                    continue;
                }
                // Трофей записывается только за карту, которую вообще можно взять: с
                // полки. Враг, которого автор не опубликовал, трофеем не станет,
                // иначе игрок видит карточку, которую взять нельзя.
                let Some(card) = shelf.iter().find(|c| c.slug == u.card.name) else {
                    continue;
                };
                let Ok(card_id) = card.id.parse::<Uuid>() else { continue };
                if trophies.iter().all(|(id, _)| *id != card_id) {
                    trophies.push((card_id, settings.trophy_price(&card.slug, card.tier)));
                }
            }
            self.repo.insert_run_trophies(run.id, wave_no, &trophies).await?;
        }

        let max_health: std::collections::HashMap<Uuid, i32> = shelf
            .iter()
            .filter_map(|c| c.id.parse().ok().map(|id| (id, c.health as i32)))
            .collect();
        let max_of = |id: Uuid| max_health.get(&id).copied().unwrap_or(1);
        let (phase, options) = if won_wave && next_state == STATE_ACTIVE {
            let room = if guest { 0 } else { self.repo.run_dust_room(run.user_id, settings.daily_dust_cap).await? };
            let opts = reward_options(run.seed, wave_no, &squad, max_of, guest, room, &settings);
            if opts.is_empty() {
                (PHASE_WAVE, None)
            } else {
                let raw = serde_json::to_string(&opts)
                    .map_err(|_| AppError::Internal("Reward will not serialise".into()))?;
                (PHASE_REWARD, Some(raw))
            }
        } else {
            (PHASE_WAVE, None)
        };

        let written = serde_json::to_string(&squad)
            .map_err(|_| AppError::Internal("Squad will not serialise".into()))?;
        let Some(saved) = self
            .repo
            .settle_battle_run(run.id, wave_no, next_wave, next_state, &written, phase, options.as_deref())
            .await?
        else {
            return Ok(None);
        };
        Self::log_domain_event(
            match next_state {
                STATE_LOST => "battle_run_lost",
                STATE_WON => "battle_run_won",
                _ => "battle_wave_won",
            },
            "battle_run",
            saved.id,
            "ok",
        );
        let mut dto = self.run_dto(&saved).await?;
        dto.paid_dust = paid_dust;
        Ok(Some(dto))
    }

    /// Выбрать награду из трёх.
    pub async fn pick_run_reward(&self, user_id: Uuid, id: Uuid, req: PickRewardRequest) -> Result<RunDto> {
        let run = self.repo.get_battle_run(id, user_id).await?;
        // Фаза уже `wave` — выбор был сделан (повтор): отвечаем тем, что есть.
        if run.state != STATE_ACTIVE || run.phase != PHASE_REWARD {
            return self.run_dto(&run).await;
        }
        let options: Vec<RewardOption> = run
            .reward_options
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_default();
        let option = options
            .get(req.index)
            .ok_or_else(|| AppError::BadRequest("run:noSuchReward".into()))?;

        let shelf = self.list_battle_cards_public().await?;
        let max_health: std::collections::HashMap<Uuid, i32> = shelf
            .iter()
            .filter_map(|c| c.id.parse().ok().map(|id| (id, c.health as i32)))
            .collect();
        let mut squad = read_squad(&run.squad);
        let applied = apply_reward(option, &mut squad, |id| max_health.get(&id).copied().unwrap_or(1));

        let settings = self.get_run_settings().await?;
        let dust = (applied.dust > 0 && !self.repo.user_is_guest(user_id).await?)
            .then_some((applied.dust, settings.daily_dust_cap));
        let written = serde_json::to_string(&squad)
            .map_err(|_| AppError::Internal("Squad will not serialise".into()))?;
        // Пыль награды пишется в той же транзакции, что и смена фазы: награда
        // выдаётся один раз, что бы ни выбрали два запроса разом.
        let run = match self
            .repo
            .finish_run_reward(
                run.id,
                user_id,
                &run.squad,
                &written,
                applied.free_swaps,
                applied.heal_charges,
                dust,
                (reward_kind_name(option.kind), option.amount),
            )
            .await?
        {
            Some(run) => run,
            None => self.repo.get_battle_run(id, user_id).await?,
        };
        self.run_dto(&run).await
    }

    /// Потратить заряд лечения на названную карту: полное здоровье. Между волнами;
    /// гостю лечение положено так же, как награда, из которой оно взялось.
    pub async fn heal_run_card(&self, user_id: Uuid, id: Uuid, req: HealRequest) -> Result<RunDto> {
        let run = self.repo.get_battle_run(id, user_id).await?;
        let run = self.reconcile_run(run).await?;
        if run.state != STATE_ACTIVE {
            return Err(AppError::BadRequest("run:over".into()));
        }
        if self.repo.find_open_run_match(run.id).await?.is_some() {
            return Err(AppError::BadRequest("run:waveOpen".into()));
        }
        // Зарядов нет: повтор уже сделанного нажатия, отвечаем тем, что есть.
        if self.repo.run_heal_charges(run.id).await? == 0 {
            return self.run_dto(&run).await;
        }
        let shelf = self.list_battle_cards_public().await?;
        let max_health: std::collections::HashMap<Uuid, i32> = shelf
            .iter()
            .filter_map(|c| c.id.parse().ok().map(|id| (id, c.health as i32)))
            .collect();
        let mut squad = read_squad(&run.squad);
        if !heal_card(&mut squad, req.card_id, |id| max_health.get(&id).copied().unwrap_or(1)) {
            return Err(AppError::BadRequest("run:noNeedHeal".into()));
        }
        let written = serde_json::to_string(&squad)
            .map_err(|_| AppError::Internal("Squad will not serialise".into()))?;
        let saved = match self.repo.heal_run_card(run.id, user_id, &run.squad, &written).await? {
            Some(saved) => saved,
            None => self.repo.get_battle_run(id, user_id).await?,
        };
        Self::log_domain_event("battle_heal", "battle_run", saved.id, "ok");
        self.run_dto(&saved).await
    }

    /// Замена карты отряда на случайную из собрания, не стоящую в отряде.
    /// Между волнами и за пыль; случай считает сервер по зерну забега.
    pub async fn swap_run_card(&self, user_id: Uuid, id: Uuid, req: SwapRequest) -> Result<RunDto> {
        if self.repo.user_is_guest(user_id).await? {
            return Err(AppError::BadRequest("run:noSwapForGuest".into()));
        }
        let run = self.repo.get_battle_run(id, user_id).await?;
        let run = self.reconcile_run(run).await?;
        if run.state != STATE_ACTIVE {
            return Err(AppError::BadRequest("run:over".into()));
        }
        if self.repo.find_open_run_match(run.id).await?.is_some() {
            return Err(AppError::BadRequest("run:waveOpen".into()));
        }
        // Повтор того же запроса: замена уже сделана, отвечаем тем, что есть.
        if req.swaps != run.swaps {
            return self.run_dto(&run).await;
        }
        let mut squad = read_squad(&run.squad);
        let Some(at) = squad.iter().position(|c| c.card_id == req.card_id) else {
            return Err(AppError::BadRequest("run:badSquad".into()));
        };

        let bench = self.bench_of(user_id, &squad).await?;
        let Some(chosen) = pick_replacement(run.seed, run.swaps, &bench) else {
            return Err(AppError::BadRequest("run:noBench".into()));
        };
        // Новая карта встаёт на место прежней.
        let seat = squad[at].cell;
        squad[at] = SquadCard { card_id: chosen, health: None, dead: false, loaned: false, cell: seat };

        let settings = self.get_run_settings().await?;
        let cost = settings.swap_cost(run.swaps, run.free_swaps);
        let written = serde_json::to_string(&squad)
            .map_err(|_| AppError::Internal("Squad will not serialise".into()))?;
        let saved = match self
            .repo
            .swap_run_card(run.id, user_id, run.swaps, &run.squad, &written, cost, run.free_swaps > 0)
            .await?
        {
            Some(saved) => saved,
            None => self.repo.get_battle_run(id, user_id).await?,
        };
        Self::log_domain_event("battle_swap", "battle_run", saved.id, "ok");
        self.run_dto(&saved).await
    }

    /// Взять побеждённого врага на полку.
    pub async fn take_run_trophy(&self, user_id: Uuid, id: Uuid, req: TakeTrophyRequest) -> Result<RunDto> {
        if self.repo.user_is_guest(user_id).await? {
            return Err(AppError::BadRequest("run:noTrophyForGuest".into()));
        }
        let run = self.repo.get_battle_run(id, user_id).await?;
        // Купленная карта встаёт в отряд, если есть место и волна не идёт:
        // иначе она лежит на полке и годится для замены.
        let between_waves = self.repo.find_open_run_match(run.id).await?.is_none();
        let card_id = req.card_id;
        let join = |old: &str| -> Option<String> {
            if !between_waves {
                return None;
            }
            crate::runs::squad_with_trophy(old, card_id)
        };
        if self.repo.take_run_trophy(run.id, user_id, card_id, join).await? {
            Self::log_domain_event("battle_trophy", "battle_run", run.id, "ok");
        }
        // Отряд мог пополниться купленной картой: отвечаем тем, что записано.
        let run = self.repo.get_battle_run(id, user_id).await?;
        self.run_dto(&run).await
    }

    /// Глубина руки врага в этой партии: у волны — как у этюда-основы.
    pub(super) async fn match_hand_depth(&self, rec: &BattleMatch) -> Result<u8> {
        if let (Some(run_id), Some(wave_no)) = (rec.run_id, rec.run_wave) {
            if let Some(run) = self.repo.get_battle_run_by_id(run_id).await? {
                let wave = self.repo.get_battle_route_wave(run.route_id, wave_no).await?;
                return self.keeper_hand_depth(Some(wave.challenge_id)).await;
            }
        }
        self.keeper_hand_depth(rec.challenge_id).await
    }

    /// Что игрок видит о волне до её начала: условие, число карт, враги, поле.
    async fn wave_preview(&self, route_id: Uuid, wave_no: i16) -> Result<WavePreviewDto> {
        let wave = self.repo.get_battle_route_wave(route_id, wave_no).await?;
        let board: Vec<String> = match self.repo.get_battle_challenge(wave.challenge_id).await {
            Ok(base) => serde_json::from_str::<ChallengeSetup>(&base.setup)
                .map(|setup| setup.keeper_board.into_iter().map(|p| p.card).collect())
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        let entries: Vec<crate::runs::WaveEntry> =
            serde_json::from_str(&wave.reinforcements).unwrap_or_default();
        let enemies = crate::runs::wave_enemy_slugs(board, &entries);
        let field = self.wave_field(&wave).await;
        Ok(WavePreviewDto {
            wave_no: wave.wave_no,
            min_cards: wave.min_cards,
            max_cards: wave.max_cards,
            goal: read_goal(wave.goal.as_deref()).ok().flatten(),
            race_hint: wave.race_hint,
            reward_dust: wave.reward_dust,
            enemies,
            field,
        })
    }

    async fn run_dto(&self, run: &BattleRun) -> Result<RunDto> {
        let route = self.repo.get_battle_route(run.route_id).await?;
        let waves = self.repo.count_battle_route_waves(route.id).await? as i16;
        let shelf = self.list_battle_cards_public().await?;
        let squad = squad_dtos(read_squad(&run.squad), &shelf);
        let match_id = self.repo.find_open_run_match(run.id).await?.map(|m| m.id.to_string());
        let guest = self.repo.user_is_guest(run.user_id).await?;
        let settings = self.get_run_settings().await?;
        let etude_only = self.list_battle_cards_etude_only().await?;
        let trophy_rows = self.repo.list_run_trophies(run.id).await?;
        let owned = if guest {
            std::collections::HashSet::new()
        } else {
            let ids: Vec<Uuid> = trophy_rows.iter().map(|t| t.card_id).collect();
            self.repo.owned_card_ids(run.user_id, &ids).await?
        };
        let trophies = trophy_rows
            .into_iter()
            .map(|t| TrophyDto {
                owned: owned.contains(&t.card_id),
                on_shelf: shelf.iter().any(|s| s.id.parse().ok() == Some(t.card_id)),
                card_id: t.card_id.to_string(),
                slug: shelf
                    .iter()
                    .chain(&etude_only)
                    .find(|c| c.id.parse().ok() == Some(t.card_id))
                    .map(|c| c.slug.clone())
                    .unwrap_or_default(),
                wave_no: t.wave_no,
                price_dust: t.price_dust,
                taken: t.taken,
            })
            .collect();
        let picks = self
            .repo
            .list_run_picks(run.id)
            .await?
            .into_iter()
            .map(|(wave_no, kind, amount)| PickDto { wave_no, kind, amount })
            .collect();
        let bench = if guest {
            0
        } else {
            self.bench_of(run.user_id, &read_squad(&run.squad)).await?.len() as i32
        };
        let reward: Vec<RewardOption> = if run.phase == PHASE_REWARD {
            run.reward_options
                .as_deref()
                .and_then(|raw| serde_json::from_str(raw).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let next_wave = if run.state == STATE_ACTIVE {
            Some(self.wave_preview(run.route_id, run.wave_no).await?)
        } else {
            None
        };
        // Проигранный забег можно переиграть: волна и отряд на её начало.
        let practice = if run.state == STATE_LOST && settings.enabled {
            match self.wave_preview(run.route_id, run.wave_no).await {
                Ok(wave) => Some(PracticeDto {
                    wave,
                    squad: squad_dtos(self.practice_squad(run).await?, &shelf),
                }),
                Err(_) => None,
            }
        } else {
            None
        };
        Ok(RunDto {
            id: run.id.to_string(),
            route: Self::route_dto(&route, waves),
            wave_no: run.wave_no,
            waves_total: run.waves_total,
            state: run.state.clone(),
            squad,
            swaps: run.swaps,
            match_id,
            next_wave,
            phase: run.phase.clone(),
            reward,
            swap_cost: (!guest).then(|| settings.swap_cost(run.swaps, run.free_swaps)),
            free_swaps: run.free_swaps,
            heal_charges: self.repo.run_heal_charges(run.id).await?,
            trophies,
            picks,
            bench,
            paid_dust: 0,
            practice,
        })
    }
}

/// Отряд на начало волны: карты, что выходили, встают с тем здоровьем, с каким
/// вышли (`health` идёт в порядке `fielded`), даже если волна их убила; остальные
/// остаются как есть, и павшие раньше остаются павшими.
pub(crate) fn squad_at_wave_start(mut squad: Vec<SquadCard>, fielded: &[Uuid], health: &[i32]) -> Vec<SquadCard> {
    for (i, card_id) in fielded.iter().enumerate() {
        if let Some(card) = squad.iter_mut().find(|c| c.card_id == *card_id) {
            card.dead = false;
            card.health = health.get(i).copied();
        }
    }
    squad
}

/// Отряд для ответа: здоровье и имя по полке.
fn squad_dtos(squad: Vec<SquadCard>, shelf: &[BattleCardDto]) -> Vec<SquadCardDto> {
    squad
        .into_iter()
        .map(|c| {
            let card = shelf.iter().find(|s| s.id.parse().ok() == Some(c.card_id));
            let max_health = card.map(|s| s.health as i32).unwrap_or(0);
            SquadCardDto {
                card_id: c.card_id.to_string(),
                slug: card.map(|s| s.slug.clone()).unwrap_or_default(),
                health: if c.dead { 0 } else { c.health.unwrap_or(max_health) },
                max_health,
                dead: c.dead,
                loaned: c.loaned,
                cell: c.cell.and_then(|(x, y)| battle_core::Cell::new(x, y)),
            }
        })
        .collect()
}

const SLUG_MAX: usize = 60;

/// Проверка маршрута, как его правит стол. Отказ словами: автор должен узнать,
/// что не легло, до того как игрок увидит маршрут без волны.
fn check_route(route: &AdminRoute) -> std::result::Result<(), &'static str> {
    let slug_ok = !route.slug.is_empty()
        && route.slug.len() <= SLUG_MAX
        && route.slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !slug_ok {
        return Err("The slug is lowercase letters, digits and dashes");
    }
    if route.title_en.trim().is_empty() || route.title_ru.trim().is_empty() {
        return Err("The route needs a title in English and Russian");
    }
    if !["draft", "published", "retired"].contains(&route.status.as_str()) {
        return Err("Unknown status");
    }
    if route.waves.is_empty() || route.waves.len() > 12 {
        return Err("A route has from 1 to 12 waves");
    }
    for (i, w) in route.waves.iter().enumerate() {
        if w.wave_no != i as i16 + 1 {
            return Err("Waves are numbered 1, 2, 3 … without gaps");
        }
        if w.min_cards < 1 || w.max_cards > 6 || w.min_cards > w.max_cards {
            return Err("Cards in a wave: from 1 to 6, minimum not above maximum");
        }
        if !(0..=1000).contains(&w.reward_dust) {
            return Err("Wave reward is from 0 to 1000 dust");
        }
        if w.reinforcements.len() > 40 || w.reinforcements.iter().any(|e| e.round == 0 || e.round > 30) {
            return Err("Reinforcement rounds are from 1 to 30, at most 40 entries");
        }
        if let Some(battle_core::WaveGoal::HoldRounds { rounds }) = &w.goal {
            if *rounds == 0 || *rounds > 30 {
                return Err("Hold rounds are from 1 to 30");
            }
        }
    }
    if let Some(n) = route.guest_waves {
        if n < 1 || n as usize > route.waves.len() {
            return Err("Guest waves are from 1 to the number of waves");
        }
    }
    Ok(())
}

impl AppService {
    /// Что у маршрута не доделано: про каждую волну и каждого врага. Строка —
    /// `код` или `код:слаг`, вкладка переводит их словами.
    async fn route_warnings(
        &self,
        route: &AdminRoute,
        etude_cards: &std::collections::BTreeMap<String, BattleCardDto>,
        settings: &RunSettings,
    ) -> Result<Vec<String>> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for w in &route.waves {
            let challenge = self.repo.get_battle_challenge(w.challenge_id).await?;
            let arrangement: ChallengeSetup = serde_json::from_str(&challenge.setup)
                .map_err(|_| AppError::Internal("Wave arrangement will not read".into()))?;
            let mut enemies: Vec<String> = arrangement.keeper_board.iter().map(|p| p.card.clone()).collect();
            enemies.extend(arrangement.keeper_hand.iter().cloned());
            enemies.extend(w.reinforcements.iter().map(|e| e.card.clone()));
            if let Some(battle_core::WaveGoal::KillTarget { name }) = &w.goal {
                if !enemies.iter().any(|e| e == name) {
                    out.push(format!("noBoss:{}:{name}", w.wave_no));
                }
            }
            if let Some(battle_core::WaveGoal::DefendStructure) = &w.goal {
                let has = arrangement.player_board.iter().any(|p| {
                    etude_cards.get(&p.card).is_some_and(|c| c.kind == "structure")
                });
                if !has {
                    out.push(format!("defendNeedsStructure:{}", w.wave_no));
                }
            }
            // Подкрепление на клетке, где стоит строение игрока, никогда не выйдет:
            // враг ждёт, пока клетка освободится, и партия не кончается по картам.
            let blocked = |x: u8, y: u8| {
                arrangement.player_board.iter().any(|p| {
                    p.x == x && p.y == y && etude_cards.get(&p.card).is_some_and(|c| c.kind == "structure")
                })
            };
            for e in &w.reinforcements {
                if blocked(e.x, e.y) {
                    out.push(format!("entranceBlocked:{}:{}", w.wave_no, e.card));
                }
            }
            if w.reinforcements.is_empty() && w.goal.is_none() {
                out.push(format!("plainWave:{}", w.wave_no));
            }
            for slug in enemies {
                let Some(card) = etude_cards.get(&slug) else { continue };
                if card.kind == "structure" || !seen.insert(slug.clone()) {
                    continue;
                }
                if card.status != "published" {
                    out.push(format!("trophyNotPublished:{slug}"));
                } else if settings.trophy_price(&slug, card.tier) == 0 {
                    out.push(format!("trophyNoPrice:{slug}"));
                }
            }
        }
        Ok(out)
    }

    async fn admin_route_dto(
        &self,
        route: BattleRoute,
        waves: Vec<BattleRouteWave>,
        etude_cards: &std::collections::BTreeMap<String, BattleCardDto>,
        settings: &RunSettings,
    ) -> Result<AdminRoute> {
        let mut dto = AdminRoute {
            id: Some(route.id),
            slug: route.slug,
            title_en: route.title_en,
            title_ru: route.title_ru,
            title_uk: route.title_uk,
            title_es: route.title_es,
            title_de: route.title_de,
            title_it: route.title_it,
            guest_waves: route.guest_waves,
            status: route.status,
            sort_order: route.sort_order,
            waves: waves
                .into_iter()
                .map(|w| AdminWave {
                    wave_no: w.wave_no,
                    challenge_id: w.challenge_id,
                    reinforcements: serde_json::from_str(&w.reinforcements).unwrap_or_default(),
                    goal: read_goal(w.goal.as_deref()).ok().flatten(),
                    min_cards: w.min_cards,
                    max_cards: w.max_cards,
                    reward_dust: w.reward_dust,
                    race_hint: w.race_hint,
                })
                .collect(),
            warnings: Vec::new(),
        };
        dto.warnings = self.route_warnings(&dto, etude_cards, settings).await?;
        Ok(dto)
    }

    pub async fn admin_list_battle_routes(&self) -> Result<Vec<AdminRoute>> {
        let etude_cards = self.cards_by_slug().await?;
        let settings = self.get_run_settings().await?;
        let mut out = Vec::new();
        for (route, waves) in self.repo.list_all_battle_routes().await? {
            out.push(self.admin_route_dto(route, waves, &etude_cards, &settings).await?);
        }
        Ok(out)
    }

    pub async fn admin_save_battle_route(&self, route: AdminRoute) -> Result<AdminRoute> {
        check_route(&route).map_err(|e| AppError::BadRequest(e.into()))?;
        // Идущие забеги читают волны по номеру: пока они есть, маршрут можно
        // править, но не укорачивать — игрок остался бы на волне, которой нет.
        if let Some(id) = route.id {
            let have = self.repo.count_battle_route_waves(id).await?;
            if (route.waves.len() as i64) < have && self.repo.count_active_route_runs(id).await? > 0 {
                return Err(AppError::BadRequest(
                    "The route has active runs: waves cannot be removed".into(),
                ));
            }
        }
        let etude_cards = self.cards_by_slug().await?;
        // Каждая волна собирается по-настоящему: поле, враги и расписание,
        // которые не читаются, отказываются здесь, а не у игрока.
        let mut rows = Vec::new();
        for w in &route.waves {
            let row = BattleRouteWave {
                id: Uuid::nil(),
                route_id: Uuid::nil(),
                wave_no: w.wave_no,
                challenge_id: w.challenge_id,
                reinforcements: serde_json::to_string(&w.reinforcements)
                    .map_err(|_| AppError::Internal("Schedule will not serialise".into()))?,
                goal: w
                    .goal
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()
                    .map_err(|_| AppError::Internal("Goal will not serialise".into()))?,
                min_cards: w.min_cards,
                max_cards: w.max_cards,
                reward_dust: w.reward_dust,
                race_hint: w.race_hint.clone().filter(|s| !s.trim().is_empty()),
            };
            self.wave_base(&row, &etude_cards).await?;
            rows.push((
                row.wave_no,
                row.challenge_id,
                row.reinforcements,
                row.goal,
                row.min_cards,
                row.max_cards,
                row.reward_dust,
                row.race_hint,
            ));
        }
        let id = self.repo.save_battle_route(route.id, &route, &rows).await?;
        let settings = self.get_run_settings().await?;
        let (saved, waves) = self
            .repo
            .list_all_battle_routes()
            .await?
            .into_iter()
            .find(|(r, _)| r.id == id)
            .ok_or_else(|| AppError::Internal("Route will not read back".into()))?;
        self.admin_route_dto(saved, waves, &etude_cards, &settings).await
    }

    pub async fn admin_delete_battle_route(&self, id: Uuid) -> Result<()> {
        self.repo.delete_battle_route(id).await
    }

    /// Проба маршрута ботами в фоне: три ступени отрядов (слабейшие, средний,
    /// сильный) по полке карт, на которых можно выйти в бой.
    pub async fn admin_start_route_probe(&self, route_id: Uuid) -> Result<Uuid> {
        let (_, waves) = self
            .repo
            .list_all_battle_routes()
            .await?
            .into_iter()
            .find(|(r, _)| r.id == route_id)
            .ok_or_else(|| AppError::NotFound("No such route".into()))?;
        if waves.is_empty() {
            return Err(AppError::BadRequest("The route has no waves".into()));
        }
        let etude_cards = self.cards_by_slug().await?;
        let shelf = self.list_battle_cards_public().await?;
        let mut probe_waves = Vec::new();
        for w in &waves {
            let base = self.wave_base(w, &etude_cards).await?;
            probe_waves.push(crate::runs_probe::ProbeWave {
                base: base.setup,
                rules: base.rules,
                depth: base.depth,
                max_cards: w.max_cards as usize,
                structures: base.structures,
                ground: base.ground,
            });
        }
        let by_slug: std::collections::HashMap<&str, &BattleCardDto> =
            shelf.iter().map(|c| (c.slug.as_str(), c)).collect();
        let mut fieldable: Vec<&BattleCardDto> =
            shelf.iter().filter(|c| crate::battles::can_take_the_field(c)).collect();
        if fieldable.len() < LOAN_CARDS {
            return Err(AppError::BadRequest("Not enough cards on the shelf to probe".into()));
        }
        fieldable.sort_by(|a, b| (strength(a), a.tier, &a.id).cmp(&(strength(b), b.tier, &b.id)));
        let snap = |c: &&BattleCardDto| crate::battles::to_snapshot_with(c, |w| by_slug.get(w).copied());
        let weakest: Vec<_> = fieldable.iter().take(LOAN_CARDS).map(snap).collect();
        let strong: Vec<_> = fieldable.iter().rev().take(SQUAD_MAX).map(snap).collect();
        let middle: Vec<_> = crate::runs::middle_squad(&fieldable, 4).iter().map(snap).collect();
        let tiers = vec![
            crate::runs_probe::ProbeTier { name: "weakest", squad: weakest },
            crate::runs_probe::ProbeTier { name: "middle", squad: middle },
            crate::runs_probe::ProbeTier { name: "strong", squad: strong },
        ];
        crate::runs_probe::start(probe_waves, tiers)
            .ok_or_else(|| AppError::BadRequest("A probe is already running".into()))
    }

    pub fn admin_read_route_probe(&self, id: Uuid) -> Result<crate::runs_probe::RouteProbeState> {
        crate::runs_probe::read(id).ok_or_else(|| AppError::NotFound("No such probe".into()))
    }
}

/// Волна без стороны игрока: враги, подкрепление, условие, поле и правила.
/// Одна сборка на партию забега и на пробу маршрута — две копии разошлись бы.
pub(super) struct WaveBase {
    pub setup: battle_core::Setup,
    pub rules: battle_core::Rules,
    pub ground: crate::battles::LineupGround,
    /// Глубина руки врага — как у этюда-основы.
    pub depth: u8,
    /// Строения игрока из этюда (стена, башня): стоят всегда и встают после
    /// отряда, чтобы номера карт отряда на поле не сдвинулись.
    pub structures: Vec<(battle_core::CardSnapshot, battle_core::Cell)>,
}

impl AppService {
    /// Карты из собрания человека, которые можно взять на замену: годятся для
    /// выхода в бой и не стоят в отряде.
    pub(super) async fn bench_of(&self, user_id: Uuid, squad: &[SquadCard]) -> Result<Vec<Uuid>> {
        let shelf = self.list_battle_cards_public().await?;
        let in_squad: std::collections::HashSet<Uuid> = squad.iter().map(|c| c.card_id).collect();
        let owned: std::collections::HashSet<Uuid> = self
            .repo
            .list_owned_battle_cards(user_id)
            .await?
            .into_iter()
            .map(|o| o.card_id)
            .collect();
        Ok(shelf
            .iter()
            .filter(|c| crate::battles::can_take_the_field(c))
            .filter_map(|c| c.id.parse().ok())
            .filter(|id| owned.contains(id) && !in_squad.contains(id))
            .collect())
    }

    /// Поле волны для экрана расстановки. Не собралось — `None`: подсказка не
    /// должна мешать начать волну, а расстановку сервер сделает сам.
    pub(super) async fn wave_field(&self, wave: &BattleRouteWave) -> Option<WaveFieldDto> {
        let etude_cards = self.cards_by_slug().await.ok()?;
        let base = self.wave_base(wave, &etude_cards).await.ok()?;
        let challenge = self.repo.get_battle_challenge(wave.challenge_id).await.ok()?;
        let arrangement: ChallengeSetup = serde_json::from_str(&challenge.setup).ok()?;

        let placed = |p: &crate::models::ChallengePlacement| PlacedDto { slug: p.card.clone(), x: p.x, y: p.y };
        let enemies = arrangement.keeper_board.iter().map(placed).collect();
        let structures = arrangement
            .player_board
            .iter()
            .filter(|p| {
                etude_cards
                    .get(&p.card)
                    .map(|c| crate::battles::to_snapshot_with(c, |want| etude_cards.get(want)).structure)
                    .unwrap_or(false)
            })
            .map(placed)
            .collect();
        let entries: Vec<WaveEntry> = serde_json::from_str(&wave.reinforcements).unwrap_or_default();
        let entrances = entries
            .into_iter()
            .map(|e| EntranceDto { round: e.round, x: e.x, y: e.y, slug: e.card })
            .collect();
        let open_cells = base
            .ground
            .open_cells()
            .into_iter()
            .filter_map(|(x, y)| battle_core::Cell::new(x, y))
            .collect();

        // Зоны ударов: враги на своих местах, у игрока только строения.
        let mut raised = base.setup.clone();
        raised.player_board = base.structures.clone();
        raised.player_health.clear();
        raised.player_hand.clear();
        let state = battle_core::MatchState::begin_with(raised, base.rules);
        let threats = state
            .strike_zones(battle_core::Side::Keeper)
            .into_iter()
            .map(|(cell, cells)| crate::models::StrikeZoneDto { cell, cells })
            .collect();

        Some(WaveFieldDto {
            width: base.ground.field.width,
            depth: base.ground.field.depth,
            terrain: base.ground.terrain.clone(),
            enemies,
            structures,
            entrances,
            open_cells,
            threats,
        })
    }

    pub(super) async fn wave_base(
        &self,
        wave: &BattleRouteWave,
        etude_cards: &std::collections::BTreeMap<String, BattleCardDto>,
    ) -> Result<WaveBase> {
        let challenge = self.repo.get_battle_challenge(wave.challenge_id).await?;
        let arrangement: ChallengeSetup = serde_json::from_str(&challenge.setup)
            .map_err(|_| AppError::Internal("Wave arrangement will not read".into()))?;
        let mut setup = Self::raise(&arrangement, etude_cards)?;
        // Сторона игрока приходит из отряда; из этюда остаются только строения.
        let structures: Vec<_> =
            setup.player_board.drain(..).filter(|(c, _)| c.structure).collect();
        setup.player_hand.clear();
        let snapshot_of = |slug: &str| {
            etude_cards
                .get(slug)
                .map(|c| crate::battles::to_snapshot_with(c, |want| etude_cards.get(want)))
        };
        setup.reinforcements = read_reinforcements(&wave.reinforcements, setup.field, snapshot_of)
            .map_err(AppError::BadRequest)?;
        setup.goal = read_goal(wave.goal.as_deref()).map_err(AppError::BadRequest)?;
        let mut ground = crate::battles::LineupGround::of(&arrangement);
        ground.occupied.extend(structures.iter().map(|(_, c)| (c.x, c.y)));
        Ok(WaveBase {
            ground,
            structures,
            rules: crate::battles::rules_of(&arrangement),
            depth: crate::battles::clamp_bot_depth(challenge.bot_depth) as u8,
            setup,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(dead: bool, health: Option<i32>) -> SquadCard {
        SquadCard { card_id: Uuid::new_v4(), health, dead, loaned: false, cell: None }
    }

    #[test]
    fn practice_starts_from_the_health_the_wave_started_with() {
        let (fell, wounded, spare, long_dead) =
            (card(true, Some(0)), card(false, Some(1)), card(false, Some(9)), card(true, Some(0)));
        let ids = (fell.card_id, wounded.card_id, spare.card_id, long_dead.card_id);
        let squad = vec![fell, wounded, spare, long_dead];
        // В волну вышли две карты: упавшая в ней и раненая; здоровье — с начала волны.
        let back = squad_at_wave_start(squad, &[ids.0, ids.1], &[7, 5]);
        let get = |id: Uuid| back.iter().find(|c| c.card_id == id).unwrap();
        assert!(!get(ids.0).dead);
        assert_eq!(get(ids.0).health, Some(7));
        assert_eq!(get(ids.1).health, Some(5));
        // Не выходившая карта не меняется, павшая раньше остаётся павшей.
        assert_eq!(get(ids.2).health, Some(9));
        assert!(get(ids.3).dead);
    }

    /// Заряд лечения тратится ровно один раз, и только на тот отряд, по которому посчитан.
    #[sqlx::test(migrations = "./migrations")]
    async fn a_heal_charge_is_spent_once(pool: sqlx::PgPool) {
        let repo = crate::db::Repository::new(pool.clone());
        let ctx = crate::models::ClientContext { ip: None, user_agent: None, country_code: None, city: None };
        let user = repo.create_guest_user(&ctx).await.unwrap();
        let route: Uuid = sqlx::query_scalar(
            "INSERT INTO battle_routes (slug, title_en, title_ru, status) VALUES ('h', 'H', 'Н', 'published') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let run = repo.insert_battle_run(user.id, route, 3, "[1]", 1, None).await.unwrap().unwrap();
        assert_eq!(repo.run_heal_charges(run.id).await.unwrap(), 0);
        // Зарядов нет: лечить нечем.
        assert!(repo.heal_run_card(run.id, user.id, "[1]", "[2]").await.unwrap().is_none());

        sqlx::query("UPDATE battle_runs SET heal_charges = 1 WHERE id = $1").bind(run.id).execute(&pool).await.unwrap();
        // Отряд уже другой (гонка с наградой или заменой): заряд не тратится.
        assert!(repo.heal_run_card(run.id, user.id, "[stale]", "[2]").await.unwrap().is_none());
        assert_eq!(repo.run_heal_charges(run.id).await.unwrap(), 1);
        // Нужный отряд: лечение записано, заряд потрачен, повтор ничего не делает.
        let healed = repo.heal_run_card(run.id, user.id, "[1]", "[2]").await.unwrap().unwrap();
        assert_eq!(healed.squad, "[2]");
        assert_eq!(repo.run_heal_charges(run.id).await.unwrap(), 0);
        assert!(repo.heal_run_card(run.id, user.id, "[2]", "[3]").await.unwrap().is_none());
    }

    /// Один забег дня на человека в сутки держит база, а не код: проверяем индекс.
    #[sqlx::test(migrations = "./migrations")]
    async fn the_database_allows_one_daily_run_a_day(pool: sqlx::PgPool) {
        let repo = crate::db::Repository::new(pool.clone());
        let ctx = crate::models::ClientContext { ip: None, user_agent: None, country_code: None, city: None };
        let user = repo.create_guest_user(&ctx).await.unwrap();
        let route: Uuid = sqlx::query_scalar(
            "INSERT INTO battle_routes (slug, title_en, title_ru, status) VALUES ('t', 'T', 'Т', 'published') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let day = chrono::NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        let next = day.succ_opt().unwrap();

        let first = repo.insert_battle_run(user.id, route, 3, "[]", 1, Some(day)).await.unwrap();
        assert!(first.is_some());
        // Закончили забег (иначе второй не вставится по другой причине — один незаконченный).
        repo.end_battle_run(first.unwrap().id, user.id, "lost").await.unwrap();

        // Тот же день — отказ базы; обычный забег и забег следующего дня проходят.
        assert!(repo.insert_battle_run(user.id, route, 3, "[]", 2, Some(day)).await.unwrap().is_none());
        let plain = repo.insert_battle_run(user.id, route, 3, "[]", 3, None).await.unwrap().unwrap();
        repo.end_battle_run(plain.id, user.id, "lost").await.unwrap();
        assert!(repo.insert_battle_run(user.id, route, 3, "[]", 4, Some(next)).await.unwrap().is_some());

        assert!(repo.find_daily_battle_run(user.id, day).await.unwrap().is_some());
        assert_eq!(repo.daily_battle_run_counts(day).await.unwrap(), (1, 0));
    }

    fn route() -> AdminRoute {
        AdminRoute {
            id: None,
            slug: "first-route".into(),
            title_en: "First".into(),
            title_ru: "Первый".into(),
            title_uk: String::new(),
            title_es: String::new(),
            title_de: String::new(),
            title_it: String::new(),
            guest_waves: Some(2),
            status: "draft".into(),
            sort_order: None,
            waves: (1..=3)
                .map(|n| AdminWave {
                    wave_no: n,
                    challenge_id: Uuid::nil(),
                    reinforcements: vec![],
                    goal: None,
                    min_cards: 1,
                    max_cards: 3,
                    reward_dust: 10,
                    race_hint: None,
                })
                .collect(),
            warnings: vec![],
        }
    }

    #[test]
    fn a_whole_route_passes() {
        assert!(check_route(&route()).is_ok());
    }

    #[test]
    fn each_breakage_is_refused_in_words() {
        let mut r = route();
        r.slug = "Bad Slug".into();
        assert!(check_route(&r).is_err());

        let mut r = route();
        r.waves[1].wave_no = 3;
        assert!(check_route(&r).is_err(), "дыра в номерах");

        let mut r = route();
        r.waves[0].min_cards = 4;
        assert!(check_route(&r).is_err(), "минимум выше максимума");

        let mut r = route();
        r.guest_waves = Some(9);
        assert!(check_route(&r).is_err(), "гостю больше волн, чем есть");

        let mut r = route();
        r.waves.clear();
        assert!(check_route(&r).is_err());

        let mut r = route();
        r.waves[0].goal = Some(battle_core::WaveGoal::HoldRounds { rounds: 0 });
        assert!(check_route(&r).is_err());
    }
}
