//! Cadrion CLI binary.

mod assembly_cmd;
mod bench_cmd;
mod build_cmd;
mod cli;
mod export_cmd;
mod fab_cmd;
mod harness_cmd;
mod inspect_cmd;
mod kernel_pick;
mod mcp_cmd;
mod migrate_cmd;
mod output;
mod robot_cmd;
mod schema_cmd;
mod sdf_cmd;
mod serve_cmd;
mod snapshot_cmd;
mod topo_from_ir;
mod view_cmd;

use clap::Parser;
use cli::{Cli, Commands};
use output::{emit, ExitCode};

fn main() {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    let code = match &cli.command {
        Commands::Build(args) => build_cmd::run(&cli, args),
        Commands::Inspect(args) => inspect_cmd::run(&cli, args),
        Commands::Export(args) => export_cmd::run(&cli, args),
        Commands::Bench(args) => bench_cmd::run(&cli, args),
        Commands::Harness(args) => harness_cmd::run(&cli, args),
        Commands::Snapshot(args) => snapshot_cmd::run(&cli, args),
        Commands::View(args) => view_cmd::run(&cli, args),
        Commands::Mcp(args) => mcp_cmd::run_mcp(&cli, args),
        Commands::Skills(args) => mcp_cmd::run_skills(&cli, args),
        Commands::Serve(args) => serve_cmd::run(&cli, args),
        Commands::Robot(args) => robot_cmd::run(&cli, args),
        Commands::Assembly(args) => assembly_cmd::run(&cli, args),
        Commands::Fab(args) => fab_cmd::run_fab(&cli, args),
        Commands::Printer(args) => fab_cmd::run_printer(&cli, args),
        Commands::Migrate(args) => migrate_cmd::run(&cli, args),
        Commands::Sdf(args) => sdf_cmd::run(&cli, args),
        Commands::Schema(args) => schema_cmd::run(&cli, args),
        Commands::Version => {
            let v = serde_json::json!({
                "ok": true,
                "cadrion": env!("CARGO_PKG_VERSION"),
                "kernel_default": kernel_pick::default_kernel_id(),
                "features": {
                    "occt": cfg!(feature = "occt"),
                    "truck": true,
                    "sdf_secondary": true,
                },
                "crates": {
                    "cadrion_kernel": cadrion_kernel::VERSION,
                    "cadrion_lang": cadrion_lang::VERSION,
                    "cadrion_model": cadrion_model::VERSION,
                    "cadrion_inspect": cadrion_inspect::VERSION,
                    "cadrion_render": cadrion_render::VERSION,
                    "cadrion_bench": cadrion_bench::VERSION,
                    "cadrion_mcp": cadrion_mcp::VERSION,
                    "cadrion_parts": cadrion_parts::VERSION,
                    "cadrion_api": cadrion_api::VERSION,
                    "cadrion_robot": cadrion_robot::VERSION,
                    "cadrion_fab": cadrion_fab::VERSION,
                    "cadrion_harness": cadrion_harness::VERSION,
                    "cadrion_truck": cadrion_truck::VERSION,
                    "cadrion_sdf": cadrion_sdf::VERSION,
                },
                "kernels": {
                    "default": kernel_pick::default_kernel_id(),
                    "truck_parity_eligible": false,
                    "truck_implementation": "truck-seed-analytic-csg",
                    "truck_brep_spike": cadrion_truck::BREP_SPIKE,
                    "truck_note": "seed = analytic CSG; truck-brep = optional H3-6 upstream spike; both NON-PARITY; never default",
                    "occt_cone": "unsupported (no silent cylinder stand-in; H3-1)",
                },
                "sdf": {
                    "primary": false,
                    "note": "H2-9 secondary analytic SDF only — never modeling path",
                },
                "metrics_doc": "docs/METRICS.md",
                "licensing_doc": "docs/LICENSING.md",
            });
            emit(cli.json, &v, true);
            ExitCode::Ok
        }
    };
    std::process::exit(code as i32);
}

fn init_tracing(verbose: bool) {
    let filter = if verbose { "debug" } else { "warn" };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}
