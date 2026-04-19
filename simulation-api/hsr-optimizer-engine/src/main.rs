mod characters;
mod lightcones;
mod damage;
mod effects;
mod enemies;
mod ids;
mod models;
mod planars;
mod relics;
mod simulator;

use axum::{routing::post, Json, Router};
use itertools::Itertools;
use models::{CharRelicConfig, IncomingCharacter, IncomingLightcone, IncomingWave, OptimizeRequest, OptimizeResult, SimReport};
use rand::prelude::*;
use rayon::prelude::*;
use simulator::run_simulation;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

// If C(N,4) is at or below this, run exhaustively (full rayon parallel).
// C(16,4) = 1820, C(20,4) = 4845, C(24,4) = 10626, C(26,4) = 14950
const EXHAUSTIVE_THRESHOLD: usize = 15_000;

// Joint SA (team + relics co-optimised).
// Each iteration tests one team+relic combo, so total sims ≈ RESTARTS × ITERS.
// 30% of moves swap a char slot; 35% change set combo; 35% change main stats.
const SA_JOINT_RESTARTS:   usize = 10;
const SA_JOINT_ITERATIONS: usize = 4_000; // 40 000 sims total

// Temperature calibration:
//   defeat-space score differences ~500–5 000, winning floor at 1e12.
const SA_T_START: f64 = 1_000.0;
const SA_T_END:   f64 = 0.5;

// ─── Server ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let app = Router::new()
        .route("/optimize", post(optimize_handler))
        .layer(CorsLayer::permissive());
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("HSR Optimizer API listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ─── Log formatting ──────────────────────────────────────────────────────────

fn format_logs(report: &SimReport) -> Vec<String> {
    report.logs.iter().flat_map(|l| {
        let base = format!("[AV {:.1}] {} {}", l.av, l.actor, l.message);
        let mut lines = vec![base];
        for sub in &l.sub_entries {
            lines.push(format!("  - {}", sub));
        }
        lines
    }).collect()
}

// ─── Comparison & scoring ────────────────────────────────────────────────────

/// Lexicographic comparison: not-defeated > fewer-cycles > higher-damage.
fn is_better(incumbent: &SimReport, challenger: &SimReport) -> bool {
    if incumbent.is_defeated != challenger.is_defeated {
        return !challenger.is_defeated;
    }
    if challenger.cycles_taken != incumbent.cycles_taken {
        return challenger.cycles_taken < incumbent.cycles_taken;
    }
    challenger.total_damage > incumbent.total_damage
}

/// Scalar score for SA acceptance.
fn score(report: &SimReport, max_cycles: i32) -> f64 {
    if report.is_defeated {
        report.total_damage
    } else {
        1e12 + (max_cycles - report.cycles_taken) as f64 * 2e9 + report.total_damage
    }
}

// ─── Element-aware default relic config ─────────────────────────────────────

/// A sensible starting relic configuration for any character:
///   Pioneer 4p + Broken Keel 2p, crit_dmg body, speed feet,
///   element-matching sphere, atk% rope.
fn default_relic_config(element: &str) -> relics::RelicConfig {
    let sphere = match element {
        "Physical"  => "physical_dmg",
        "Fire"      => "fire_dmg",
        "Ice"       => "ice_dmg",
        "Lightning" => "lightning_dmg",
        "Wind"      => "wind_dmg",
        "Quantum"   => "quantum_dmg",
        "Imaginary" => "imaginary_dmg",
        _           => "atk_percent",
    };
    relics::RelicConfig {
        relic_set:    "Pioneer 4p".to_string(),    // must match RELIC_SETS display label
        ornament_set: "broken_keel_2p".to_string(), // short code used by config_to_relics
        body_main:    "crit_dmg".to_string(),
        feet_main:    "speed".to_string(),
        sphere_main:  sphere.to_string(),
        rope_main:    "atk_percent".to_string(),
    }
}

// ─── Joint genome (team + relics) ───────────────────────────────────────────

#[derive(Clone)]
struct JointGenome {
    char_idxs:     Vec<usize>,
    relic_configs: Vec<relics::RelicConfig>,
}

fn run_genome(
    genome:     &JointGenome,
    char_pool:  &[IncomingCharacter],
    lc_pool:    &[IncomingLightcone],
    waves_data: &[IncomingWave],
    max_cycles: i32,
    with_logs:  bool,
) -> SimReport {
    run_team_with_relics(
        &genome.char_idxs,
        char_pool,
        lc_pool,
        &genome.relic_configs,
        waves_data,
        max_cycles,
        with_logs,
    )
}

// ─── LC greedy assignment ────────────────────────────────────────────────────

/// For each character in `chars`, find the best path-compatible LC from
/// `lc_pool` that hasn't been assigned yet. "Best" = highest (level * 10 +
/// superimposition). Characters that don't match any LC get `None`.
fn assign_lcs_greedily<'p>(
    chars: &[&IncomingCharacter],
    lc_pool: &'p [IncomingLightcone],
) -> Vec<Option<&'p IncomingLightcone>> {
    let mut used = vec![false; lc_pool.len()];
    chars.iter().map(|ch| {
        let char_path = ch.path.as_deref().unwrap_or("");
        let best = lc_pool.iter().enumerate()
            .filter(|(li, lc)| {
                !used[*li] && lc.path.as_deref().unwrap_or("") == char_path
            })
            .max_by_key(|(_, lc)| {
                lc.level.unwrap_or(0) * 10 + lc.superimposition.unwrap_or(0)
            });
        if let Some((li, lc)) = best {
            used[li] = true;
            Some(lc)
        } else {
            None
        }
    }).collect()
}

// ─── Shared simulation runners ───────────────────────────────────────────────

fn run_team(
    char_idxs: &[usize],
    char_pool: &[IncomingCharacter],
    lc_pool: &[IncomingLightcone],
    waves_data: &[IncomingWave],
    max_cycles: i32,
    with_logs: bool,
) -> SimReport {
    let char_refs: Vec<&IncomingCharacter> = char_idxs.iter().map(|&i| &char_pool[i]).collect();
    let lcs = assign_lcs_greedily(&char_refs, lc_pool);
    let mut chars_cloned: Vec<IncomingCharacter> = char_refs.iter().map(|&c| c.clone()).collect();
    for ch in chars_cloned.iter_mut() {
        if ch.relics.is_none() {
            let element = ch.attribute.as_deref().unwrap_or("Physical");
            ch.relics = Some(relics::config_to_relics(&default_relic_config(element)));
        }
    }
    run_simulation(&chars_cloned, &lcs, waves_data, max_cycles, with_logs)
}

fn run_team_with_relics(
    char_idxs: &[usize],
    char_pool: &[IncomingCharacter],
    lc_pool: &[IncomingLightcone],
    relic_configs: &[relics::RelicConfig],
    waves_data: &[IncomingWave],
    max_cycles: i32,
    with_logs: bool,
) -> SimReport {
    let char_refs: Vec<&IncomingCharacter> = char_idxs.iter().map(|&i| &char_pool[i]).collect();
    let lcs = assign_lcs_greedily(&char_refs, lc_pool);
    let mut chars_cloned: Vec<IncomingCharacter> = char_refs.iter().map(|&c| c.clone()).collect();
    for (ch, cfg) in chars_cloned.iter_mut().zip(relic_configs.iter()) {
        ch.relics = Some(relics::config_to_relics(cfg));
    }
    run_simulation(&chars_cloned, &lcs, waves_data, max_cycles, with_logs)
}

// ─── Exhaustive search ───────────────────────────────────────────────────────

fn exhaustive_search(
    char_pool: &[IncomingCharacter],
    lc_pool: &[IncomingLightcone],
    waves_data: &[IncomingWave],
    max_cycles: i32,
    team_size: usize,
) -> Vec<usize> {
    let combos: Vec<Vec<usize>> = (0..char_pool.len()).combinations(team_size).collect();
    combos
        .into_par_iter()
        .map(|char_idxs| {
            let report = run_team(&char_idxs, char_pool, lc_pool, waves_data, max_cycles, false);
            (char_idxs, report)
        })
        .reduce(
            || (vec![], SimReport { total_damage: -1.0, cycles_taken: i32::MAX, logs: vec![], is_defeated: true }),
            |(best_idxs, best_rep), (idxs, rep)| {
                if is_better(&best_rep, &rep) { (idxs, rep) } else { (best_idxs, best_rep) }
            },
        )
        .0
}

// ─── Two-pass greedy relic optimizer ────────────────────────────────────────
//
// Per character per round:
//   Pass A — sweep 1 647 set combos  (body/feet/sphere/rope held fixed) →  1 647 sims
//   Pass B — sweep 1 400 main combos (relic/ornament set held fixed)    →  1 400 sims
// Total per char per round: 3 047  (vs 2 311 800 for the full product)
//
// 4 chars × 2 rounds (exhaustive) = 24 376 sims
// 4 chars × 1 round  (SA polish)  = 12 188 sims

fn optimize_relics_two_pass(
    char_idxs:   &[usize],
    char_pool:   &[IncomingCharacter],
    lc_pool:     &[IncomingLightcone],
    set_combos:  &[relics::SetCombo],
    main_combos: &[relics::MainStatCombo],
    waves_data:  &[IncomingWave],
    max_cycles:  i32,
    rounds:      usize,
) -> Vec<relics::RelicConfig> {
    let n = char_idxs.len();
    let mut best: Vec<relics::RelicConfig> = char_idxs.iter()
        .map(|&i| default_relic_config(char_pool[i].attribute.as_deref().unwrap_or("Physical")))
        .collect();

    for _ in 0..rounds {
        for slot in 0..n {
            let fixed = best.clone();

            // ── Pass A: find best (relic set, ornament set) pair ─────────────
            let winner_set = set_combos.par_iter()
                .map(|sc| {
                    let mut cfgs = fixed.clone();
                    cfgs[slot].relic_set    = sc.relic_set.clone();
                    cfgs[slot].ornament_set = sc.ornament_set.clone();
                    let rep = run_team_with_relics(
                        char_idxs, char_pool, lc_pool, &cfgs, waves_data, max_cycles, false,
                    );
                    (sc.clone(), rep)
                })
                .reduce_with(|(sc_a, rep_a), (sc_b, rep_b)| {
                    if is_better(&rep_a, &rep_b) { (sc_b, rep_b) } else { (sc_a, rep_a) }
                });

            if let Some((sc, _)) = winner_set {
                best[slot].relic_set    = sc.relic_set;
                best[slot].ornament_set = sc.ornament_set;
            }

            // ── Pass B: find best (body, feet, sphere, rope) main stats ──────
            let fixed2 = best.clone();
            let winner_mains = main_combos.par_iter()
                .map(|mc| {
                    let mut cfgs = fixed2.clone();
                    cfgs[slot].body_main   = mc.body_main.clone();
                    cfgs[slot].feet_main   = mc.feet_main.clone();
                    cfgs[slot].sphere_main = mc.sphere_main.clone();
                    cfgs[slot].rope_main   = mc.rope_main.clone();
                    let rep = run_team_with_relics(
                        char_idxs, char_pool, lc_pool, &cfgs, waves_data, max_cycles, false,
                    );
                    (mc.clone(), rep)
                })
                .reduce_with(|(mc_a, rep_a), (mc_b, rep_b)| {
                    if is_better(&rep_a, &rep_b) { (mc_b, rep_b) } else { (mc_a, rep_a) }
                });

            if let Some((mc, _)) = winner_mains {
                best[slot].body_main   = mc.body_main;
                best[slot].feet_main   = mc.feet_main;
                best[slot].sphere_main = mc.sphere_main;
                best[slot].rope_main   = mc.rope_main;
            }
        }
    }

    best
}

// ─── Joint Simulated Annealing (team + relics) ───────────────────────────────
//
// Move types (per iteration):
//   30% — swap one char slot (reset that slot's relics to element-aware default)
//   35% — change one char's (relic set, ornament set) to a random set combo
//   35% — change one char's (body, feet, sphere, rope) to a random main stat combo
//
// Total: SA_JOINT_RESTARTS × SA_JOINT_ITERATIONS = 40 000 sims.

fn sa_chain_joint(
    char_pool:   &[IncomingCharacter],
    lc_pool:     &[IncomingLightcone],
    set_combos:  &[relics::SetCombo],
    main_combos: &[relics::MainStatCombo],
    waves_data:  &[IncomingWave],
    max_cycles:  i32,
    team_size:   usize,
    seed:        u64,
) -> (JointGenome, SimReport) {
    let mut rng    = SmallRng::seed_from_u64(seed);
    let n          = char_pool.len();
    let n_sets     = set_combos.len();
    let n_mains    = main_combos.len();

    let mut all_idxs: Vec<usize> = (0..n).collect();
    all_idxs.shuffle(&mut rng);
    let char_idxs: Vec<usize> = all_idxs[..team_size].to_vec();
    let relic_configs: Vec<relics::RelicConfig> = char_idxs.iter()
        .map(|&i| default_relic_config(char_pool[i].attribute.as_deref().unwrap_or("Physical")))
        .collect();

    let mut genome         = JointGenome { char_idxs, relic_configs };
    let mut current_report = run_genome(&genome, char_pool, lc_pool, waves_data, max_cycles, false);
    let mut current_score  = score(&current_report, max_cycles);
    let mut best_genome    = genome.clone();
    let mut best_report    = current_report.clone();
    let mut best_score     = current_score;

    let alpha = (SA_T_END / SA_T_START).powf(1.0 / SA_JOINT_ITERATIONS as f64);
    let mut t = SA_T_START;

    for _ in 0..SA_JOINT_ITERATIONS {
        let mut neighbor = genome.clone();
        let r = rng.gen::<f64>();

        if r < 0.30 {
            // Swap one char slot; reset that slot's relics to element-aware default.
            let slot = rng.gen_range(0..team_size);
            let candidates: Vec<usize> = (0..n).filter(|x| !genome.char_idxs.contains(x)).collect();
            if candidates.is_empty() { t *= alpha; continue; }
            let new_char = *candidates.choose(&mut rng).unwrap();
            neighbor.char_idxs[slot]     = new_char;
            neighbor.relic_configs[slot] = default_relic_config(
                char_pool[new_char].attribute.as_deref().unwrap_or("Physical"),
            );
        } else if r < 0.65 {
            // Change the (relic set, ornament set) for one character.
            let slot = rng.gen_range(0..team_size);
            let sc = &set_combos[rng.gen_range(0..n_sets)];
            neighbor.relic_configs[slot].relic_set    = sc.relic_set.clone();
            neighbor.relic_configs[slot].ornament_set = sc.ornament_set.clone();
        } else {
            // Change the (body, feet, sphere, rope) main stats for one character.
            let slot = rng.gen_range(0..team_size);
            let mc = &main_combos[rng.gen_range(0..n_mains)];
            neighbor.relic_configs[slot].body_main   = mc.body_main.clone();
            neighbor.relic_configs[slot].feet_main   = mc.feet_main.clone();
            neighbor.relic_configs[slot].sphere_main = mc.sphere_main.clone();
            neighbor.relic_configs[slot].rope_main   = mc.rope_main.clone();
        }

        let neighbor_report = run_genome(&neighbor, char_pool, lc_pool, waves_data, max_cycles, false);
        let neighbor_score  = score(&neighbor_report, max_cycles);

        let delta  = neighbor_score - current_score;
        let accept = delta > 0.0 || rng.gen::<f64>() < (delta / t).exp();

        if accept {
            genome         = neighbor;
            current_report = neighbor_report;
            current_score  = neighbor_score;

            if current_score > best_score {
                best_score  = current_score;
                best_genome = genome.clone();
                best_report = current_report.clone();
            }
        }

        t *= alpha;
    }

    (best_genome, best_report)
}

fn sa_search_joint(
    char_pool:   &[IncomingCharacter],
    lc_pool:     &[IncomingLightcone],
    set_combos:  &[relics::SetCombo],
    main_combos: &[relics::MainStatCombo],
    waves_data:  &[IncomingWave],
    max_cycles:  i32,
    team_size:   usize,
) -> JointGenome {
    let dummy = || (
        JointGenome { char_idxs: vec![], relic_configs: vec![] },
        SimReport { total_damage: -1.0, cycles_taken: i32::MAX, logs: vec![], is_defeated: true },
    );
    (0..SA_JOINT_RESTARTS as u64)
        .into_par_iter()
        .map(|seed| sa_chain_joint(
            char_pool, lc_pool, set_combos, main_combos,
            waves_data, max_cycles, team_size, seed * 0xC0DE_BABE_u64,
        ))
        .reduce(dummy, |(best_g, best_r), (g, r)| {
            if is_better(&best_r, &r) { (g, r) } else { (best_g, best_r) }
        })
        .0
}

// ─── HTTP handler ────────────────────────────────────────────────────────────

async fn optimize_handler(Json(request): Json<OptimizeRequest>) -> Json<OptimizeResult> {
    let payload    = request.payload;
    let char_pool  = payload.character_pool;
    let lc_pool    = payload.lightcone_pool.unwrap_or_default();
    let waves_data = payload.waves;
    let max_cycles = payload.settings.as_ref().and_then(|s| s.max_cycles).unwrap_or(5);

    let team_size = 4.min(char_pool.len());
    let n_chars   = char_pool.len();

    let total_combos = n_combinations(n_chars, team_size);

    // Pre-build the two independent search axes once (shared across all calls).
    let set_combos  = relics::all_set_combos();   // 1 647 entries
    let main_combos = relics::all_main_stat_combos(); // 1 400 entries
    let sims_per_char_per_round = set_combos.len() + main_combos.len(); // 3 047

    let (best_char_idxs, best_relic_configs, simulations_count) =
        if total_combos <= EXHAUSTIVE_THRESHOLD {
            // Exhaustive team sweep with default relics, then two-pass relic polish.
            let idxs      = exhaustive_search(&char_pool, &lc_pool, &waves_data, max_cycles, team_size);
            let relic_sims = idxs.len() * 2 * sims_per_char_per_round;
            let cfgs = optimize_relics_two_pass(
                &idxs, &char_pool, &lc_pool,
                &set_combos, &main_combos,
                &waves_data, max_cycles, 2,
            );
            (idxs, cfgs, total_combos + relic_sims)
        } else {
            // Joint SA: co-optimise team composition and relics together.
            let genome = sa_search_joint(
                &char_pool, &lc_pool,
                &set_combos, &main_combos,
                &waves_data, max_cycles, team_size,
            );
            // Quick two-pass polish (1 round) to sharpen relic choices.
            let polish_sims = genome.char_idxs.len() * sims_per_char_per_round;
            let cfgs = optimize_relics_two_pass(
                &genome.char_idxs, &char_pool, &lc_pool,
                &set_combos, &main_combos,
                &waves_data, max_cycles, 1,
            );
            let total = SA_JOINT_RESTARTS * SA_JOINT_ITERATIONS + polish_sims;
            (genome.char_idxs, cfgs, total)
        };

    // Re-run the winning team with best relics and full logs enabled.
    let mut best_chars: Vec<IncomingCharacter> = best_char_idxs.iter().map(|&i| char_pool[i].clone()).collect();
    for (ch, cfg) in best_chars.iter_mut().zip(best_relic_configs.iter()) {
        ch.relics = Some(relics::config_to_relics(cfg));
    }
    let best_char_refs: Vec<&IncomingCharacter> = best_chars.iter().collect();
    let best_lcs = assign_lcs_greedily(&best_char_refs, &lc_pool);
    let best_report = run_simulation(&best_chars, &best_lcs, &waves_data, max_cycles, true);

    let best_team_names: Vec<String> = best_chars.iter().zip(best_lcs.iter())
        .map(|(c, lc)| {
            let char_name = c.name.clone().unwrap_or_else(|| c.character_id.clone());
            let lc_name   = lc.and_then(|l| l.name.clone()).unwrap_or_else(|| "None".to_string());
            format!("{} ({})", char_name, lc_name)
        })
        .collect();

    // cfg.relic_set is already a display label (e.g. "Pioneer 4p").
    // ornament_display maps the short code to a display name.
    let best_relics: Vec<CharRelicConfig> = best_chars.iter().zip(best_relic_configs.iter())
        .map(|(c, cfg)| CharRelicConfig {
            character_name: c.name.clone().unwrap_or_else(|| c.character_id.clone()),
            relic_set:      cfg.relic_set.clone(),
            ornament_set:   relics::ornament_display(&cfg.ornament_set).to_string(),
            body_main:      cfg.body_main.clone(),
            feet_main:      cfg.feet_main.clone(),
            sphere_main:    cfg.sphere_main.clone(),
            rope_main:      cfg.rope_main.clone(),
        })
        .collect();

    Json(OptimizeResult {
        best_team:         best_team_names,
        total_damage:      best_report.total_damage.floor(),
        cycles:            best_report.cycles_taken,
        logs:              format_logs(&best_report),
        simulations_count,
        is_defeated:       best_report.is_defeated,
        best_relics,
    })
}

// ─── Utility ─────────────────────────────────────────────────────────────────

/// C(n, k) without floating point.
fn n_combinations(n: usize, k: usize) -> usize {
    if k > n { return 0; }
    let k = k.min(n - k);
    let mut result = 1usize;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}
