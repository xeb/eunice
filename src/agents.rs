use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use toml_edit::{value, ArrayOfTables, DocumentMut, Item, Table, Value};

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
    /// Resolved absolute path of `prompt_file`, `None` for an inline prompt.
    pub prompt_file: Option<PathBuf>,
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

    validate_text(&content, path, model_validator)
}

/// Validate proposed file text without touching the live config. Returns the parsed
/// config so the caller can swap it in on success.
///
/// This is the one validation body; `load_agents_file` is a read plus a call to it,
/// so a proposed edit is held to exactly the rules a startup load enforces.
pub fn validate_text(
    doc_text: &str,
    source_path: &Path,
    model_validator: &dyn Fn(&str) -> Result<()>,
) -> Result<AgentsConfig> {
    let path = source_path;

    let parsed: AgentsFile = toml::from_str(doc_text)
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

        let (prompt, prompt_file) = match (&spec.prompt, &spec.prompt_file) {
            (Some(prompt), None) => (prompt.clone(), None),
            (None, Some(file)) => {
                let resolved = if Path::new(file).is_absolute() {
                    PathBuf::from(file)
                } else {
                    base_dir.join(file)
                };
                let text = fs::read_to_string(&resolved).map_err(|e| {
                    anyhow!(
                        "agent '{}': failed to read prompt_file '{}': {}",
                        spec.name,
                        resolved.display(),
                        e
                    )
                })?;
                let absolute = fs::canonicalize(&resolved).unwrap_or(resolved);
                (text, Some(absolute))
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
            prompt_file,
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

/// Content fingerprint of the config: agents.toml plus every file it references
/// via `prompt_file`. Editing a prompt file must trigger a reload just like
/// editing agents.toml, so both feed the hash.
pub fn fingerprint(config_path: &Path, agents: &[LoadedAgent]) -> String {
    let mut hasher = DefaultHasher::new();
    hash_file(&mut hasher, config_path);

    // Sorted and deduped so the hash depends on the set of referenced files, not
    // on the order agents happen to appear in.
    let mut prompt_files: Vec<&PathBuf> = agents
        .iter()
        .filter_map(|agent| agent.prompt_file.as_ref())
        .collect();
    prompt_files.sort();
    prompt_files.dedup();

    for path in prompt_files {
        path.hash(&mut hasher);
        hash_file(&mut hasher, path);
    }

    format!("{:016x}", hasher.finish())
}

/// Fold a file's bytes into the hash. An unreadable file contributes a distinct
/// marker instead of erroring, so deleting a prompt file reads as "changed".
fn hash_file(hasher: &mut DefaultHasher, path: &Path) {
    match fs::read(path) {
        Ok(bytes) => {
            1u8.hash(hasher);
            bytes.hash(hasher);
        }
        Err(_) => 0u8.hash(hasher),
    }
}

/// A single change requested against agents.toml.
#[derive(Debug, Clone)]
pub enum AgentMutation {
    /// Create when `original_name` is None, otherwise update that agent in place.
    Upsert {
        original_name: Option<String>,
        spec: AgentSpec,
    },
    Delete {
        name: String,
    },
}

/// Apply a mutation to agents.toml source text, returning the new text.
/// Pure: no filesystem access. Uses toml_edit so comments, key order, blank lines
/// and formatting outside the touched keys are preserved exactly.
pub fn apply_mutation(doc_text: &str, mutation: &AgentMutation) -> Result<String> {
    let mut doc: DocumentMut = doc_text
        .parse()
        .map_err(|e| anyhow!("failed to parse agents.toml: {}", e))?;

    match mutation {
        AgentMutation::Upsert {
            original_name: Some(original_name),
            spec,
        } => {
            let index = find_agent(&doc, original_name)
                .ok_or_else(|| anyhow!("no agent named '{}' in the config", original_name))?;
            if &spec.name != original_name {
                return Err(anyhow!(
                    "renaming an agent is not supported (run history is keyed by name)"
                ));
            }
            update_agent_table(agent_tables_mut(&mut doc)?.get_mut(index).unwrap(), spec);
        }
        AgentMutation::Upsert {
            original_name: None,
            spec,
        } => {
            if find_agent(&doc, &spec.name).is_some() {
                return Err(anyhow!("an agent named '{}' already exists", spec.name));
            }
            let separated = !doc_text.trim().is_empty();
            let mut table = Table::new();
            fill_new_agent_table(&mut table, spec);
            if separated {
                table.decor_mut().set_prefix("\n");
            }
            agent_tables_mut(&mut doc)?.push(table);
        }
        AgentMutation::Delete { name } => {
            let index = find_agent(&doc, name)
                .ok_or_else(|| anyhow!("no agent named '{}' in the config", name))?;

            // toml_edit hangs the file's leading comments off the first table, so a
            // plain remove would delete the header along with the agent.
            let carried = table_prefix(&doc, index)
                .map(|prefix| detachable_prefix(prefix).to_string())
                .unwrap_or_default();

            let mut orphaned = String::new();
            {
                let tables = agent_tables_mut(&mut doc)?;
                tables.remove(index);
                match tables.get_mut(index) {
                    Some(next) if !carried.is_empty() => {
                        let existing = next
                            .decor()
                            .prefix()
                            .and_then(|p| p.as_str())
                            .unwrap_or("")
                            .trim_start_matches('\n')
                            .to_string();
                        next.decor_mut().set_prefix(format!("{}{}", carried, existing));
                    }
                    Some(_) => {}
                    None => orphaned = carried,
                }
            }

            if !orphaned.is_empty() {
                let trailing = doc.trailing().as_str().unwrap_or("").to_string();
                doc.set_trailing(format!("{}{}", trailing, orphaned));
            }
        }
    }

    Ok(doc.to_string())
}

/// The `[[agent]]` array-of-tables, created empty when the document has none.
fn agent_tables_mut(doc: &mut DocumentMut) -> Result<&mut ArrayOfTables> {
    if doc.get("agent").is_none() {
        doc.insert("agent", Item::ArrayOfTables(ArrayOfTables::new()));
    }
    doc.get_mut("agent")
        .and_then(Item::as_array_of_tables_mut)
        .ok_or_else(|| anyhow!("`agent` in agents.toml is not an array of [[agent]] tables"))
}

fn table_prefix<'a>(doc: &'a DocumentMut, index: usize) -> Option<&'a str> {
    doc.get("agent")
        .and_then(Item::as_array_of_tables)?
        .get(index)?
        .decor()
        .prefix()
        .and_then(|prefix| prefix.as_str())
}

/// The part of a table's leading decor that belongs to the file rather than to the
/// table itself: everything up to and including the last blank line. A comment block
/// sitting directly on top of `[[agent]]` describes that agent and goes with it.
fn detachable_prefix(prefix: &str) -> &str {
    match prefix.rfind("\n\n") {
        Some(at) => &prefix[..at + 2],
        None => "",
    }
}

fn find_agent(doc: &DocumentMut, name: &str) -> Option<usize> {
    doc.get("agent")
        .and_then(Item::as_array_of_tables)?
        .iter()
        .position(|table| table.get("name").and_then(Item::as_str) == Some(name))
}

/// Overwrite the keys a spec carries, removing those it leaves unset. Keys that
/// already exist keep their position; new ones land at the end of the table.
fn update_agent_table(table: &mut Table, spec: &AgentSpec) {
    assign(table, "schedule", Value::from(spec.schedule.as_str()));
    set_or_remove(table, "model", spec.model.as_deref());

    // The prompt of a prompt_file agent lives in that file; writing a `prompt` key
    // here would break the exactly-one-of rule the loader enforces.
    if !table.contains_key("prompt_file") {
        set_or_remove(table, "prompt", spec.prompt.as_deref());
    }

    assign(table, "enabled", Value::from(spec.enabled));
    assign(table, "timeout_secs", Value::from(spec.timeout_secs as i64));
    set_or_remove(table, "working_dir", spec.working_dir.as_deref());
}

fn set_or_remove(table: &mut Table, key: &str, new_value: Option<&str>) {
    match new_value {
        Some(text) => assign(table, key, Value::from(text)),
        None => {
            table.remove(key);
        }
    }
}

/// Assign a value, carrying over the decor of the value being replaced so the
/// spacing and any trailing inline comment on that line survive the edit.
fn assign(table: &mut Table, key: &str, new_value: Value) {
    let mut new_value = new_value;
    if let Some(existing) = table.get(key).and_then(Item::as_value) {
        *new_value.decor_mut() = existing.decor().clone();
    }
    table[key] = Item::Value(new_value);
}

/// Write a fresh `[[agent]]` table, omitting optional keys left at their default.
fn fill_new_agent_table(table: &mut Table, spec: &AgentSpec) {
    table["name"] = value(spec.name.as_str());
    table["schedule"] = value(spec.schedule.as_str());
    if let Some(model) = &spec.model {
        table["model"] = value(model.as_str());
    }
    match &spec.prompt_file {
        Some(prompt_file) => table["prompt_file"] = value(prompt_file.as_str()),
        None => {
            if let Some(prompt) = &spec.prompt {
                table["prompt"] = value(prompt.as_str());
            }
        }
    }
    if !spec.enabled {
        table["enabled"] = value(false);
    }
    if spec.timeout_secs != default_timeout_secs() {
        table["timeout_secs"] = value(spec.timeout_secs as i64);
    }
    if let Some(working_dir) = &spec.working_dir {
        table["working_dir"] = value(working_dir.as_str());
    }
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

    fn spec(name: &str, schedule: &str, prompt: &str) -> AgentSpec {
        AgentSpec {
            name: name.to_string(),
            schedule: schedule.to_string(),
            model: None,
            prompt: Some(prompt.to_string()),
            prompt_file: None,
            enabled: true,
            timeout_secs: 600,
            working_dir: None,
        }
    }

    fn upsert(original_name: Option<&str>, spec: AgentSpec) -> AgentMutation {
        AgentMutation::Upsert {
            original_name: original_name.map(str::to_string),
            spec,
        }
    }

    // A hand-written file: comments above, inside and between tables, a trailing
    // inline comment, blank lines, and keys in a non-canonical order.
    const HAND_WRITTEN: &str = r#"# Scheduled agents for this box.
# Edit by hand or from the web UI.

[[agent]]
# fires before standup
timeout_secs = 30
name = "daily-digest"
prompt = "Summarize yesterday" # keep this short
schedule = "0 9 * * *"

# the noisy one
[[agent]]
name = "repo-watch"
schedule = "*/30 * * * *"
prompt = "Watch the repo"
"#;

    #[test]
    fn test_apply_mutation_round_trip_preserves_comments_and_key_order() {
        let updated = apply_mutation(
            HAND_WRITTEN,
            &upsert(
                Some("daily-digest"),
                AgentSpec {
                    timeout_secs: 30,
                    ..spec("daily-digest", "0 10 * * *", "Summarize yesterday")
                },
            ),
        )
        .unwrap();

        for comment in [
            "# Scheduled agents for this box.",
            "# Edit by hand or from the web UI.",
            "# fires before standup",
            "# keep this short",
            "# the noisy one",
        ] {
            assert!(updated.contains(comment), "lost {:?}:\n{}", comment, updated);
        }

        assert!(updated.contains(r#"schedule = "0 10 * * *""#), "{}", updated);
        assert!(!updated.contains(r#"schedule = "0 9 * * *""#), "{}", updated);

        // The unusual order (timeout_secs before name) is untouched.
        let timeout_at = updated.find("timeout_secs").unwrap();
        let name_at = updated.find(r#"name = "daily-digest""#).unwrap();
        assert!(timeout_at < name_at, "{}", updated);

        // The second table is byte-identical.
        assert!(
            updated.contains("# the noisy one\n[[agent]]\nname = \"repo-watch\"\nschedule = \"*/30 * * * *\"\nprompt = \"Watch the repo\"\n"),
            "{}",
            updated
        );

        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, &updated);
        let config = load_agents_file(&path, &allow_all_models).unwrap();
        assert_eq!(config.agents[0].schedule_expr, "0 10 * * *");
        assert_eq!(config.agents[0].timeout_secs, 30);
    }

    #[test]
    fn test_apply_mutation_clearing_optional_field_removes_the_key() {
        let original = "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\nmodel = \"sonnet\"\nworking_dir = \"/tmp\"\n";

        let updated = apply_mutation(
            original,
            &upsert(Some("a"), spec("a", "0 9 * * *", "hi")),
        )
        .unwrap();

        assert!(!updated.contains("model"), "{}", updated);
        assert!(!updated.contains("working_dir"), "{}", updated);
        assert!(updated.contains(r#"prompt = "hi""#), "{}", updated);
    }

    #[test]
    fn test_apply_mutation_sets_optional_field_that_was_absent() {
        let original = "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n";

        let updated = apply_mutation(
            original,
            &upsert(
                Some("a"),
                AgentSpec {
                    model: Some("sonnet".to_string()),
                    ..spec("a", "0 9 * * *", "hi")
                },
            ),
        )
        .unwrap();

        assert!(updated.contains(r#"model = "sonnet""#), "{}", updated);
    }

    #[test]
    fn test_apply_mutation_never_adds_prompt_to_a_prompt_file_agent() {
        let original = "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt_file = \"prompts/a.md\"\n";

        let updated = apply_mutation(
            original,
            &upsert(
                Some("a"),
                spec("a", "0 10 * * *", "this text belongs in the prompt file"),
            ),
        )
        .unwrap();

        assert!(!updated.contains("prompt ="), "{}", updated);
        assert!(!updated.contains("this text belongs"), "{}", updated);
        assert!(
            updated.contains(r#"prompt_file = "prompts/a.md""#),
            "{}",
            updated
        );
        assert!(updated.contains(r#"schedule = "0 10 * * *""#), "{}", updated);
    }

    #[test]
    fn test_apply_mutation_create_appends_without_disturbing_existing() {
        let updated = apply_mutation(
            HAND_WRITTEN,
            &upsert(None, spec("new-agent", "0 6 * * 1", "Do the new thing")),
        )
        .unwrap();

        assert!(updated.starts_with(HAND_WRITTEN), "{}", updated);
        assert!(updated.contains(r#"name = "new-agent""#), "{}", updated);
        assert!(updated.contains(r#"prompt = "Do the new thing""#), "{}", updated);

        // Optional keys left at their defaults are not written out.
        let appended = &updated[HAND_WRITTEN.len()..];
        assert!(!appended.contains("enabled"), "{}", appended);
        assert!(!appended.contains("timeout_secs"), "{}", appended);
        assert!(!appended.contains("model"), "{}", appended);

        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, &updated);
        let config = load_agents_file(&path, &allow_all_models).unwrap();
        assert_eq!(config.agents.len(), 3);
        assert_eq!(config.agents[2].name, "new-agent");
        assert!(config.agents[2].enabled);
        assert_eq!(config.agents[2].timeout_secs, 600);
    }

    #[test]
    fn test_apply_mutation_create_writes_non_default_optionals_in_order() {
        let updated = apply_mutation(
            "",
            &upsert(
                None,
                AgentSpec {
                    model: Some("sonnet".to_string()),
                    enabled: false,
                    timeout_secs: 120,
                    working_dir: Some("/tmp".to_string()),
                    ..spec("fresh", "0 9 * * *", "hi")
                },
            ),
        )
        .unwrap();

        let order: Vec<&str> = [
            "name",
            "schedule",
            "model",
            "prompt",
            "enabled",
            "timeout_secs",
            "working_dir",
        ]
        .into_iter()
        .collect();

        let mut last = 0;
        for key in order {
            let at = updated
                .find(&format!("{} =", key))
                .unwrap_or_else(|| panic!("missing {} in\n{}", key, updated));
            assert!(at >= last, "{} out of order in\n{}", key, updated);
            last = at;
        }

        assert!(updated.starts_with("[[agent]]"), "{:?}", updated);
    }

    #[test]
    fn test_apply_mutation_create_uses_prompt_file_when_given() {
        let updated = apply_mutation(
            "",
            &upsert(
                None,
                AgentSpec {
                    prompt: None,
                    prompt_file: Some("prompts/a.md".to_string()),
                    ..spec("a", "0 9 * * *", "unused")
                },
            ),
        )
        .unwrap();

        assert!(updated.contains(r#"prompt_file = "prompts/a.md""#), "{}", updated);
        assert!(!updated.contains("prompt ="), "{}", updated);
    }

    #[test]
    fn test_apply_mutation_create_rejects_duplicate_name() {
        let err = apply_mutation(
            HAND_WRITTEN,
            &upsert(None, spec("repo-watch", "0 9 * * *", "hi")),
        )
        .unwrap_err()
        .to_string();
        assert_eq!(err, "an agent named 'repo-watch' already exists");
    }

    #[test]
    fn test_apply_mutation_update_rejects_missing_agent() {
        let err = apply_mutation(
            HAND_WRITTEN,
            &upsert(Some("ghost"), spec("ghost", "0 9 * * *", "hi")),
        )
        .unwrap_err()
        .to_string();
        assert_eq!(err, "no agent named 'ghost' in the config");
    }

    #[test]
    fn test_apply_mutation_rejects_rename() {
        let err = apply_mutation(
            HAND_WRITTEN,
            &upsert(Some("repo-watch"), spec("repo-watcher", "0 9 * * *", "hi")),
        )
        .unwrap_err()
        .to_string();
        assert_eq!(
            err,
            "renaming an agent is not supported (run history is keyed by name)"
        );
    }

    #[test]
    fn test_apply_mutation_delete_removes_only_the_target() {
        let updated = apply_mutation(
            HAND_WRITTEN,
            &AgentMutation::Delete {
                name: "daily-digest".to_string(),
            },
        )
        .unwrap();

        assert!(!updated.contains("daily-digest"), "{}", updated);
        assert!(updated.contains(r#"name = "repo-watch""#), "{}", updated);
        assert!(updated.contains("# the noisy one"), "{}", updated);
        assert!(updated.contains("# Scheduled agents for this box."), "{}", updated);

        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, &updated);
        let config = load_agents_file(&path, &allow_all_models).unwrap();
        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.agents[0].name, "repo-watch");
    }

    #[test]
    fn test_apply_mutation_delete_of_the_only_agent_keeps_the_file_header() {
        let original = "# Scheduled agents for this box.\n\n[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n";

        let updated = apply_mutation(
            original,
            &AgentMutation::Delete {
                name: "a".to_string(),
            },
        )
        .unwrap();

        assert!(
            updated.contains("# Scheduled agents for this box."),
            "{:?}",
            updated
        );
        assert!(!updated.contains("[[agent]]"), "{:?}", updated);

        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, &updated);
        assert!(load_agents_file(&path, &allow_all_models)
            .unwrap()
            .agents
            .is_empty());
    }


    #[test]
    fn test_detachable_prefix_keeps_the_agents_own_comment_with_it() {
        assert_eq!(detachable_prefix("# header\n\n# about this one\n"), "# header\n\n");
        assert_eq!(detachable_prefix("\n# about this one\n"), "");
        assert_eq!(detachable_prefix(""), "");
        assert_eq!(detachable_prefix("# header\n\n"), "# header\n\n");
    }

    #[test]
    fn test_apply_mutation_delete_rejects_missing_agent() {
        let err = apply_mutation(
            HAND_WRITTEN,
            &AgentMutation::Delete {
                name: "ghost".to_string(),
            },
        )
        .unwrap_err()
        .to_string();
        assert_eq!(err, "no agent named 'ghost' in the config");
    }

    #[test]
    fn test_apply_mutation_rejects_unparseable_text() {
        let err = apply_mutation("[[agent]\n", &AgentMutation::Delete { name: "a".into() })
            .unwrap_err()
            .to_string();
        assert!(err.contains("failed to parse agents.toml"), "{}", err);
    }

    #[test]
    fn test_validate_text_matches_load_agents_file() {
        let dir = TempDir::new().unwrap();
        let body = "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n";
        let path = write_config(&dir, body);

        let loaded = load_agents_file(&path, &allow_all_models).unwrap();
        let validated = validate_text(body, &path, &allow_all_models).unwrap();

        assert_eq!(validated.source_path, loaded.source_path);
        assert_eq!(validated.agents.len(), 1);
        assert_eq!(validated.agents[0].name, "a");
    }

    #[test]
    fn test_validate_text_applies_the_same_rules() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "# placeholder\n");

        let err = validate_text(
            "[[agent]]\nname = \"Bad\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n",
            &path,
            &allow_all_models,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("kebab-case"), "{}", err);

        let err = validate_text(
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n\n[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n",
            &path,
            &allow_all_models,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("duplicate agent name 'a'"), "{}", err);

        let err = validate_text(
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\nmodel = \"bogus\"\n",
            &path,
            &reject_all_models,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("unknown model 'bogus'"), "{}", err);
    }

    #[test]
    fn test_validate_text_does_not_read_the_file_on_disk() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "[[agent]]\nname = \"ondisk\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n");

        let validated = validate_text(
            "[[agent]]\nname = \"proposed\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n",
            &path,
            &allow_all_models,
        )
        .unwrap();

        assert_eq!(validated.agents[0].name, "proposed");
    }

    #[test]
    fn test_prompt_file_path_is_retained_and_absolute() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("prompts")).unwrap();
        fs::write(dir.path().join("prompts/a.md"), "Check the repo").unwrap();

        let path = write_config(
            &dir,
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt_file = \"prompts/a.md\"\n\n[[agent]]\nname = \"b\"\nschedule = \"0 9 * * *\"\nprompt = \"inline\"\n",
        );

        let config = load_agents_file(&path, &allow_all_models).unwrap();
        let prompt_file = config.agents[0].prompt_file.as_ref().unwrap();
        assert!(prompt_file.is_absolute(), "{}", prompt_file.display());
        assert_eq!(
            prompt_file,
            &fs::canonicalize(dir.path().join("prompts/a.md")).unwrap()
        );
        assert!(config.agents[1].prompt_file.is_none());
    }

    #[test]
    fn test_fingerprint_is_stable_across_repeated_calls() {
        let dir = TempDir::new().unwrap();
        let path = write_config(
            &dir,
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n",
        );
        let config = load_agents_file(&path, &allow_all_models).unwrap();

        let first = fingerprint(&path, &config.agents);
        assert_eq!(first, fingerprint(&path, &config.agents));
        assert_eq!(first, fingerprint(&path, &config.agents));
    }

    #[test]
    fn test_fingerprint_changes_when_agents_toml_changes() {
        let dir = TempDir::new().unwrap();
        let path = write_config(
            &dir,
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt = \"hi\"\n",
        );
        let config = load_agents_file(&path, &allow_all_models).unwrap();
        let before = fingerprint(&path, &config.agents);

        fs::write(
            &path,
            "[[agent]]\nname = \"a\"\nschedule = \"0 10 * * *\"\nprompt = \"hi\"\n",
        )
        .unwrap();

        assert_ne!(before, fingerprint(&path, &config.agents));
    }

    #[test]
    fn test_fingerprint_changes_when_a_prompt_file_changes_or_is_deleted() {
        let dir = TempDir::new().unwrap();
        let prompt_path = dir.path().join("a.md");
        fs::write(&prompt_path, "original prompt").unwrap();
        let path = write_config(
            &dir,
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt_file = \"a.md\"\n",
        );

        let config = load_agents_file(&path, &allow_all_models).unwrap();
        let before = fingerprint(&path, &config.agents);

        fs::write(&prompt_path, "edited prompt").unwrap();
        let after_edit = fingerprint(&path, &config.agents);
        assert_ne!(before, after_edit);

        fs::remove_file(&prompt_path).unwrap();
        let after_delete = fingerprint(&path, &config.agents);
        assert_ne!(after_edit, after_delete);
        assert_ne!(before, after_delete);
    }

    #[test]
    fn test_fingerprint_is_order_independent() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.md"), "prompt a").unwrap();
        fs::write(dir.path().join("b.md"), "prompt b").unwrap();
        let path = write_config(
            &dir,
            "[[agent]]\nname = \"a\"\nschedule = \"0 9 * * *\"\nprompt_file = \"a.md\"\n\n[[agent]]\nname = \"b\"\nschedule = \"0 9 * * *\"\nprompt_file = \"b.md\"\n",
        );

        let config = load_agents_file(&path, &allow_all_models).unwrap();
        let forward = fingerprint(&path, &config.agents);

        let mut reversed = config.agents.clone();
        reversed.reverse();
        assert_eq!(forward, fingerprint(&path, &reversed));
    }

    #[test]
    fn test_fingerprint_of_a_missing_config_does_not_panic() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("gone.toml");
        assert!(!fingerprint(&missing, &[]).is_empty());
    }
}
