# Performance measurement (issue #34)

## Design targets

See `docs/design/02-tech-and-performance.md` §3.

## Running locally

```bash
# Synthetic fixture (default) + worker sweep + soft threshold check
cargo run --release --bin gitbolt-bench -- --sweep-workers --check-thresholds

# Large external repository
GITBOLT_BENCH_REPO=/path/to/rust cargo run --release --bin gitbolt-bench -- --sweep-workers

# Override worker pool used by the app
GITBOLT_WORKERS=6 cargo run
```

## CI

The `Perf regression (warn)` job step runs the synthetic suite with `--warn-only`.
Threshold exceedances emit GitHub `::warning::` annotations but do not fail the build.
Hard failures without `--warn-only` exit non-zero for local strict checks.

## Worker sizing

`--sweep-workers` times parallel `open+status` jobs for 1/2/4/6/8 workers and
recommends the smallest count within 10% of the best wall time. Default remains
4 (`GITBOLT_WORKERS`).
