use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// One `[[agent]]` table as written in agents.toml.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSpec {
    pub name: String,
    pub schedule: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub prompt_file: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub working_dir: Option<String>,
}

/// Top level of agents.toml.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentsFile {
    #[serde(default, rename = "agent")]
    pub agents: Vec<AgentSpec>,
}

/// A fully validated agent, ready to schedule.
#[derive(Debug, Clone)]
pub struct LoadedAgent {
    pub name: String,
    /// The original 5-field expression, for display.
    pub schedule_expr: String,
    /// The translated 6-field expression handed to the cron crate.
    pub schedule_normalized: String,
    pub schedule: cron::Schedule,
    pub model: Option<String>,
    /// Fully resolved prompt text (inline, or the contents of prompt_file).
    pub prompt: String,
    pub enabled: bool,
    pub timeout_secs: u64,
    pub working_dir: Option<PathBuf>,
}

/// The validated contents of an agents.toml.
#[derive(Debug, Clone)]
pub struct AgentsConfig {
    /// Absolute path to the agents.toml that produced this.
    pub source_path: PathBuf,
    pub agents: Vec<LoadedAgent>,
}

fn default_enabled() -> bool {
    true
}

fn default_timeout_secs() -> u64 {
    600
}

/// Translate a standard 5-field Unix cron expression into the 6-field form the
/// `cron` crate requires (seconds-first, day-of-week 1=Sunday..7=Saturday).
pub fn normalize_cron(expr: &str) -> Result<String> {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return Err(anyhow!(
            "expected a 5-field cron expression (minute hour day-of-month month day-of-week), got {} fields",
            fields.len()
        ));
    }

    let normalized = format!(
        "0 {} {} {} {} {}",
        fields[0],
        fields[1],
        fields[2],
        fields[3],
        translate_dow(fields[4])?
    );

    cron::Schedule::from_str(&normalized)
        .map_err(|e| anyhow!("cron expression '{}' is not valid: {}", expr, e))?;

    Ok(normalized)
}

/// Whether a 5-field expression restricts both day-of-month and day-of-week.
///
/// Standard Unix cron treats that combination as a union — `0 0 1 * 1` fires on the
/// 1st *or* any Monday. The `cron` crate intersects instead, firing only on a 1st
/// that is also a Monday. Callers warn rather than reject, since the intersection is
/// occasionally what someone meant, but it is almost never what a crontab user expects.
pub fn restricts_both_day_fields(expr: &str) -> bool {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return false;
    }
    let unrestricted = |f: &str| f == "*" || f == "?" || f.starts_with("*/");
    !unrestricted(fields[2]) && !unrestricted(fields[4])
}

/// Remap the day-of-week field from Unix numbering (0=Sunday) to the cron crate's
/// (1=Sunday), preserving lists, ranges, steps and alphabetic day names.
fn translate_dow(field: &str) -> Result<String> {
    let mut parts = Vec::new();

    for part in field.split(',') {
        let (range, step) = match part.split_once('/') {
            Some((range, step)) => (range, Some(step)),
            None => (part, None),
        };

        let mut endpoints = Vec::new();
        for endpoint in range.split('-') {
            endpoints.push(translate_dow_value(endpoint)?);
        }

        let mut translated = endpoints.join("-");
        if let Some(step) = step {
            translated.push('/');
            translated.push_str(step);
        }
        parts.push(translated);
    }

    Ok(parts.join(","))
}

/// Remap a single day-of-week token. Non-numeric tokens (`*`, `?`, `MON`) pass through.
fn translate_dow_value(value: &str) -> Result<String> {
    match value.parse::<u64>() {
        Ok(n @ 0..=6) => Ok((n + 1).to_string()),
        Ok(7) => Ok("1".to_string()),
        Ok(n) => Err(anyhow!(
            "day-of-week value {} is out of range (expected 0-7)",
            n
        )),
        Err(_) => Ok(value.to_string()),
    }
}

/// Parse and fully validate an agents.toml. Fails fast with a message naming the
/// offending agent and field. `model_validator` is called for each agent that
/// declares a model.
pub fn load_agents_file(
    path: &Path,
    model_validator: &dyn Fn(&str) -> Result<()>,
) -> Result<AgentsConfig> {
    let content = fs::read_to_string(path)
        .map_err(|e| anyhow!("failed to read agents file '{}': {}", path.display(), e))?;

    let parsed: AgentsFile = toml::from_str(&content)
        .map_err(|e| anyhow!("failed to parse '{}': {}", path.display(), e))?;

    let source_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let base_dir = source_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut seen_names: HashSet<String> = HashSet::new();
    let mut agents = Vec::new();

    for spec in parsed.agents {
        if !is_valid_agent_name(&spec.name) {
            return Err(anyhow!(
                "agent '{}': name must be lowercase kebab-case (a-z, 0-9, hyphen) and start with a letter or digit",
                spec.name
            ));
        }

        if !seen_names.insert(spec.name.clone()) {
            return Err(anyhow!("duplicate agent name '{}'", spec.name));
        }

        let prompt = match (&spec.prompt, &spec.prompt_file) {
            (Some(prompt), None) => prompt.clone(),
            (None, Some(file)) => {
                let resolved = if Path::new(file).is_absolute() {
                    PathBuf::from(file)
                } else {
                    base_dir.join(file)
                };
                fs::read_to_string(&resolved).map_err(|e| {
                    anyhow!(
                        "agent '{}': failed to read prompt_file '{}': {}",
                        spec.name,
                        resolved.display(),
                        e
                    )
                })?
            }
            _ => {
                return Err(anyhow!(
                    "agent '{}': set exactly one of `prompt` or `prompt_file`",
                    spec.name
                ))
            }
        };

        if prompt.trim().is_empty() {
            return Err(anyhow!("agent '{}': prompt is empty", spec.name));
        }

        if spec.timeout_secs == 0 {
            return Err(anyhow!(
                "agent '{}': timeout_secs must be greater than 0",
                spec.name
            ));
        }

        let schedule_normalized = normalize_cron(&spec.schedule).map_err(|e| {
            anyhow!(
                "agent '{}': invalid schedule '{}': {}",
                spec.name,
                spec.schedule,
                e
            )
        })?;
        let schedule = cron::Schedule::from_str(&schedule_normalized).map_err(|e| {
            anyhow!(
                "agent '{}': invalid schedule '{}': {}",
                spec.name,
                spec.schedule,
                e
            )
        })?;

        if let Some(model) = &spec.model {
            model_validator(model).map_err(|e| {
                anyhow!("agent '{}': unknown model '{}': {}", spec.name, model, e)
            })?;
        }

        let working_dir = match &spec.working_dir {
            Some(dir) => {
                let candidate = Path::new(dir);
                if !candidate.exists() {
                    return Err(anyhow!(
                        "agent '{}': working_dir '{}' does not exist",
                        spec.name,
                        dir
                    ));
                }
                if !candidate.is_dir() {
                    return Err(anyhow!(
                        "agent '{}': working_dir '{}' is not a directory",
                        spec.name,
                        dir
                    ));
                }
                Some(fs::canonicalize(candidate).map_err(|e| {
                    anyhow!(
                        "agent '{}': working_dir '{}' could not be resolved: {}",
                        spec.name,
                        dir,
                        e
                    )
                })?)
            }
            None => None,
        };

        agents.push(LoadedAgent {
            name: spec.name,
            schedule_expr: spec.schedule,
            schedule_normalized,
            schedule,
            model: spec.model,
            prompt,
            enabled: spec.enabled,
            timeout_secs: spec.timeout_secs,
            working_dir,
        });
    }

    Ok(AgentsConfig {
        source_path,
        agents,
    })
}

/// Resolve a model on a dedicated thread.
///
/// For models it does not recognise, `detect_provider` probes Ollama with a
/// blocking reqwest client. Dropping that client's runtime inside an async
/// context aborts the process, and both callers here run inside one, so the
/// probe is given a thread with no ambient runtime.
pub fn detect_provider_isolated(model: &str) -> Result<crate::models::ProviderInfo> {
    let model = model.to_string();
    std::thread::spawn(move || crate::provider::detect_provider(&model))
        .join()
        .map_err(|_| anyhow!("model detection panicked"))?
}

/// Convenience wrapper used by main.rs / the daemon installer: validates models
/// via `crate::provider::detect_provider`.
pub fn load_agents(path: &Path) -> Result<AgentsConfig> {
    load_agents_file(path, &|model| detect_provider_isolated(model).map(|_| ()))
}

/// First `max_chars` of the prompt, with trailing whitespace trimmed and an
/// ellipsis appended when truncated. Used for the read-only UI preview.
pub fn prompt_preview(prompt: &str, max_chars: usize) -> String {
    let trimmed = prompt.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    let head: String = trimmed.chars().take(max_chars).collect();
    format!("{}…", head.trim_end())
}

fn is_valid_agent_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }

    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Utc, Weekday};
    use tempfile::TempDir;

    fn allow_all_models(_model: &str) -> Result<()> {
        Ok(())
    }

    fn reject_all_models(model: &str) -> Result<()> {
        Err(anyhow!("no provider matches '{}'", model))
    }

    fn write_config(dir: &TempDir, body: &str) -> PathBuf {
        let path = dir.path().join("agents.toml");
        fs::write(&path, body).unwrap();
        path
    }

    fn next_weekdays(normalized: &str, count: usize) -> Vec<Weekday> {
        let schedule = cron::Schedule::from_str(normalized).unwrap();
        schedule
            .upcoming(Utc)
            .take(count)
            .map(|dt| dt.weekday())
            .collect()
    }

    #[test]
    fn test_normalize_cron_daily() {
        assert_eq!(normalize_cron("0 9 * * *").unwrap(), "0 0 9 * * *");
    }

    #[test]
    fn test_normalize_cron_step_minutes() {
        assert_eq!(normalize_cron("*/30 * * * *").unwrap(), "0 */30 * * * *");
    }

    #[test]
    fn test_normalize_cron_monday_fires_on_monday() {
        let normalized = normalize_cron("0 9 * * 1").unwrap();
        assert_eq!(normalized, "0 0 9 * * 2");
        assert_eq!(next_weekdays(&normalized, 1), vec![Weekday::Mon]);
    }

    #[test]
    fn test_normalize_cron_weekday_range() {
        let normalized = normalize_cron("0 9 * * 1-5").unwrap();
        assert_eq!(normalized, "0 0 9 * * 2-6");
        for weekday in next_weekdays(&normalized, 5) {
            assert!(
                matches!(
                    weekday,
                    Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri
                ),
                "unexpected weekday {:?}",
                weekday
            );
        }
    }

    #[test]
    fn test_normalize_cron_sunday_zero_fires_on_sunday() {
        let normalized = normalize_cron("0 9 * * 0").unwrap();
        assert_eq!(normalized, "0 0 9 * * 1");
        assert_eq!(next_weekdays(&normalized, 1), vec![Weekday::Sun]);
    }

    #[test]
    fn test_normalize_cron_sunday_seven() {
        assert_eq!(normalize_cron("0 9 * * 7").unwrap(), "0 0 9 * * 1");
    }

    #[test]
    fn test_normalize_cron_names_pass_through() {
        assert_eq!(
            normalize_cron("0 9 * * MON-FRI").unwrap(),
            "0 0 9 * * MON-FRI"
        );
    }

    #[test]
    fn test_normalize_cron_list_and_step() {
        assert_eq!(normalize_cron("0 9 * * 1,3,5").unwrap(), "0 0 9 * * 2,4,6");
        assert_eq!(normalize_cron("0 9 * * 1-5/2").unwrap(), "0 0 9 * * 2-6/2");
    }

    // Both day fields restricted means "and" here but "or" in Unix cron, so the
    // scheduler warns. These pin which expressions trip that warning.
    #[test]
    fn test_restricts_both_day_fields() {
        assert!(restricts_both_day_fields("0 9 1 * 1"));
        assert!(restricts_both_day_fields("0 9 1,15 * MON"));

        assert!(!restricts_both_day_fields("0 9 * * 1"));
        assert!(!restricts_both_day_fields("0 9 1 * *"));
        assert!(!restricts_both_day_fields("0 9 * * *"));
        assert!(!restricts_both_day_fields("0 9 ? * 1"));
        assert!(!restricts_both_day_fields("*/30 * * * *"));
        assert!(!restricts_both_day_fields("garbage"));
    }

    #[test]
    fn test_normalize_cron_rejects_wrong_field_counts() {
        let err = normalize_cron("0 9 * *").unwrap_err().to_string();
        assert!(err.contains("got 4 fields"), "{}", err);

        let err = normalize_cron("0 9 * * * *").unwrap_err().to_string();
        assert!(err.contains("got 6 fields"), "{}", err);

        let err = normalize_cron("nonsense").unwrap_err().to_string();
        assert!(err.contains("got 1 fields"), "{}", err);
    }

    #[test]
    fn test_normalize_cron_rejects_out_of_range_dow() {
        let err = normalize_cron("0 9 * * 9").unwrap_err().to_string();
        assert!(err.contains("out of range"), "{}", err);
    }

    #[test]
    fn test_normalize_cron_rejects_bad_minute() {
        assert!(normalize_cron("bogus 9 * * *").is_err());
    }

    #[test]
    fn test_load_multiple_agents() {
        let dir = TempDir::new().unwrap();
        let path = write_config(
            &dir,
            r#"
[[agent]]
name = "daily-digest"
schedule = "0 9 * * *"
model = "sonnet"
prompt = "Summarize yesterday"

[[agent]]
name = "repo-watch2"
schedule = "*/30 * * * *"
prompt = "Watch the repo"
enabled = false
timeout_secs = 120
"#,
        );

        let config = load_agents_file(&path, &allow_all_models).unwrap();
        assert_eq!(config.agents.len(), 2);
        assert!(config.source_path.is_absolute());

        let first = &config.agents[0];
        assert_eq!(first.name, "daily-digest");
        assert_eq!(first.schedule_expr, "0 9 * * *");
        assert_eq!(first.schedule_normalized, "0 0 9 * * *");
        assert_eq!(first.model.as_deref(), Some("sonnet"));
        assert!(first.enabled);
        assert_eq!(first.timeout_secs, 600);
        assert!(first.working_dir.is_none());

        let second = &config.agents[1];
        assert!(!second.enabled);
        assert_eq!(second.timeout_secs, 120);
        assert!(second.model.is_none());
    }

    #[test]
    fn test_load_empty_agent_list_is_valid() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "# no agents here\n");
        let config = load_agents_file(&path, &allow_all_models).unwrap();
        assert!(config.agents.is_empty());
    }

    #[test]
    fn test_load_missing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nope.toml");
        let err = load_agents_file(&path, &allow_all_models)
            .unwrap_err()
            .to_string();
        assert!(err.contains("failed to read agents file"), "{}", err);
    }

    #[test]
    fn test_load_invalid_toml() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "[[agent]\nname = \"x\"\n");
        let err = load_agents_file(&path, &allow_all_models)
            .unwrap_err()
            .to_string();
        assert!(err.contains("failed to parse"), "{}", err);
    }

    #[test]
    fn test_load_rejects_unknown_field() {
        let dir = TempDir::new().unwrap();
        let path = write_config(
            &dir,
            r#"
[[agent]]
name = "a"
schedule = "0 9 * * *"
prompt = "hi"
bogus = true
"#,
        );
        let err = load_agents_file(&path, &allow_all_models)
            .unwrap_err()
            .to_string();
        assert!(err.contains("failed to parse"), "{}", err);
    }

    #[test]
    fn test_load_rejects_invalid_name() {
        let dir = TempDir::new().unwrap();
        for name in ["Daily", "-lead", "has space", "under_score", ""] {
            let path = write_config(
                &dir,
                &format!(
                    "[[agent]]\nname = \"{}\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n",
                    name
                ),
            );
            let err = load_agents_file(&path, &allow_all_models)
                .unwrap_err()
                .to_string();
            assert!(err.contains("kebab-case"), "name {:?}: {}", name, err);
        }
    }

    #[test]
    fn test_load_rejects_long_name() {
        let dir = TempDir::new().unwrap();
        let path = write_config(
            &dir,
            &format!(
                "[[agent]]\nname = \"{}\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n",
                "a".repeat(65)
            ),
        );
        let err = load_agents_file(&path, &allow_all_models)
            .unwrap_err()
            .to_string();
        assert!(err.contains("kebab-case"), "{}", err);
    }

    #[test]
    fn test_load_rejects_duplicate_name() {
        let dir = TempDir::new().unwrap();
        let path = write_config(
            &dir,
            r#"
[[agent]]
name = "dup"
schedule = "0 9 * * *"
prompt = "one"

[[agent]]
name = "dup"
schedule = "0 10 * * *"
prompt = "two"
"#,
        );
        let err = load_agents_file(&path, &allow_all_models)
            .unwrap_err()
            .to_string();
        assert!(err.contains("duplicate agent name 'dup'"), "{}", err);
    }

    #[test]
    fn test_load_rejects_both_prompt_sources() {
        let dir = TempDir::new().unwrap();
        let path = write_config(
            &dir,
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\nprompt_file = \"p.md\"\n",
        );
        let err = load_agents_file(&path, &allow_all_models)
            .unwrap_err()
            .to_string();
        assert!(err.contains("exactly one of"), "{}", err);
    }

    #[test]
    fn test_load_rejects_neither_prompt_source() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\n");
        let err = load_agents_file(&path, &allow_all_models)
            .unwrap_err()
            .to_string();
        assert!(err.contains("exactly one of"), "{}", err);
    }

    #[test]
    fn test_prompt_file_resolves_relative_to_config_dir() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("prompts")).unwrap();
        fs::write(
            dir.path().join("prompts/repo-watch.md"),
            "Check the repo for changes",
        )
        .unwrap();

        let path = write_config(
            &dir,
            "[[agent]]\nname = \"repo-watch\"\nschedule = \"*/30 * * * *\"\nprompt_file = \"prompts/repo-watch.md\"\n",
        );

        let config = load_agents_file(&path, &allow_all_models).unwrap();
        assert_eq!(config.agents[0].prompt, "Check the repo for changes");
    }

    #[test]
    fn test_load_rejects_missing_prompt_file() {
        let dir = TempDir::new().unwrap();
        let path = write_config(
            &dir,
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt_file = \"missing.md\"\n",
        );
        let err = load_agents_file(&path, &allow_all_models)
            .unwrap_err()
            .to_string();
        assert!(err.contains("failed to read prompt_file"), "{}", err);
        assert!(err.contains("missing.md"), "{}", err);
    }

    #[test]
    fn test_load_rejects_empty_prompt() {
        let dir = TempDir::new().unwrap();
        let path = write_config(
            &dir,
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"   \\n\"\n",
        );
        let err = load_agents_file(&path, &allow_all_models)
            .unwrap_err()
            .to_string();
        assert!(err.contains("prompt is empty"), "{}", err);
    }

    #[test]
    fn test_load_rejects_zero_timeout() {
        let dir = TempDir::new().unwrap();
        let path = write_config(
            &dir,
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\ntimeout_secs = 0\n",
        );
        let err = load_agents_file(&path, &allow_all_models)
            .unwrap_err()
            .to_string();
        assert!(err.contains("timeout_secs must be greater than 0"), "{}", err);
    }

    #[test]
    fn test_load_rejects_invalid_schedule() {
        let dir = TempDir::new().unwrap();
        let path = write_config(
            &dir,
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * *\"\nprompt = \"hi\"\n",
        );
        let err = load_agents_file(&path, &allow_all_models)
            .unwrap_err()
            .to_string();
        assert!(err.contains("agent 'a': invalid schedule '0 9 * *'"), "{}", err);
    }

    #[test]
    fn test_load_rejects_unknown_model() {
        let dir = TempDir::new().unwrap();
        let path = write_config(
            &dir,
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\nmodel = \"bogus\"\n",
        );
        let err = load_agents_file(&path, &reject_all_models)
            .unwrap_err()
            .to_string();
        assert!(err.contains("unknown model 'bogus'"), "{}", err);
    }

    #[test]
    fn test_model_validator_not_called_without_model() {
        let dir = TempDir::new().unwrap();
        let path = write_config(
            &dir,
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n",
        );
        assert!(load_agents_file(&path, &reject_all_models).is_ok());
    }

    #[test]
    fn test_working_dir_is_canonicalized() {
        let dir = TempDir::new().unwrap();
        let work = dir.path().join("work");
        fs::create_dir(&work).unwrap();

        let path = write_config(
            &dir,
            &format!(
                "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\nworking_dir = \"{}\"\n",
                work.display()
            ),
        );

        let config = load_agents_file(&path, &allow_all_models).unwrap();
        assert_eq!(
            config.agents[0].working_dir.as_ref().unwrap(),
            &fs::canonicalize(&work).unwrap()
        );
    }

    #[test]
    fn test_load_rejects_missing_working_dir() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("nowhere");
        let path = write_config(
            &dir,
            &format!(
                "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\nworking_dir = \"{}\"\n",
                missing.display()
            ),
        );
        let err = load_agents_file(&path, &allow_all_models)
            .unwrap_err()
            .to_string();
        assert!(err.contains("does not exist"), "{}", err);
    }

    #[test]
    fn test_load_rejects_working_dir_that_is_a_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("notadir");
        fs::write(&file, "x").unwrap();

        let path = write_config(
            &dir,
            &format!(
                "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\nworking_dir = \"{}\"\n",
                file.display()
            ),
        );
        let err = load_agents_file(&path, &allow_all_models)
            .unwrap_err()
            .to_string();
        assert!(err.contains("is not a directory"), "{}", err);
    }

    #[test]
    fn test_prompt_preview_short_prompt_unchanged() {
        assert_eq!(prompt_preview("  hello there \n", 40), "hello there");
    }

    #[test]
    fn test_prompt_preview_truncates_multibyte_without_panicking() {
        let prompt = "héllo wörld — summarize everything 日本語です";
        let preview = prompt_preview(prompt, 7);
        assert_eq!(preview, "héllo w…");

        for len in 0..prompt.chars().count() + 2 {
            let _ = prompt_preview(prompt, len);
        }
    }

    #[test]
    fn test_prompt_preview_trims_trailing_space_before_ellipsis() {
        assert_eq!(prompt_preview("summarize the repo now", 10), "summarize…");
    }
}
