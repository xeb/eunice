use std::process::{Command, Output};

fn run(home: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_eunice"))
        .env_clear()
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .env("EUNICE_LLAMA_SERVER", "/nonexistent/eunice-config-test")
        .current_dir(home)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn configured_local_default_reaches_local_runtime_without_model_flag() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir(home.path().join(".eunice")).unwrap();
    std::fs::write(home.path().join(".eunice/config.toml"), "default_model = 'hf:qwen3.5:2b'\n").unwrap();
    let output = run(home.path(), &["hello"]);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    // Fails before download because the deliberately absent runtime proves routing.
    assert!(error.contains("Install llama-server"), "{error}");
    assert!(!home.path().join(".eunice/models/Qwen3.5-2B-Q4_K_M.gguf").exists());
}

#[test]
fn explicit_model_and_maintenance_commands_bypass_bad_default_file() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir(home.path().join(".eunice")).unwrap();
    std::fs::write(home.path().join(".eunice/config.toml"), "default_model = 42\n").unwrap();
    let output = run(home.path(), &["hello"]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("Invalid configuration"));
    let output = run(home.path(), &["--model", "hf:qwen3.5:2b", "hello"]);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("Install llama-server"), "{error}");
    assert!(!error.contains("Invalid configuration"));
    assert!(run(home.path(), &["--version"]).status.success());
}
