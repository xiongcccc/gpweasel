use assert_cmd::cargo;
use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn simple_connection_aggregate() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(cargo::cargo_bin!("gpweasel"));

    cmd.args(["conn", "./tests/files/azure_connections.log"])
        .assert()
        .success()
        .stdout(predicates::str::contains("5  2025-05-21 11:00:00"));

    Ok(())
}

#[test]
fn greenplum_connection_aggregate() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(cargo::cargo_bin!("gpweasel"));

    cmd.args(["conn", "./tests/files/gpdb_sample.csv"])
        .assert()
        .success()
        .stdout(predicates::str::contains("1  10.1.2.3"))
        .stdout(predicates::str::contains("1  gpadmin"))
        .stdout(predicates::str::contains("1  sales"));

    Ok(())
}

#[test]
fn greenplum_csv_record_in_log_file_is_skipped_when_not_connection()
    -> Result<(), Box<dyn std::error::Error>>
{
    let mut cmd = Command::new(cargo::cargo_bin!("gpweasel"));

    cmd.args(["conn", "./tests/files/gpdb_startup_like.log"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Connections by host:"));

    Ok(())
}
