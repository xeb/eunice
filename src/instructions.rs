use anyhow::{anyhow, Result};
use std::path::Path;

pub const AGENTS_FILE: &str = "AGENTS.md";

/// Load project instructions from `AGENTS.md` in exactly the supplied directory.
/// Parent directories are intentionally not searched.
pub fn load_agents_md(dir: &Path) -> Result<Option<String>> {
    let path = dir.join(AGENTS_FILE);
    let metadata = match std::fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(anyhow!(
                "Failed to inspect system instructions '{}': {}",
                path.display(),
                error
            ))
        }
    };

    if !metadata.is_file() {
        return Ok(None);
    }

    std::fs::read_to_string(&path).map(Some).map_err(|error| {
        anyhow!(
            "Failed to read system instructions '{}': {}",
            path.display(),
            error
        )
    })
}

/// Combine project instructions with separately configured system instructions.
/// Project instructions come first so task-specific configuration can follow them.
pub fn combine_system_instructions(
    project: Option<&str>,
    configured: Option<&str>,
) -> Option<String> {
    let project = project.filter(|value| !value.trim().is_empty());
    let configured = configured.filter(|value| !value.trim().is_empty());

    match (project, configured) {
        (Some(project), Some(configured)) => Some(format!("{}\n\n---\n\n{}", project, configured)),
        (Some(project), None) => Some(project.to_string()),
        (None, Some(configured)) => Some(configured.to_string()),
        (None, None) => None,
    }
}

/// Eunice's shared message model has no system role, so system instructions ride
/// in the first user message while the visible prompt remains separate in the UI.
pub fn compose_first_user_message(
    system_instructions: Option<&str>,
    first_turn: bool,
    prompt: &str,
) -> String {
    match system_instructions.filter(|value| !value.trim().is_empty()) {
        Some(instructions) if first_turn => format!("{}\n\n---\n\n{}", instructions, prompt),
        _ => prompt.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_agents_md_from_only_the_supplied_directory() {
        let parent = tempfile::tempdir().unwrap();
        std::fs::write(parent.path().join(AGENTS_FILE), "parent rules").unwrap();
        let child = parent.path().join("child");
        std::fs::create_dir(&child).unwrap();

        assert_eq!(
            load_agents_md(parent.path()).unwrap().as_deref(),
            Some("parent rules")
        );
        assert_eq!(load_agents_md(&child).unwrap(), None);
    }

    #[test]
    fn ignores_a_non_file_named_agents_md() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(AGENTS_FILE)).unwrap();

        assert_eq!(load_agents_md(dir.path()).unwrap(), None);
    }

    #[test]
    fn combines_project_and_configured_instructions_in_order() {
        assert_eq!(
            combine_system_instructions(Some("project"), Some("configured")).as_deref(),
            Some("project\n\n---\n\nconfigured")
        );
        assert_eq!(
            combine_system_instructions(Some("project"), None).as_deref(),
            Some("project")
        );
        assert_eq!(combine_system_instructions(Some("  "), None), None);
    }

    #[test]
    fn composes_instructions_on_the_first_turn_only() {
        assert_eq!(
            compose_first_user_message(Some("rules"), true, "hello"),
            "rules\n\n---\n\nhello"
        );
        assert_eq!(
            compose_first_user_message(Some("rules"), false, "hello"),
            "hello"
        );
        assert_eq!(compose_first_user_message(None, true, "hello"), "hello");
    }
}
