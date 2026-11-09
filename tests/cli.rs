use std::process::{Command, Stdio};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_guineanote"))
}

#[test]
fn help_and_version_need_no_terminal() {
    let help = binary().arg("--help").output().unwrap();
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    assert!(text.contains("--readonly"));
    assert!(text.contains("--config"));
    let version = binary().arg("--version").output().unwrap();
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).unwrap().trim(),
        "guineanote 0.1.0"
    );
}

#[test]
fn file_and_folder_modes_require_an_interactive_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let output = binary().arg(dir.path()).output().unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("interactive terminal")
    );
    let output = binary().stdin(Stdio::null()).output().unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("interactive terminal")
    );
    assert!(!output.stdout.contains(&0x1b));
}

#[test]
fn invalid_config_reports_error_before_touching_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("bad.toml");
    std::fs::write(&config, "tab_width = 0").unwrap();
    let output = binary().arg("--config").arg(config).output().unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("tab_width")
    );
    assert!(!output.stdout.contains(&0x1b));
}
