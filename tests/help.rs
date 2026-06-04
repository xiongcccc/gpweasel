use assert_cmd::cargo;
use assert_cmd::prelude::*; // Add methods on commands
use std::process::Command; // Run programs

#[test]
fn base_help_with_options() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(cargo::cargo_bin!("gpweasel"));

    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("gpweasel [OPTIONS] <COMMAND>"));

    Ok(())
}

#[test]
fn base_help_contains_page_size() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(cargo::cargo_bin!("gpweasel"));

    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("--page-size"));

    Ok(())
}

#[test]
fn errors_command_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(cargo::cargo_bin!("gpweasel"));

    cmd.args(["errors", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "gpweasel errors [OPTIONS] <PATH>...",
        ));

    Ok(())
}

#[test]
fn errors_command_with_sub_command_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(cargo::cargo_bin!("gpweasel"));

    cmd.args(["errors", "list", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "gpweasel errors list [OPTIONS] <PATH>...",
        ));

    Ok(())
}

#[test]
fn slow_command_help_contains_threshold() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(cargo::cargo_bin!("gpweasel"));

    cmd.args(["slow", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("slow <THRESHOLD>"));

    Ok(())
}

#[test]
fn slow_command_help_contains_subcommand_top() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(cargo::cargo_bin!("gpweasel"));

    cmd.args(["slow", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("top"));

    Ok(())
}

#[test]
fn peaks_command_help_contains_bucket_and_max() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(cargo::cargo_bin!("gpweasel"));

    cmd.args(["peaks", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("--bucket"))
        .stdout(predicates::str::contains("--max"));

    Ok(())
}

#[test]
fn stats_command_help_accepts_files() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(cargo::cargo_bin!("gpweasel"));

    cmd.args(["stats", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("gpweasel stats <PATH>..."));

    Ok(())
}
