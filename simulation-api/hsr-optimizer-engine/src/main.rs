mod models;
mod simulator;

use axum::{
    routing::post,
    Json, Router,
};
use models::*;
use simulator::run_simulation;
use itertools::Itertools;
use rayon::prelude::*;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Build our application with a route
    let app = Router::new()
        .route("/optimize", post(optimize_handler))
        .layer(CorsLayer::permissive());

    // Run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn optimize_handler(Json(payload): Json<OptimizeRequest>) -> Json<OptimizeResult> {
    let payload = payload.payload;
    let character_pool = payload.character_pool;
    let waves = payload.waves;
    let settings = payload.settings;

    // Generate combinations of 4 characters
    let combinations: Vec<Vec<Character>> = character_pool.into_iter()
        .combinations(4)
        .collect();

    // Run simulations in parallel
    let results: Vec<(Vec<String>, SimReport)> = combinations.into_par_iter()
        .map(|team| {
            let team_names = team.iter().map(|c| c.name.clone()).collect();
            let report = run_simulation(team, waves.clone(), settings.clone());
            (team_names, report)
        })
        .collect();

    // Find the best team
    let simulations_count = results.len();
    let best = results.into_iter()
        .max_by(|a, b| {
            let a_report = &a.1;
            let b_report = &b.1;

            // 1. Survival: !is_defeated is better
            if a_report.is_defeated != b_report.is_defeated {
                if !a_report.is_defeated { return std::cmp::Ordering::Greater; }
                return std::cmp::Ordering::Less;
            }

            // 2. Cycles: lower is better
            if a_report.cycles_taken != b_report.cycles_taken {
                return b_report.cycles_taken.cmp(&a_report.cycles_taken);
            }

            // 3. Damage: higher is better
            a_report.total_damage.partial_cmp(&b_report.total_damage).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap();

    Json(OptimizeResult {
        best_team: best.0,
        total_damage: best.1.total_damage,
        cycles: best.1.cycles_taken,
        simulations_count,
        is_defeated: best.1.is_defeated,
    })
}
