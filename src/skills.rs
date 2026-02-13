use anyhow::{Context, Result};
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// A skill file with its name and content
struct SkillFile {
    filename: &'static str,
    content: &'static str,
    executable: bool,
}

/// A skill with its name and files
struct Skill {
    name: &'static str,
    files: &'static [SkillFile],
}

/// Default skills embedded in the binary - these are auto-installed on first run
const DEFAULT_SKILLS: &[Skill] = &[
    Skill {
        name: "image_analysis",
        files: &[
            SkillFile {
                filename: "SKILL.md",
                content: include_str!("../skills/image_analysis/SKILL.md"),
                executable: false,
            },
            SkillFile {
                filename: "analyze.py",
                content: include_str!("../skills/image_analysis/analyze.py"),
                executable: true,
            },
        ],
    },
    Skill {
        name: "web_search",
        files: &[
            SkillFile {
                filename: "SKILL.md",
                content: include_str!("../skills/web_search/SKILL.md"),
                executable: false,
            },
            SkillFile {
                filename: "search.py",
                content: include_str!("../skills/web_search/search.py"),
                executable: true,
            },
        ],
    },
    Skill {
        name: "git_helper",
        files: &[
            SkillFile {
                filename: "SKILL.md",
                content: include_str!("../skills/git_helper/SKILL.md"),
                executable: false,
            },
        ],
    },
    Skill {
        name: "pdf_analysis",
        files: &[
            SkillFile {
                filename: "SKILL.md",
                content: include_str!("../skills/pdf_analysis/SKILL.md"),
                executable: false,
            },
            SkillFile {
                filename: "analyze.py",
                content: include_str!("../skills/pdf_analysis/analyze.py"),
                executable: true,
            },
        ],
    },
];

/// Get the skills directory path
pub fn skills_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".eunice").join("skills"))
}

/// Ensure default skills are installed (called on first run)
pub fn ensure_default_skills() -> Result<()> {
    let dir = skills_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;

    // Only install if skills directory doesn't exist
    if dir.exists() {
        return Ok(());
    }

    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create skills directory: {}", dir.display()))?;

    for skill in DEFAULT_SKILLS {
        let skill_dir = dir.join(skill.name);
        std::fs::create_dir_all(&skill_dir)
            .with_context(|| format!("Failed to create skill directory: {}", skill_dir.display()))?;

        for file in skill.files {
            let file_path = skill_dir.join(file.filename);
            std::fs::write(&file_path, file.content)
                .with_context(|| format!("Failed to write file: {}", file_path.display()))?;

            // Make executable if needed
            if file.executable {
                let mut perms = std::fs::metadata(&file_path)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&file_path, perms)?;
            }
        }
    }

    Ok(())
}

/// Extract the ## Description section from a SKILL.md file
fn extract_description(content: &str) -> Option<String> {
    let mut in_description = false;
    let mut description_lines = Vec::new();

    for line in content.lines() {
        if line.starts_with("## Description") {
            in_description = true;
            continue;
        }

        if in_description {
            // Stop at next heading
            if line.starts_with("## ") || line.starts_with("# ") {
                break;
            }
            description_lines.push(line);
        }
    }

    if description_lines.is_empty() {
        return None;
    }

    // Trim leading/trailing empty lines and join
    let description = description_lines
        .iter()
        .map(|s| s.trim())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    if description.is_empty() {
        None
    } else {
        Some(description)
    }
}

/// List all available skills with their descriptions
pub fn list_skills() -> Result<HashMap<String, String>> {
    let dir = skills_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;

    if !dir.exists() {
        return Ok(HashMap::new());
    }

    let mut skills = HashMap::new();

    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let skill_file = path.join("SKILL.md");
        if !skill_file.exists() {
            continue;
        }

        let name = path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        if let Some(name) = name {
            if let Ok(content) = std::fs::read_to_string(&skill_file) {
                if let Some(description) = extract_description(&content) {
                    skills.insert(name, description);
                }
            }
        }
    }

    Ok(skills)
}

/// List all available skills (no query filtering)
pub async fn list_all_skills() -> Result<String> {
    let skills = list_skills()?;

    if skills.is_empty() {
        return Ok("No skills found in ~/.eunice/skills/. Create a skill by adding a directory with a SKILL.md file.".to_string());
    }

    let dir = skills_dir().unwrap();
    let mut result = format!("Available skills ({}):\n\n", skills.len());

    let mut sorted_skills: Vec<_> = skills.iter().collect();
    sorted_skills.sort_by(|a, b| a.0.cmp(b.0));

    for (name, desc) in sorted_skills {
        result.push_str(&format!("## {}\n", name));
        result.push_str(&format!("Path: {}/\n", dir.join(name).display()));
        result.push_str(&format!("Description: {}\n\n", desc));
    }

    result.push_str("Read a SKILL.md file for detailed instructions.");

    Ok(result)
}

/// Discover skills matching a query using LLM-based matching
pub async fn discover_skills(query: &str) -> Result<String> {
    let skills = list_skills()?;

    if skills.is_empty() {
        return Ok("No skills found in ~/.eunice/skills/. Create a skill by adding a directory with a SKILL.md file.".to_string());
    }

    // Build the skill list for matching
    let skill_list: Vec<String> = skills
        .iter()
        .enumerate()
        .map(|(i, (name, desc))| format!("{}. {}: {}", i + 1, name, desc))
        .collect();

    // For now, use simple keyword matching instead of LLM
    // TODO: Add LLM-based matching when we have access to the client
    let matching_skills = find_matching_skills(query, &skills);

    if matching_skills.is_empty() {
        return Ok(format!(
            "No skills match your query: \"{}\"\n\nAvailable skills:\n{}",
            query,
            skill_list.join("\n")
        ));
    }

    // Format the results
    let dir = skills_dir().unwrap();
    let mut result = format!("Found {} matching skill(s):\n\n", matching_skills.len());

    for name in &matching_skills {
        if let Some(desc) = skills.get(name) {
            result.push_str(&format!("## {}\n", name));
            result.push_str(&format!("Path: {}/\n", dir.join(name).display()));
            result.push_str(&format!("Description: {}\n\n", desc));
        }
    }

    result.push_str("Read the SKILL.md file for detailed instructions.");

    Ok(result)
}

/// Simple keyword-based skill matching (fallback when LLM not available)
fn find_matching_skills(query: &str, skills: &HashMap<String, String>) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();

    let mut matches: Vec<(String, usize)> = skills
        .iter()
        .filter_map(|(name, desc)| {
            let name_lower = name.to_lowercase();
            let desc_lower = desc.to_lowercase();
            let combined = format!("{} {}", name_lower, desc_lower);

            // Count matching words
            let score: usize = query_words
                .iter()
                .filter(|word| combined.contains(*word))
                .count();

            if score > 0 {
                Some((name.clone(), score))
            } else {
                None
            }
        })
        .collect();

    // Sort by score (descending)
    matches.sort_by(|a, b| b.1.cmp(&a.1));

    // Return top matches
    matches.into_iter().map(|(name, _)| name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_description() {
        let content = r#"# My Skill

## Description
This is a skill that does amazing things.
It can also do other stuff.

## Usage
Use it like this...
"#;
        let desc = extract_description(content).unwrap();
        assert!(desc.contains("amazing things"));
        assert!(desc.contains("other stuff"));
        assert!(!desc.contains("Usage"));
    }

    #[test]
    fn test_extract_description_missing() {
        let content = "# No Description\n## Usage\nJust use it.";
        assert!(extract_description(content).is_none());
    }

    #[test]
    fn test_find_matching_skills() {
        let mut skills = HashMap::new();
        skills.insert("image_analysis".to_string(), "Analyze images and extract text".to_string());
        skills.insert("web_search".to_string(), "Search the web for information".to_string());
        skills.insert("code_review".to_string(), "Review code for bugs".to_string());

        let matches = find_matching_skills("analyze images", &skills);
        assert!(!matches.is_empty());
        assert!(matches.contains(&"image_analysis".to_string()));

        let matches = find_matching_skills("search web", &skills);
        assert!(matches.contains(&"web_search".to_string()));
    }
}
