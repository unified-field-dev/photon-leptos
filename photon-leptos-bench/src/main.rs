#![allow(missing_docs)]
//! photon-leptos-bench CLI entry.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr
)]

use anyhow::Result;
use clap::Parser;
use photon_leptos_bench::cli::{Cli, Command};
use photon_leptos_bench::experiments::{status_label, REGISTRY};
use photon_leptos_bench::hardware::{load_profiles, validate_hardware};
use photon_leptos_bench::{harness, matrix_run, run};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Experiments => {
            for meta in REGISTRY {
                println!(
                    "{}  {}  {}",
                    meta.id,
                    status_label(meta.status),
                    meta.summary
                );
            }
            println!("See photon-leptos-bench/EXPERIMENTS.md");
        }
        Command::Run {
            experiment,
            hardware,
            server_url,
            server_urls,
            connections,
            rate_per_sec,
            duration_secs,
            payload_bytes,
            allow_phase2,
            substrate_report,
            report,
            ws_mode,
            key_groups,
        } => {
            let outcome = run::run_experiment_args(run::RunArgs {
                experiment,
                hardware,
                server_url,
                server_urls,
                connections,
                rate_per_sec,
                duration_secs,
                payload_bytes,
                allow_phase2,
                substrate_report,
                report,
                ws_mode,
                key_groups,
            })
            .await?;
            println!("{}", serde_json::to_string_pretty(&outcome.report)?);
        }
        Command::Matrix {
            hardware,
            slice,
            server_url,
            reports_dir,
            duration_secs,
            allow_phase2,
            skip_existing,
            ws_mode,
        } => {
            matrix_run::run_matrix(matrix_run::MatrixRunOptions {
                hardware,
                slice,
                server_url,
                reports_dir,
                duration_secs,
                allow_phase2,
                skip_existing,
                ws_mode,
            })
            .await?;
        }
        Command::Hardware {
            profile,
            allow_phase2,
        } => {
            let p = validate_hardware(&profile, allow_phase2)?;
            let detail = harness::capture_hardware();
            let file = load_profiles()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "profile": profile,
                    "config": p,
                    "hardware_detail": detail,
                    "loadgen_profile": file.loadgen_profile,
                }))?
            );
        }
    }
    Ok(())
}
