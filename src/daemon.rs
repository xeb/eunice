use anyhow::{anyhow, Result};

/// Options captured from the CLI at install time and baked into the unit's ExecStart.
pub struct InstallOptions {
    pub port: u16,
    pub host: String,
    pub agents_file: Option<String>,
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub no_persist: bool,
}

/// The environment variables snapshotted into ~/.eunice/eunice.env. PATH is included
/// because systemd user services get a minimal PATH and the Bash tool spawns $SHELL -c
/// with the service environment, so agent commands would not find ~/.cargo/bin etc.
pub const SNAPSHOT_ENV_VARS: &[&str] = &[
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "GEMINI_API_KEY",
    "GOOGLE_API_KEY",
    "OLLAMA_HOST",
    "AZURE_OPENAI_ENDPOINT",
    "AZURE_OPENAI_API_KEY",
    "AZURE_OPENAI_API_VERSION",
    "GEMMAD_HOST",
    "GEMMAD_PORT",
    "GEMMAD_MODEL_ID",
    "GEMMAD_API_KEY",
    "GEMMAD_KEYS_FILE",
    "PATH",
];

const SERVICE_NAME: &str = "eunice.service";

/// Characters that never need quoting inside a systemd ExecStart word.
fn is_safe_arg_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '=' | '@' | '+' | ',')
}

/// Quote an ExecStart argument unless it is a plain safe token. Inside the quoted
/// form backslashes and double quotes are escaped; `%` is doubled because systemd
/// expands specifiers in unit values regardless of quoting.
fn quote_arg(arg: &str) -> String {
    if !arg.is_empty() && arg.chars().all(is_safe_arg_char) {
        return arg.to_string();
    }
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    for c in arg.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '%' => out.push_str("%%"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Build the ExecStart command line from the resolved binary path and options.
/// Every path must already be absolute. Shell-quotes arguments containing spaces.
pub fn build_exec_start(binary: &str, opts: &InstallOptions) -> String {
    let mut parts = vec![quote_arg(binary), "--webapp".to_string()];
    parts.push("--port".to_string());
    parts.push(opts.port.to_string());
    parts.push("--host".to_string());
    parts.push(quote_arg(&opts.host));

    if let Some(ref model) = opts.model {
        parts.push("--model".to_string());
        parts.push(quote_arg(model));
    }
    if let Some(ref agents) = opts.agents_file {
        parts.push("--agents".to_string());
        parts.push(quote_arg(agents));
    }
    if let Some(ref prompt) = opts.prompt {
        parts.push("--prompt".to_string());
        parts.push(quote_arg(prompt));
    }
    if opts.no_persist {
        parts.push("--no-persist".to_string());
    }

    parts.join(" ")
}

/// Render the systemd unit file. Pure — no filesystem or process access.
pub fn render_unit(exec_start: &str, working_dir: &str, home: &str) -> String {
    format!(
        "[Unit]\n\
         Description=Eunice agentic webapp server\n\
         After=network-online.target\n\
         \n\
         [Service]\n\
         Type=simple\n\
         ExecStart={}\n\
         WorkingDirectory={}\n\
         EnvironmentFile=-{}/.eunice/eunice.env\n\
         Restart=on-failure\n\
         RestartSec=5\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        exec_start, working_dir, home
    )
}

/// Render the EnvironmentFile body from (name, value) pairs already read from the
/// environment. Pure, so it can be tested without mutating the process env.
pub fn render_env_file(vars: &[(String, String)]) -> String {
    let mut out = String::from("# Written by `eunice --install`. API keys snapshotted at install time.\n");
    for (name, value) in vars {
        out.push_str(&format!("{}={}\n", name, value));
    }
    out
}

/// Collect the currently-set, non-empty vars from SNAPSHOT_ENV_VARS.
fn snapshot_env_vars() -> Vec<(String, String)> {
    SNAPSHOT_ENV_VARS
        .iter()
        .filter_map(|name| {
            let value = std::env::var(name).ok()?;
            if value.trim().is_empty() {
                None
            } else {
                Some((name.to_string(), value))
            }
        })
        .collect()
}

fn home_dir() -> Result<std::path::PathBuf> {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| anyhow!("HOME is not set; cannot locate the systemd user unit directory"))
}

fn unit_path() -> Result<std::path::PathBuf> {
    Ok(home_dir()?
        .join(".config")
        .join("systemd")
        .join("user")
        .join(SERVICE_NAME))
}

/// Install eunice --webapp as a systemd user service and start it.
#[cfg(target_os = "linux")]
pub fn run_install(opts: &InstallOptions) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    use std::process::{Command, Stdio};

    println!("Installing eunice as a systemd user service...");
    println!();

    if Command::new("systemctl").arg("--version").output().is_err() {
        return Err(anyhow!(
            "systemctl is not available. A systemd-based Linux distribution is required."
        ));
    }

    // Validate the agents file before writing anything — never install a unit that
    // would crash-loop on startup.
    let agents_file = match opts.agents_file {
        Some(ref path) => {
            let resolved = std::fs::canonicalize(path)
                .map_err(|e| anyhow!("failed to resolve agents file '{}': {}", path, e))?;
            crate::agents::load_agents(&resolved)?;
            println!("Validated agents file: {}", resolved.display());
            Some(resolved.display().to_string())
        }
        None => None,
    };

    let binary = std::env::current_exe()?;
    let working_dir = std::env::current_dir()?;
    let home = home_dir()?;

    let eunice_dir = home.join(".eunice");
    std::fs::create_dir_all(&eunice_dir)?;
    let env_path = eunice_dir.join("eunice.env");
    let vars = snapshot_env_vars();
    // Create with 0600 up front so the keys are never briefly world-readable, then
    // tighten again in case the file already existed with looser permissions.
    let mut env_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&env_path)?;
    env_file.write_all(render_env_file(&vars).as_bytes())?;
    env_file.flush()?;
    std::fs::set_permissions(&env_path, std::fs::Permissions::from_mode(0o600))?;

    if vars.is_empty() {
        println!();
        println!("WARNING: no API keys were found in the current environment.");
        println!("The service will start with NO API credentials and every request will fail.");
        println!("Export your keys and re-run --install, or edit {} by hand.", env_path.display());
        println!();
    } else {
        println!("Captured {} API key(s) into {}", vars.len(), env_path.display());
    }

    let resolved_opts = InstallOptions {
        port: opts.port,
        host: opts.host.clone(),
        agents_file,
        model: opts.model.clone(),
        prompt: opts.prompt.clone(),
        no_persist: opts.no_persist,
    };
    let exec_start = build_exec_start(&binary.display().to_string(), &resolved_opts);
    let unit = render_unit(
        &exec_start,
        &working_dir.display().to_string(),
        &home.display().to_string(),
    );

    let unit_file = unit_path()?;
    if let Some(parent) = unit_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&unit_file, unit)?;
    println!("Wrote unit file: {}", unit_file.display());
    println!();

    let status = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if !status.success() {
        return Err(anyhow!("systemctl --user daemon-reload failed"));
    }

    let status = Command::new("systemctl")
        .args(["--user", "enable", SERVICE_NAME])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if !status.success() {
        return Err(anyhow!(
            "systemctl --user enable {} failed. Check the output above for details.",
            SERVICE_NAME
        ));
    }

    let was_active = Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", SERVICE_NAME])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    // `enable --now` only starts the unit, which is a no-op when it is already running,
    // so a reinstall would keep serving the old config. Restart unconditionally.
    let status = Command::new("systemctl")
        .args(["--user", "restart", SERVICE_NAME])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if !status.success() {
        return Err(anyhow!(
            "systemctl --user restart {} failed. Check the output above for details.",
            SERVICE_NAME
        ));
    }
    println!(
        "{} the service with the new configuration.",
        if was_active { "Restarted" } else { "Started" }
    );

    // Lingering is what keeps the service alive without a login session. Some distros
    // gate it behind polkit, so a failure is a warning rather than an install failure.
    let user = std::env::var("USER").unwrap_or_default();
    let linger_ok = if user.is_empty() {
        false
    } else {
        Command::new("loginctl")
            .args(["enable-linger", &user])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };
    if !linger_ok {
        println!();
        println!("WARNING: could not enable lingering, so the service will stop when you log out.");
        println!(
            "Run manually: loginctl enable-linger {}",
            if user.is_empty() { "<your-user>" } else { &user }
        );
    }

    println!();
    println!("Install complete!");
    println!("  Unit:   {}", unit_file.display());
    println!("  URL:    http://{}:{}", resolved_opts.host, resolved_opts.port);
    if let Some(ref agents) = resolved_opts.agents_file {
        println!("  Agents: {}", agents);
    }
    println!("  Logs:   journalctl --user -u eunice -f");
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn run_install(_opts: &InstallOptions) -> Result<()> {
    Err(anyhow!(
        "--install requires Linux (systemd user services are not available on this platform)"
    ))
}

/// Stop, disable, and remove the systemd user service installed by --install.
#[cfg(target_os = "linux")]
pub fn run_uninstall_service() -> Result<()> {
    use std::process::{Command, Stdio};

    println!("Removing the eunice systemd user service...");
    println!();

    if Command::new("systemctl").arg("--version").output().is_err() {
        return Err(anyhow!(
            "systemctl is not available. A systemd-based Linux distribution is required."
        ));
    }

    // Tolerate failure here: the service may never have been installed.
    let _ = Command::new("systemctl")
        .args(["--user", "disable", "--now", SERVICE_NAME])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    let unit_file = unit_path()?;
    if unit_file.exists() {
        std::fs::remove_file(&unit_file)?;
        println!("Removed unit file: {}", unit_file.display());
    } else {
        println!("No unit file at {} (nothing to remove).", unit_file.display());
    }

    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    println!();
    println!("Uninstall complete!");
    println!();
    println!("Note: the following were left untouched:");
    println!("  ~/.eunice/eunice.env  (snapshotted API keys)");
    println!("  sessions.db           (webapp session history)");
    println!("  user lingering        (disable with: loginctl disable-linger $USER)");
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn run_uninstall_service() -> Result<()> {
    Err(anyhow!(
        "--uninstall-service requires Linux (systemd user services are not available on this platform)"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> InstallOptions {
        InstallOptions {
            port: 9000,
            host: "0.0.0.0".to_string(),
            agents_file: None,
            model: None,
            prompt: None,
            no_persist: false,
        }
    }

    #[test]
    fn test_build_exec_start_minimal() {
        let cmd = build_exec_start("/usr/bin/eunice", &opts());
        assert_eq!(cmd, "/usr/bin/eunice --webapp --port 9000 --host 0.0.0.0");
    }

    #[test]
    fn test_build_exec_start_with_model() {
        let mut o = opts();
        o.model = Some("sonnet".to_string());
        let cmd = build_exec_start("/usr/bin/eunice", &o);
        assert_eq!(
            cmd,
            "/usr/bin/eunice --webapp --port 9000 --host 0.0.0.0 --model sonnet"
        );
    }

    #[test]
    fn test_build_exec_start_with_agents() {
        let mut o = opts();
        o.agents_file = Some("/home/xeb/agents.toml".to_string());
        let cmd = build_exec_start("/usr/bin/eunice", &o);
        assert_eq!(
            cmd,
            "/usr/bin/eunice --webapp --port 9000 --host 0.0.0.0 --agents /home/xeb/agents.toml"
        );
    }

    #[test]
    fn test_build_exec_start_with_prompt_and_no_persist() {
        let mut o = opts();
        o.prompt = Some("/home/xeb/prompt.md".to_string());
        o.no_persist = true;
        let cmd = build_exec_start("/usr/bin/eunice", &o);
        assert_eq!(
            cmd,
            "/usr/bin/eunice --webapp --port 9000 --host 0.0.0.0 --prompt /home/xeb/prompt.md --no-persist"
        );
    }

    #[test]
    fn test_build_exec_start_all_options() {
        let o = InstallOptions {
            port: 8811,
            host: "127.0.0.1".to_string(),
            agents_file: Some("/a/agents.toml".to_string()),
            model: Some("gpt-5".to_string()),
            prompt: Some("/a/prompt.md".to_string()),
            no_persist: true,
        };
        let cmd = build_exec_start("/usr/bin/eunice", &o);
        assert_eq!(
            cmd,
            "/usr/bin/eunice --webapp --port 8811 --host 127.0.0.1 --model gpt-5 --agents /a/agents.toml --prompt /a/prompt.md --no-persist"
        );
    }

    #[test]
    fn test_build_exec_start_quotes_paths_with_spaces() {
        let mut o = opts();
        o.agents_file = Some("/home/xeb/my agents/agents.toml".to_string());
        let cmd = build_exec_start("/opt/my tools/eunice", &o);
        assert_eq!(
            cmd,
            "\"/opt/my tools/eunice\" --webapp --port 9000 --host 0.0.0.0 --agents \"/home/xeb/my agents/agents.toml\""
        );
    }

    #[test]
    fn test_quote_arg_safe_tokens_unquoted() {
        assert_eq!(quote_arg("/usr/bin/eunice"), "/usr/bin/eunice");
        assert_eq!(quote_arg("127.0.0.1"), "127.0.0.1");
        assert_eq!(quote_arg("gpt-5"), "gpt-5");
    }

    #[test]
    fn test_quote_arg_escapes_quotes_and_backslashes() {
        assert_eq!(quote_arg("a\"b"), "\"a\\\"b\"");
        assert_eq!(quote_arg("a\\b"), "\"a\\\\b\"");
        assert_eq!(quote_arg("say \"hi\""), "\"say \\\"hi\\\"\"");
    }

    #[test]
    fn test_quote_arg_escapes_systemd_specifier() {
        assert_eq!(quote_arg("/tmp/100%done"), "\"/tmp/100%%done\"");
        assert_eq!(quote_arg("%h/agents.toml"), "\"%%h/agents.toml\"");
    }

    #[test]
    fn test_quote_arg_quotes_other_unsafe_tokens() {
        assert_eq!(quote_arg(""), "\"\"");
        assert_eq!(quote_arg("a;b"), "\"a;b\"");
        assert_eq!(quote_arg("a$b"), "\"a$b\"");
    }

    #[test]
    fn test_build_exec_start_quotes_path_with_specifier() {
        let mut o = opts();
        o.agents_file = Some("/home/xeb/50%/agents.toml".to_string());
        let cmd = build_exec_start("/usr/bin/eunice", &o);
        assert_eq!(
            cmd,
            "/usr/bin/eunice --webapp --port 9000 --host 0.0.0.0 --agents \"/home/xeb/50%%/agents.toml\""
        );
    }

    #[test]
    fn test_render_unit_exact() {
        let unit = render_unit(
            "/usr/bin/eunice --webapp --port 9000 --host 0.0.0.0",
            "/home/xeb/work",
            "/home/xeb",
        );
        assert_eq!(
            unit,
            "[Unit]\n\
             Description=Eunice agentic webapp server\n\
             After=network-online.target\n\
             \n\
             [Service]\n\
             Type=simple\n\
             ExecStart=/usr/bin/eunice --webapp --port 9000 --host 0.0.0.0\n\
             WorkingDirectory=/home/xeb/work\n\
             EnvironmentFile=-/home/xeb/.eunice/eunice.env\n\
             Restart=on-failure\n\
             RestartSec=5\n\
             \n\
             [Install]\n\
             WantedBy=default.target\n"
        );
    }

    #[test]
    fn test_render_env_file() {
        let vars = vec![
            ("OPENAI_API_KEY".to_string(), "sk-abc".to_string()),
            ("OLLAMA_HOST".to_string(), "http://localhost:11434".to_string()),
        ];
        let body = render_env_file(&vars);
        assert!(body.starts_with("# Written by `eunice --install`."));
        assert!(body.contains("OPENAI_API_KEY=sk-abc\n"));
        assert!(body.contains("OLLAMA_HOST=http://localhost:11434\n"));
    }

    #[test]
    fn test_render_env_file_empty() {
        let body = render_env_file(&[]);
        assert_eq!(
            body,
            "# Written by `eunice --install`. API keys snapshotted at install time.\n"
        );
    }

    #[test]
    fn test_snapshot_env_vars_covers_expected_keys() {
        assert!(SNAPSHOT_ENV_VARS.contains(&"OPENAI_API_KEY"));
        assert!(SNAPSHOT_ENV_VARS.contains(&"ANTHROPIC_API_KEY"));
        assert!(SNAPSHOT_ENV_VARS.contains(&"GEMINI_API_KEY"));
        assert!(SNAPSHOT_ENV_VARS.contains(&"GOOGLE_API_KEY"));
        assert!(SNAPSHOT_ENV_VARS.contains(&"OLLAMA_HOST"));
    }

    #[test]
    fn test_snapshot_env_vars_covers_azure_and_gemmad() {
        for name in [
            "AZURE_OPENAI_ENDPOINT",
            "AZURE_OPENAI_API_KEY",
            "AZURE_OPENAI_API_VERSION",
            "GEMMAD_HOST",
            "GEMMAD_PORT",
            "GEMMAD_MODEL_ID",
            "GEMMAD_API_KEY",
            "GEMMAD_KEYS_FILE",
        ] {
            assert!(SNAPSHOT_ENV_VARS.contains(&name), "missing {}", name);
        }
    }

    #[test]
    fn test_snapshot_env_vars_includes_path() {
        assert!(SNAPSHOT_ENV_VARS.contains(&"PATH"));
    }
}
