//! `gitbolt-bench` — Git op timings, worker sweep, threshold checks (issue #34).
//!
//! ```text
//! cargo run --release --bin gitbolt-bench -- --sweep-workers --check-thresholds --warn-only
//! GITBOLT_BENCH_REPO=/path/to/large/repo cargo run --release --bin gitbolt-bench
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use gitbolt::perf::fixture::BenchRepo;
use gitbolt::perf::suite::run_suite;
use gitbolt::perf::thresholds::Thresholds;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let warn_only = args.iter().any(|a| a == "--warn-only");
    let check = args.iter().any(|a| a == "--check-thresholds");
    let sweep = args.iter().any(|a| a == "--sweep-workers");
    let files = flag_usize(&args, "--files").unwrap_or(40);
    let commits = flag_usize(&args, "--commits").unwrap_or(12);

    let env_repo = std::env::var("GITBOLT_BENCH_REPO").ok();
    let owned = match flag_value(&args, "--repo").or(env_repo.as_deref()) {
        Some(path) => BenchRepo::open(PathBuf::from(path)),
        None => match BenchRepo::scale(files, commits) {
            Ok(r) => r,
            Err(err) => {
                eprintln!("failed to build synthetic repo: {err}");
                return ExitCode::from(2);
            }
        },
    };

    let report = run_suite(owned.path(), &Thresholds::default(), sweep);
    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(err) => {
            eprintln!("serialize: {err}");
            return ExitCode::from(2);
        }
    }

    for w in &report.warnings {
        println!("::warning::gitbolt-bench: {w}");
        eprintln!("warn: {w}");
    }
    for f in &report.failures {
        println!("::error::gitbolt-bench: {f}");
        eprintln!("fail: {f}");
    }

    if !check {
        return ExitCode::SUCCESS;
    }
    if report.failures.is_empty() {
        return ExitCode::SUCCESS;
    }
    if warn_only {
        // Soft regression: surface via annotations, keep CI green.
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn flag_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).map(String::as_str))
}

fn flag_usize(args: &[String], name: &str) -> Option<usize> {
    flag_value(args, name)?.parse().ok()
}
