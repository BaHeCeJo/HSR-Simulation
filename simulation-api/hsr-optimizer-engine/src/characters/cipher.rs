use crate::damage;
use crate::effects;
use crate::ids;
use crate::models::{ActionParams, ActionType, SimState, StatusEffect};

// ── SPD helper ────────────────────────────────────────────────────────────────
fn effective_spd(member: &crate::models::TeamMember) -> f64 {
    member.base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0)
        * (1.0 + member.buffs.speed_percent / 100.0)
}

// ── Tally rates ───────────────────────────────────────────────────────────────
fn tally_rate(state: &SimState, c_idx: usize) -> f64 {
    // Base 12%; E1 ×1.5; A2 SPD≥140 ×1.5, SPD≥170 ×2.0
    let e1_mult = if state.team[c_idx].eidolon >= 1 { 1.5 } else { 1.0 };
    let spd     = effective_spd(&state.team[c_idx]);
    let a2_mult = if spd >= 170.0 { 2.0 } else if spd >= 140.0 { 1.5 } else { 1.0 };
    12.0 * e1_mult * a2_mult
}

fn a4_tally_rate(state: &SimState, c_idx: usize) -> f64 {
    // Base 8%; same E1/A2 scaling
    let e1_mult = if state.team[c_idx].eidolon >= 1 { 1.5 } else { 1.0 };
    let spd     = effective_spd(&state.team[c_idx]);
    let a2_mult = if spd >= 170.0 { 2.0 } else if spd >= 140.0 { 1.5 } else { 1.0 };
    8.0 * e1_mult * a2_mult
}

// ── Patron helpers ────────────────────────────────────────────────────────────
fn get_patron(state: &SimState) -> Option<usize> {
    let v = state.stacks.get("cipher_patron_slot").copied().unwrap_or(-1.0);
    if v < 0.0 { None } else { Some(v as usize) }
}

fn find_highest_hp_enemy(state: &SimState) -> Option<usize> {
    state.enemies.iter().enumerate()
        .filter_map(|(i, s)| s.as_ref().filter(|e| e.hp > 0.0).map(|e| (i, e.max_hp)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
}

fn ensure_patron(state: &mut SimState, c_idx: usize) {
    let alive = get_patron(state)
        .and_then(|s| state.enemies.get(s))
        .and_then(|s| s.as_ref())
        .map_or(false, |e| e.hp > 0.0);
    if !alive {
        let new_p = find_highest_hp_enemy(state);
        state.stacks.insert("cipher_patron_slot".to_string(), new_p.map(|i| i as f64).unwrap_or(-1.0));
        if let Some(s) = new_p {
            let cname = state.team[c_idx].name.clone();
            let ename = state.enemies[s].as_ref().map(|e| e.name.clone()).unwrap_or_default();
            state.add_log(&cname, format!("Patron auto-assigned: {}", ename));
        }
    }
}

fn set_patron(state: &mut SimState, c_idx: usize, slot: usize) {
    state.stacks.insert("cipher_patron_slot".to_string(), slot as f64);
    let cname = state.team[c_idx].name.clone();
    let ename = state.enemies.get(slot).and_then(|s| s.as_ref()).map(|e| e.name.clone()).unwrap_or_default();
    state.add_log(&cname, format!("Patron: {}", ename));
}

// ── Adjacent slots ────────────────────────────────────────────────────────────
fn adj_slots(state: &SimState, t: usize) -> Vec<usize> {
    let len = state.enemies.len();
    let mut v = Vec::new();
    if t > 0 { v.push(t - 1); }
    if t + 1 < len { v.push(t + 1); }
    v
}

// ── True DMG (bypasses CRIT/DMG%/DEF/RES; applies vuln, mitig, broken) ───────
fn apply_true_dmg(state: &mut SimState, slot: usize, amount: f64) -> f64 {
    if amount <= 0.0 { return 0.0; }
    let (vuln_m, mitig_m, broken_m) = match state.enemies.get(slot).and_then(|s| s.as_ref()) {
        Some(e) if e.hp > 0.0 => (
            1.0 + (e.vulnerability + e.cached_vuln_bonus) / 100.0,
            1.0 - e.dmg_reduction / 100.0,
            if e.is_broken { 1.0 } else { 0.9 },
        ),
        _ => return 0.0,
    };
    let dmg = (amount * vuln_m * mitig_m * broken_m).floor();
    if dmg > 0.0 {
        if let Some(e) = state.enemies[slot].as_mut() { e.hp -= dmg; }
        state.total_damage += dmg;
        if state.enemies[slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[slot] = None;
        }
    }
    dmg
}

// ── Tally accumulation ────────────────────────────────────────────────────────
fn add_tally(state: &mut SimState, amount: f64) {
    if amount <= 0.0 { return; }
    let cur = state.stacks.get("cipher_tally").copied().unwrap_or(0.0);
    state.stacks.insert("cipher_tally".to_string(), cur + amount);
}

// ── E2 vulnerability debuff (direct field + stacks duration tracking) ─────────
fn apply_e2_vuln(state: &mut SimState, c_idx: usize, slot: usize) {
    let ehr = state.team[c_idx].buffs.effect_hit_rate;
    let effect_res = state.enemies.get(slot).and_then(|s| s.as_ref()).map(|e| e.effect_res).unwrap_or(0.0);
    if !effects::debuff_lands(ehr, effect_res, 1.2) { return; }

    let key  = format!("cipher_e2_{slot}");
    let prev = state.stacks.get(&key).copied().unwrap_or(0.0);
    if prev <= 0.0 {
        // Fresh application: add 30% vulnerability
        if let Some(enemy) = state.enemies[slot].as_mut() {
            enemy.vulnerability += 30.0;
        }
    }
    // Refresh duration to 2 enemy turns
    state.stacks.insert(key, 2.0);
}

// ── FUA (Talent Follow-up ATK) ────────────────────────────────────────────────
fn fire_fua(state: &mut SimState, c_idx: usize, target_slot: usize) -> f64 {
    if state.enemies.get(target_slot).and_then(|s| s.as_ref()).map_or(true, |e| e.hp <= 0.0) {
        return 0.0;
    }
    let e6 = state.team[c_idx].eidolon >= 6;
    // Base 150% + E6 +350% = 500% total at E6
    let fua_mult = if e6 { 5.00 } else { 1.50 };

    let mut member = state.team[c_idx].clone();
    // A6: FUA CRIT DMG +100%
    member.buffs.crit_dmg += 100.0;

    let fua_action = ActionParams {
        action_type:      ActionType::FollowUp,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       fua_mult,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 20.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };

    let fua_dmg = state.enemies[target_slot].as_ref()
        .map(|e| damage::calculate_damage(&member, e, &fua_action))
        .unwrap_or(0.0);
    if fua_dmg > 0.0 {
        if let Some(e) = state.enemies[target_slot].as_mut() { e.hp -= fua_dmg; }
        state.total_damage += fua_dmg;
        if state.enemies[target_slot].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[target_slot] = None;
        }
    }

    // Tally FUA damage (Patron hit) + E6 extra 16%
    let rate     = tally_rate(state, c_idx);
    let e6_extra = if e6 { 16.0 } else { 0.0 };
    add_tally(state, fua_dmg * (rate + e6_extra) / 100.0);

    // E1: Cipher ATK +80% for 2 turns on FUA (counter for on_before_action)
    if state.team[c_idx].eidolon >= 1 {
        let old = state.stacks.get("cipher_atk_e1_rem").copied().unwrap_or(0.0);
        state.stacks.insert("cipher_atk_e1_rem".to_string(), old.max(2.0));
    }

    let tally = state.stacks.get("cipher_tally").copied().unwrap_or(0.0);
    let cname = state.team[c_idx].name.clone();
    state.add_log(&cname, format!(
        "Talent FUA: {:.0}% ATK Quantum, {:.0} DMG (Tally → {:.1})",
        fua_mult * 100.0, fua_dmg, tally
    ));
    fua_dmg
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

pub fn on_battle_start(state: &mut SimState, idx: usize) {
    state.team[idx].max_energy = 130.0;
    state.team[idx].is_fua     = true;

    // Minor traces
    state.team[idx].buffs.dmg_boost       += 14.4; // Quantum DMG +14.4%
    state.team[idx].buffs.effect_hit_rate += 10.0;  // EHR +10%
    // SPD +14 (flat, added to base_stats so it survives snapshot)
    let cur_spd = state.team[idx].base_stats.get(ids::CHAR_SPD_ID).copied().unwrap_or(100.0);
    state.team[idx].base_stats.insert(ids::CHAR_SPD_ID.to_string(), cur_spd + 14.0);

    // A2: CRIT Rate +25/50% based on SPD threshold
    let spd = effective_spd(&state.team[idx]);
    let a2cr = if spd >= 170.0 { 50.0 } else if spd >= 140.0 { 25.0 } else { 0.0 };
    state.team[idx].buffs.crit_rate += a2cr;

    // Init cross-character state
    state.stacks.insert("cipher_patron_slot".to_string(), -1.0);
    state.stacks.insert("cipher_tally".to_string(), 0.0);
    state.stacks.insert("cipher_fua_used".to_string(), 0.0);

    // A6: all enemies take +40% DMG while Cipher is on the battlefield
    for slot in state.enemies.iter_mut().flatten() {
        slot.vulnerability += 40.0;
    }

    // Assign initial Patron (highest max HP enemy)
    ensure_patron(state, idx);

    // Technique: AoE 100% ATK Quantum to all enemies with 200% tally boost
    let member = state.team[idx].clone();
    let tech_action = ActionParams {
        action_type:      ActionType::TalentProc,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       1.0,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 10.0,
        inflicts_debuff:  false,
        is_ult_dmg:       false,
    };
    let alive: Vec<usize> = state.enemies.iter().enumerate()
        .filter_map(|(i, s)| if s.as_ref().map_or(false, |e| e.hp > 0.0) { Some(i) } else { None })
        .collect();
    let patron_now = get_patron(state);
    let mut tech_dmg = 0.0f64;
    for &i in &alive {
        let dmg = state.enemies[i].as_ref()
            .map(|e| damage::calculate_damage(&member, e, &tech_action))
            .unwrap_or(0.0);
        if dmg > 0.0 {
            if let Some(e) = state.enemies[i].as_mut() { e.hp -= dmg; }
            tech_dmg += dmg;
        }
        // Tally at 3× normal rate (base + 200% boost from Technique)
        let rate = if Some(i) == patron_now { tally_rate(state, idx) } else { a4_tally_rate(state, idx) };
        add_tally(state, dmg * rate * 3.0 / 100.0);
        if state.enemies[i].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[i] = None;
        }
    }
    state.total_damage += tech_dmg;

    let tally = state.stacks.get("cipher_tally").copied().unwrap_or(0.0);
    let name  = state.team[idx].name.clone();
    state.add_log(&name, format!("Technique AoE: {:.0} DMG, Tally → {:.1}", tech_dmg, tally));
    ensure_patron(state, idx);
}

pub fn on_turn_start(state: &mut SimState, idx: usize) {
    // Reset per-Cipher-turn FUA
    state.stacks.insert("cipher_fua_used".to_string(), 0.0);
    // Re-check Patron validity
    ensure_patron(state, idx);
}

pub fn on_before_action(
    state: &mut SimState,
    idx: usize,
    action: &mut ActionParams,
    _target_idx: Option<usize>,
) {
    match action.action_type {
        ActionType::Skill => {
            // Apply +30% ATK temporarily for Skill's own damage
            state.team[idx].buffs.atk_percent += 30.0;
            // Handle Skill damage manually (blast), signal debuff for Acheron etc.
            action.multiplier       = 0.0;
            action.toughness_damage = 0.0;
            action.inflicts_debuff  = true; // Weaken applied → trigger on_global_debuff
        }
        ActionType::Ultimate => {
            // AoE + True DMG handled in on_ult; suppress default single-target path
            action.inflicts_debuff = false;
        }
        _ => {
            // Apply persistent Skill ATK buff for non-Skill actions
            let sk_rem = state.stacks.get("cipher_atk_skill_rem").copied().unwrap_or(0.0);
            if sk_rem > 0.0 {
                state.team[idx].buffs.atk_percent += 30.0;
                if sk_rem <= 1.0 { state.stacks.remove("cipher_atk_skill_rem"); }
                else { state.stacks.insert("cipher_atk_skill_rem".to_string(), sk_rem - 1.0); }
            }
        }
    }

    // E1 ATK +80% (applies to all action types, decrements each time Cipher acts)
    let e1_rem = state.stacks.get("cipher_atk_e1_rem").copied().unwrap_or(0.0);
    if e1_rem > 0.0 {
        state.team[idx].buffs.atk_percent += 80.0;
        if e1_rem <= 1.0 { state.stacks.remove("cipher_atk_e1_rem"); }
        else { state.stacks.insert("cipher_atk_e1_rem".to_string(), e1_rem - 1.0); }
    }
}

pub fn on_after_action(
    state: &mut SimState,
    idx: usize,
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    ensure_patron(state, idx);

    match action.action_type {
        ActionType::Basic => {
            if let Some(t) = target_idx {
                // E2: vulnerability debuff on Basic hit (120% base chance)
                if state.team[idx].eidolon >= 2 { apply_e2_vuln(state, idx, t); }

                // Tally Cipher's own Basic damage to Patron
                if get_patron(state) == Some(t) {
                    let member = state.team[idx].clone();
                    let dmg    = state.enemies[t].as_ref()
                        .map(|e| damage::calculate_damage(&member, e, action))
                        .unwrap_or(0.0);
                    let rate = tally_rate(state, idx);
                    add_tally(state, dmg * rate / 100.0);
                }
            }
        }

        ActionType::Skill => {
            // Persist ATK +30% for next 2 of Cipher's own action windows
            state.stacks.insert("cipher_atk_skill_rem".to_string(), 2.0);

            if let Some(t) = target_idx {
                // Primary target becomes Patron
                set_patron(state, idx, t);

                let member = state.team[idx].clone();
                let ehr    = member.buffs.effect_hit_rate;
                let e2     = state.team[idx].eidolon >= 2;

                // Weaken primary + adjacent (120% base chance; duration 2 enemy turns)
                let weaken = StatusEffect { duration: 2, value: 10.0, stat: Some("Weaken".to_string()), effects: vec![] };
                if let Some(enemy) = state.enemies[t].as_mut() {
                    effects::try_apply_enemy_debuff(ehr, enemy, "cipher_weaken", weaken.clone(), 1.2);
                }
                let adjs = adj_slots(state, t);
                for &adj in &adjs {
                    if let Some(enemy) = state.enemies[adj].as_mut() {
                        effects::try_apply_enemy_debuff(ehr, enemy, "cipher_weaken", weaken.clone(), 1.2);
                    }
                }

                // E2: vulnerability on all hit targets
                if e2 {
                    apply_e2_vuln(state, idx, t);
                    for &adj in &adjs { apply_e2_vuln(state, idx, adj); }
                }

                // Blast damage: main 200% ATK + adjacent 100% ATK
                let main_action = ActionParams {
                    action_type:      ActionType::Skill,
                    scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
                    multiplier:       2.0,
                    extra_multiplier: 0.0,
                    extra_dmg:        0.0,
                    toughness_damage: 20.0,
                    inflicts_debuff:  false,
                    is_ult_dmg:       false,
                };
                let adj_action = ActionParams { multiplier: 1.0, toughness_damage: 10.0, ..main_action.clone() };

                let main_dmg = state.enemies[t].as_ref()
                    .filter(|e| e.hp > 0.0)
                    .map(|e| damage::calculate_damage(&member, e, &main_action))
                    .unwrap_or(0.0);
                if main_dmg > 0.0 {
                    if let Some(e) = state.enemies[t].as_mut() { e.hp -= main_dmg; }
                    state.total_damage += main_dmg;
                    if state.enemies[t].as_ref().map_or(false, |e| e.hp <= 0.0) {
                        state.enemies[t] = None;
                    }
                }
                // Tally main (Patron)
                add_tally(state, main_dmg * tally_rate(state, idx) / 100.0);

                let mut adj_total = 0.0f64;
                for &adj in &adjs {
                    let is_patron = get_patron(state) == Some(adj);
                    let adj_dmg   = state.enemies[adj].as_ref()
                        .filter(|e| e.hp > 0.0)
                        .map(|e| damage::calculate_damage(&member, e, &adj_action))
                        .unwrap_or(0.0);
                    if adj_dmg > 0.0 {
                        if let Some(e) = state.enemies[adj].as_mut() { e.hp -= adj_dmg; }
                        adj_total += adj_dmg;
                    }
                    if state.enemies[adj].as_ref().map_or(false, |e| e.hp <= 0.0) {
                        state.enemies[adj] = None;
                    }
                    let adj_rate = if is_patron { tally_rate(state, idx) } else { a4_tally_rate(state, idx) };
                    add_tally(state, adj_dmg * adj_rate / 100.0);
                }
                state.total_damage += adj_total;

                let tally = state.stacks.get("cipher_tally").copied().unwrap_or(0.0);
                let cname = state.team[idx].name.clone();
                state.add_log(&cname, format!(
                    "Skill: main {:.0}, adj {:.0}, Weaken applied, Tally → {:.1}",
                    main_dmg, adj_total, tally
                ));
            }
        }
        _ => {}
    }
}

pub fn on_ult(state: &mut SimState, idx: usize) {
    state.team[idx].stacks.insert("_ult_handled", 1.0);
    state.team[idx].energy = 5.0;

    ensure_patron(state, idx);

    // Ult primary target = Patron, else highest HP enemy
    let t = match get_patron(state).or_else(|| find_highest_hp_enemy(state)) {
        Some(s) if state.enemies[s].as_ref().map_or(false, |e| e.hp > 0.0) => s,
        _ => return,
    };
    set_patron(state, idx, t);

    let e4 = state.team[idx].eidolon >= 4;
    let e6 = state.team[idx].eidolon >= 6;
    let member = state.team[idx].clone();

    // E2: vulnerability on all hit targets
    if state.team[idx].eidolon >= 2 {
        apply_e2_vuln(state, idx, t);
        for &adj in &adj_slots(state, t) { apply_e2_vuln(state, idx, adj); }
    }

    // ── Quantum DMG: 120% ATK to main ────────────────────────────────────────
    let q_action = ActionParams {
        action_type:      ActionType::Ultimate,
        scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
        multiplier:       1.2,
        extra_multiplier: 0.0,
        extra_dmg:        0.0,
        toughness_damage: 30.0,
        inflicts_debuff:  false,
        is_ult_dmg:       true,
    };
    let main_q = state.enemies[t].as_ref().filter(|e| e.hp > 0.0)
        .map(|e| damage::calculate_damage(&member, e, &q_action)).unwrap_or(0.0);
    if main_q > 0.0 {
        if let Some(e) = state.enemies[t].as_mut() { e.hp -= main_q; }
        state.total_damage += main_q;
    }

    // ── Read + clear tally ────────────────────────────────────────────────────
    let tally = state.stacks.get("cipher_tally").copied().unwrap_or(0.0);
    let e6_return = if e6 { tally * 0.20 } else { 0.0 };
    state.stacks.insert("cipher_tally".to_string(), e6_return);

    // ── True DMG: 25% of tally to main target ────────────────────────────────
    let true_main = apply_true_dmg(state, t, tally * 0.25);

    // ── Blast phase: 40% ATK Quantum + 75% tally True DMG to main + adjacent ─
    let q_blast_action = ActionParams { multiplier: 0.4, toughness_damage: 20.0, ..q_action.clone() };

    let all_targets: Vec<usize> = {
        let adjs = adj_slots(state, t);
        std::iter::once(t)
            .chain(adjs)
            .filter(|&i| state.enemies.get(i).and_then(|s| s.as_ref()).map_or(false, |e| e.hp > 0.0))
            .collect()
    };
    let n = all_targets.len().max(1) as f64;
    let true_each = tally * 0.75 / n;

    let mut blast_q = 0.0f64;
    let mut blast_t = 0.0f64;
    for &i in &all_targets {
        let qd = state.enemies[i].as_ref().filter(|e| e.hp > 0.0)
            .map(|e| damage::calculate_damage(&member, e, &q_blast_action)).unwrap_or(0.0);
        if qd > 0.0 {
            if let Some(e) = state.enemies[i].as_mut() { e.hp -= qd; }
            blast_q += qd;
        }
        let td = apply_true_dmg(state, i, true_each);
        blast_t += td;

        if state.enemies[i].as_ref().map_or(false, |e| e.hp <= 0.0) {
            state.enemies[i] = None;
        }
    }
    state.total_damage += blast_q;

    // Tally Cipher's own Quantum hits from the Ult (NOT the True DMG, NOT added back to cleared tally)
    // These go into the NEW tally cycle (post-clear)
    let patron_now = get_patron(state);
    add_tally(state, main_q * tally_rate(state, idx) / 100.0);
    for &i in &all_targets {
        let q_here = if i == t { 0.0 } else { // main already tallied above
            state.enemies.get(i).and_then(|s| s.as_ref())
                .filter(|e| e.hp >= 0.0) // might be dead but we tallied during deal
                .map(|e| damage::calculate_damage(&member, e, &q_blast_action)).unwrap_or(0.0)
        };
        let is_patron = patron_now == Some(i);
        let rate = if is_patron { tally_rate(state, idx) } else { a4_tally_rate(state, idx) };
        add_tally(state, q_here * rate / 100.0);
    }
    // Also tally the 40% blast hit on main
    add_tally(state, {
        let qd = all_targets.first().copied().filter(|&i| i == t)
            .and_then(|_| state.enemies.get(t))
            .and_then(|s| s.as_ref())
            .map(|e| damage::calculate_damage(&member, e, &q_blast_action)).unwrap_or(0.0);
        qd * tally_rate(state, idx) / 100.0
    });

    // E4: additional 50% ATK Quantum on each target hit
    if e4 {
        let e4_action = ActionParams {
            action_type:      ActionType::FollowUp,
            scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
            multiplier:       0.5,
            extra_multiplier: 0.0,
            extra_dmg:        0.0,
            toughness_damage: 0.0,
            inflicts_debuff:  false,
            is_ult_dmg:       false,
        };
        let mut e4_total = 0.0f64;
        for &i in &all_targets {
            let e4d = state.enemies.get(i).and_then(|s| s.as_ref())
                .filter(|e| e.hp > 0.0)
                .map(|e| damage::calculate_damage(&member, e, &e4_action)).unwrap_or(0.0);
            if e4d > 0.0 {
                if let Some(e) = state.enemies[i].as_mut() { e.hp -= e4d; }
                e4_total += e4d;
            }
            if state.enemies.get(i).and_then(|s| s.as_ref()).map_or(false, |e| e.hp <= 0.0) {
                if i < state.enemies.len() { state.enemies[i] = None; }
            }
        }
        state.total_damage += e4_total;
    }

    ensure_patron(state, idx);
    let new_tally = state.stacks.get("cipher_tally").copied().unwrap_or(0.0);
    let name = state.team[idx].name.clone();
    state.add_log(&name, format!(
        "Yours Truly: Q={:.0}, TrueDMG={:.0}+{:.0}, BlastQ={:.0}, Tally: {:.1}→{:.1}{}",
        main_q, true_main, blast_t, blast_q, tally, new_tally,
        if e6 { format!(" [E6 ret {:.0}]", e6_return) } else { String::new() }
    ));
}

#[allow(dead_code)]
pub fn on_break(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_global_debuff(
    _state: &mut SimState,
    _idx: usize,
    _source_idx: usize,
    _enemy_idx: usize,
) {}

pub fn on_enemy_turn_start(state: &mut SimState, _idx: usize, enemy_idx: usize) {
    // Tick E2 vulnerability duration; remove on expiry
    let key = format!("cipher_e2_{enemy_idx}");
    let rem = state.stacks.get(&key).copied().unwrap_or(0.0);
    if rem > 0.0 {
        if rem <= 1.0 {
            state.stacks.remove(&key);
            if let Some(enemy) = state.enemies[enemy_idx].as_mut() {
                enemy.vulnerability = (enemy.vulnerability - 30.0).max(0.0);
            }
        } else {
            state.stacks.insert(key, rem - 1.0);
        }
    }
}

pub fn on_enemy_action(_state: &mut SimState, _idx: usize, _enemy_idx: usize) {}

pub fn on_ally_action(
    state: &mut SimState,
    idx: usize,        // Cipher's team index
    source_idx: usize, // ally who just acted
    action: &ActionParams,
    target_idx: Option<usize>,
) {
    ensure_patron(state, idx);

    // Only react to damage-dealing actions with a target
    let is_attack = matches!(action.action_type,
        ActionType::Basic | ActionType::Skill | ActionType::Ultimate
        | ActionType::FollowUp | ActionType::TalentProc);
    let t = match (is_attack, target_idx) {
        (true, Some(t)) => t,
        _ => return,
    };

    // Enemy must be alive for any tally/FUA to make sense
    if state.enemies.get(t).and_then(|s| s.as_ref()).map_or(true, |e| e.hp <= 0.0) {
        return;
    }

    let patron = get_patron(state);

    // Tally damage to non-Patron enemies (A4)
    if patron != Some(t) {
        let member = state.team[source_idx].clone();
        let dmg    = state.enemies[t].as_ref()
            .map(|e| damage::calculate_damage(&member, e, action)).unwrap_or(0.0);
        add_tally(state, dmg * a4_tally_rate(state, idx) / 100.0);
        return;
    }

    // Patron was hit by an ally ─────────────────────────────────────────────
    // Tally damage
    let member = state.team[source_idx].clone();
    let dmg    = state.enemies[t].as_ref()
        .map(|e| damage::calculate_damage(&member, e, action)).unwrap_or(0.0);
    add_tally(state, dmg * tally_rate(state, idx) / 100.0);

    // FUA (1× per Cipher turn)
    let fua_used = state.stacks.get("cipher_fua_used").copied().unwrap_or(0.0);
    if fua_used < 1.0 {
        state.stacks.insert("cipher_fua_used".to_string(), 1.0);
        fire_fua(state, idx, t);
        ensure_patron(state, idx);
    }

    // E4: additional 50% ATK Quantum to Patron after any ally hit (no turn limit)
    if state.team[idx].eidolon >= 4 {
        let patron_now = get_patron(state).unwrap_or(t);
        if state.enemies.get(patron_now).and_then(|s| s.as_ref()).map_or(false, |e| e.hp > 0.0) {
            let e4_member = state.team[idx].clone();
            let e4_action = ActionParams {
                action_type:      ActionType::FollowUp,
                scaling_stat_id:  ids::CHAR_ATK_ID.to_string(),
                multiplier:       0.5,
                extra_multiplier: 0.0,
                extra_dmg:        0.0,
                toughness_damage: 0.0,
                inflicts_debuff:  false,
                is_ult_dmg:       false,
            };
            let e4_dmg = state.enemies[patron_now].as_ref()
                .map(|e| damage::calculate_damage(&e4_member, e, &e4_action)).unwrap_or(0.0);
            if e4_dmg > 0.0 {
                if let Some(e) = state.enemies[patron_now].as_mut() { e.hp -= e4_dmg; }
                state.total_damage += e4_dmg;
                if state.enemies[patron_now].as_ref().map_or(false, |e| e.hp <= 0.0) {
                    state.enemies[patron_now] = None;
                }
            }
            // Tally E4 hit
            add_tally(state, e4_dmg * tally_rate(state, idx) / 100.0);
            let cname = state.team[idx].name.clone();
            state.add_log(&cname, format!("E4 Quantum Additional: {:.0} DMG", e4_dmg));
            ensure_patron(state, idx);
        }
    }
}
