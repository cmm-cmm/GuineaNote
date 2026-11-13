use std::process::{Command, Stdio};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_guinea"))
}

#[test]
fn help_and_version_need_no_terminal() {
    let help = binary().arg("--help").output().unwrap();
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    assert!(text.contains("--readonly"));
    assert!(text.contains("--config"));
    assert!(text.contains("--language"));
    assert!(text.contains("Usage: guinea"));
    assert!(text.contains("guinea a.txt b.txt"));
    let version = binary().arg("--version").output().unwrap();
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).unwrap().trim(),
        concat!("guinea ", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn unsupported_language_fails_before_entering_terminal() {
    let output = binary().args(["--language", "fr"]).output().unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("invalid value")
    );
    assert!(!output.stdout.contains(&0x1b));
}

#[test]
fn folders_are_rejected_and_files_require_an_interactive_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let output = binary().arg(dir.path()).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("is a folder"), "{stderr}");
    assert!(!output.stdout.contains(&0x1b));
    let file = dir.path().join("note.txt");
    let output = binary().arg(&file).output().unwrap();
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
