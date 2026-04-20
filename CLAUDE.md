# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### TypeScript API Server (`simulation-api/`)
```bash
npm run dev          # Start dev server (tsx server.ts, port 3000)
npm test             # Run tests (not yet configured)
```

### Rust Optimizer Engine (`simulation-api/hsr-optimizer-engine/`)
```bash
cargo build                  # Debug build
cargo build --release        # Optimized release build (always use for perf testing)
cargo run --release          # Run the optimizer server (port 3000)
cargo test                   # Run all tests
cargo test <test_name>       # Run a single test
```

### Release Profile
`Cargo.toml` already has a `[profile.release]` section with `lto = "thin"`, `codegen-units = 1`, `opt-level = 3`, `strip = true`. Always build with `--release` for any benchmarking.

## Architecture

This is a **dual-language system**: a TypeScript Fastify server handles API routing and mapping, while a Rust Axum server performs the heavy optimization work.

### Request Flow
1. Client sends a JSON optimize request (see `hsr_request_optimize_*.json` for format)
2. TypeScript server (`server.ts`, port 3000) maps game DB UUIDs to typed structures
3. The Rust optimizer (`hsr-optimizer-engine/`, also port 3000) receives `OptimizeRequest` and runs:
   - **Team search**: exhaustive if C(N,4) ≤ 15,000 combos; otherwise Joint Simulated Annealing (10 restarts × 4,000 iterations)
   - **Relic polish**: two-pass greedy (Pass A: ~1,647 set combos; Pass B: ~1,400 main stat combos)
4. Returns `OptimizeResult`: best team, total damage, cycle count, simulation logs, relic configs

### Rust Engine Structure (`src/`)

| File | Role |
|------|------|
| `main.rs` | Axum HTTP server, optimization orchestration (exhaustive/SA/greedy loops) |
| `simulator.rs` | Turn-based combat engine: Action Value queue, buff snapshots, hook dispatch |
| `damage.rs` | Damage formula pipeline (CRIT → DMG boost → DEF → RES → vulnerability → mitigation → broken) |
| `models.rs` | All data types: `IncomingCharacter`, `SimState`, `TeamMember`, `SimEnemy`, `SimReport` |
| `effects.rs` | Status effect application, duration ticking, `SimEnemy` cache recomputation |
| `ids.rs` | Stat UUID constants and character/enemy ID string literals |

Character, lightcone, relic, and planar set logic live in their own subdirectories and are dispatched via `match` on character/set IDs in the hook functions.

### Action Value (AV) Turn System
Turns are ordered by lowest AV. Each actor's next AV = current + 10000 / SPD. Character hooks fire around each turn: `on_battle_start → on_turn_start → on_before_action → [damage] → on_after_action → on_ult → on_break`.

### Status Effect Caching
`SimEnemy` carries four pre-computed cache fields (`cached_def_reduce`, `cached_all_res_reduce`, `cached_weakness_res_reduce`, `cached_vuln_bonus`) that are updated whenever debuffs/buffs are applied or expire. **Always call `effects::recompute_enemy_caches(enemy)`** after manually inserting into or removing from `enemy.active_debuffs` / `enemy.active_buffs` — otherwise damage calculations will use stale values.

### Key Typing Choices
- `TeamMember.stacks` and `.turn_counters` use `HashMap<&'static str, f64>` — all keys must be string literals, never `format!()` strings. Dynamic keys (e.g. per-target counters) must go on `SimState.stacks: HashMap<String, f64>` instead.
- `SimState.stacks` is a catch-all for cross-character or dynamic string keys.

### Adding Characters / Relics / Lightcones
Refer to the docs in `hsr-optimizer-engine/docs/`:
- `ADDING_CHARACTERS.md` — hook signatures, required fields, worked examples
- `ADDING_RELICS.md`, `ADDING_PLANARS.md`, `ADDING_LIGHTCONES.md`, `ADDING_ENEMIES.md`

Character implementations go in `src/characters/<name>.rs` and are registered via a `match` arm in the simulator's hook dispatcher.

### TypeScript Implementation
`simulation-api/hsr/` contains an older TypeScript simulator that mirrors the Rust engine. It can run simulations directly (used by `server.ts` for quick single-sim calls) but the Rust engine is the performance path for optimization. Stat UUID mappings are documented in `HSR_ID_MAPPING.md`.
