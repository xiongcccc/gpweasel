use assert_cmd::cargo;
use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn greenplum_stats_summary() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(cargo::cargo_bin!("gpweasel"));

    cmd.args(["stats", "./tests/files/gpdb_sample.csv"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Log summary:"))
        .stdout(predicates::str::contains("total events: 4"))
        .stdout(predicates::str::contains("duration events: 1"))
        .stdout(predicates::str::contains(
            "records without user/database/host: 0/0/0",
        ))
        .stdout(predicates::str::contains("connection attempts: 1"))
        .stdout(predicates::str::contains("authenticated connections: 1"))
        .stdout(predicates::str::contains("Top users:"))
        .stdout(predicates::str::contains("4  gpadmin"));

    Ok(())
}

#[test]
fn greenplum_peaks_summary() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(cargo::cargo_bin!("gpweasel"));

    cmd.args([
        "peaks",
        "--bucket",
        "1m",
        "--max",
        "1",
        "./tests/files/gpdb_sample.csv",
    ])
    .assert()
    .success()
    .stdout(predicates::str::contains("Top 1 busiest time buckets:"))
    .stdout(predicates::str::contains("4  2026-06-03 10:15:00"));

    Ok(())
}
