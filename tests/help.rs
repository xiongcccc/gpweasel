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
