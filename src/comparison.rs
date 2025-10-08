use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub enum ChangeType {
    CommandAdded {
        parent: String,
        command: String,
    },
    CommandRemoved {
        parent: String,
        command: String,
    },
    FlagAdded {
        command: String,
        flag: String,
    },
    FlagRemoved {
        command: String,
        flag: String,
    },
    FlagDescriptionChanged {
        command: String,
        flag: String,
        old_desc: String,
        new_desc: String,
    },
    FlagDataTypeChanged {
        command: String,
        flag: String,
        old_type: Option<String>,
        new_type: Option<String>,
    },
}

impl ChangeType {
    pub fn format(&self) -> String {
        match self {
            ChangeType::CommandAdded { parent, command } => {
                if parent.is_empty() {
                    format!("+ Added command: {}", command)
                } else {
                    format!("+ Added command: {} (to {})", command, parent)
                }
            }
            ChangeType::CommandRemoved { parent, command } => {
                if parent.is_empty() {
                    format!("- Removed command: {}", command)
                } else {
                    format!("- Removed command: {} (from {})", command, parent)
                }
            }
            ChangeType::FlagAdded { command, flag } => {
                format!("+ Added flag: {} (command: {})", flag, command)
            }
            ChangeType::FlagRemoved { command, flag } => {
                format!("- Removed flag: {} (command: {})", flag, command)
            }
            ChangeType::FlagDescriptionChanged {
                command,
                flag,
                old_desc,
                new_desc,
            } => {
                format!(
                    "~ Modified flag: {} (command: {})\n    Description changed:\n      Before: \"{}\"\n      After:  \"{}\"",
                    flag, command, old_desc, new_desc
                )
            }
            ChangeType::FlagDataTypeChanged {
                command,
                flag,
                old_type,
                new_type,
            } => {
                let old_str = old_type.as_deref().unwrap_or("none");
                let new_str = new_type.as_deref().unwrap_or("none");
                format!(
                    "~ Modified flag: {} (command: {})\n    Data type changed: {} -> {}",
                    flag, command, old_str, new_str
                )
            }
        }
    }
}

pub fn compare_json_structures(
    from_path: &Path,
    to_path: &Path,
) -> Result<Vec<ChangeType>, Box<dyn std::error::Error>> {
    let from_content = fs::read_to_string(from_path)?;
    let to_content = fs::read_to_string(to_path)?;

    let from_json: Value = serde_json::from_str(&from_content)?;
    let to_json: Value = serde_json::from_str(&to_content)?;

    let mut changes = Vec::new();
    compare_commands_json(&from_json, &to_json, "", &mut changes);

    Ok(changes)
}

fn compare_commands_json(
    from: &Value,
    to: &Value,
    parent_path: &str,
    changes: &mut Vec<ChangeType>,
) {
    // Get command maps from both structures
    let from_commands = from
        .get("children")
        .and_then(|c| c.get("COMMAND"))
        .and_then(|c| c.as_object());

    let to_commands = to
        .get("children")
        .and_then(|c| c.get("COMMAND"))
        .and_then(|c| c.as_object());

    let from_set: HashSet<String> = from_commands
        .map(|cmds| cmds.keys().cloned().collect())
        .unwrap_or_default();

    let to_set: HashSet<String> = to_commands
        .map(|cmds| cmds.keys().cloned().collect())
        .unwrap_or_default();

    // Find added commands
    for command in to_set.difference(&from_set) {
        changes.push(ChangeType::CommandAdded {
            parent: parent_path.to_string(),
            command: command.clone(),
        });
    }

    // Find removed commands
    for command in from_set.difference(&to_set) {
        changes.push(ChangeType::CommandRemoved {
            parent: parent_path.to_string(),
            command: command.clone(),
        });
    }

    // Compare existing commands
    if let (Some(from_cmds), Some(to_cmds)) = (from_commands, to_commands) {
        for command_name in from_set.intersection(&to_set) {
            if let (Some(from_cmd), Some(to_cmd)) =
                (from_cmds.get(command_name), to_cmds.get(command_name))
            {
                let current_path = if parent_path.is_empty() {
                    command_name.clone()
                } else {
                    format!("{} {}", parent_path, command_name)
                };

                // Compare flags for this command
                compare_flags_json(from_cmd, to_cmd, &current_path, changes);

                // Recursively compare subcommands
                compare_commands_json(from_cmd, to_cmd, &current_path, changes);
            }
        }
    }
}

fn compare_flags_json(from: &Value, to: &Value, command_path: &str, changes: &mut Vec<ChangeType>) {
    let from_flags = extract_flags_from_json(from);
    let to_flags = extract_flags_from_json(to);

    // Create maps for easier comparison using flag signature as key
    let from_flag_map: HashMap<String, &Value> = from_flags
        .iter()
        .map(|f| (get_flag_signature(f), *f))
        .collect();

    let to_flag_map: HashMap<String, &Value> = to_flags
        .iter()
        .map(|f| (get_flag_signature(f), *f))
        .collect();

    // Find added flags
    for (key, flag) in &to_flag_map {
        if !from_flag_map.contains_key(key) {
            changes.push(ChangeType::FlagAdded {
                command: command_path.to_string(),
                flag: format_flag_display(flag),
            });
        }
    }

    // Find removed flags
    for (key, flag) in &from_flag_map {
        if !to_flag_map.contains_key(key) {
            changes.push(ChangeType::FlagRemoved {
                command: command_path.to_string(),
                flag: format_flag_display(flag),
            });
        }
    }

    // Compare existing flags
    for (key, from_flag) in &from_flag_map {
        if let Some(to_flag) = to_flag_map.get(key) {
            let flag_name = format_flag_display(from_flag);

            // Compare descriptions
            let from_desc = from_flag
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let to_desc = to_flag
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if from_desc != to_desc {
                changes.push(ChangeType::FlagDescriptionChanged {
                    command: command_path.to_string(),
                    flag: flag_name.clone(),
                    old_desc: from_desc.to_string(),
                    new_desc: to_desc.to_string(),
                });
            }

            // Compare data types
            let from_type = from_flag
                .get("data_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let to_type = to_flag
                .get("data_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            if from_type != to_type {
                changes.push(ChangeType::FlagDataTypeChanged {
                    command: command_path.to_string(),
                    flag: flag_name,
                    old_type: from_type,
                    new_type: to_type,
                });
            }
        }
    }
}

fn extract_flags_from_json(structure: &Value) -> Vec<&Value> {
    structure
        .get("children")
        .and_then(|c| c.get("FLAG"))
        .and_then(|f| f.as_array())
        .map(|flags| flags.iter().collect())
        .unwrap_or_default()
}

fn get_flag_signature(flag: &Value) -> String {
    let long = flag.get("long").and_then(|v| v.as_str()).unwrap_or("");
    let short = flag.get("short").and_then(|v| v.as_str()).unwrap_or("");

    // Use long flag as primary key, fall back to short if no long flag
    if !long.is_empty() {
        long.to_string()
    } else {
        short.to_string()
    }
}

fn format_flag_display(flag: &Value) -> String {
    let long = flag.get("long").and_then(|v| v.as_str()).unwrap_or("");
    let short = flag.get("short").and_then(|v| v.as_str()).unwrap_or("");

    match (short.is_empty(), long.is_empty()) {
        (false, false) => format!("{}/{}", short, long),
        (false, true) => short.to_string(),
        (true, false) => long.to_string(),
        (true, true) => "unknown".to_string(),
    }
}

pub fn compare_typescript_directories(
    from_dir: &Path,
    to_dir: &Path,
) -> Result<Vec<ChangeType>, Box<dyn std::error::Error>> {
    let mut changes = Vec::new();

    // Get all TypeScript files in both directories
    let from_files = get_ts_files(from_dir)?;
    let to_files = get_ts_files(to_dir)?;

    let from_set: HashSet<_> = from_files.iter().collect();
    let to_set: HashSet<_> = to_files.iter().collect();

    // Find added files (new commands)
    for file in to_set.difference(&from_set) {
        if let Some(command_info) = extract_command_from_path(file) {
            changes.push(ChangeType::CommandAdded {
                parent: command_info.parent,
                command: command_info.command,
            });
        }
    }

    // Find removed files (removed commands)
    for file in from_set.difference(&to_set) {
        if let Some(command_info) = extract_command_from_path(file) {
            changes.push(ChangeType::CommandRemoved {
                parent: command_info.parent,
                command: command_info.command,
            });
        }
    }

    // Find modified files and analyze their content
    for file in from_set.intersection(&to_set) {
        let from_path = from_dir.join(file);
        let to_path = to_dir.join(file);

        if let (Ok(from_content), Ok(to_content)) =
            (fs::read_to_string(&from_path), fs::read_to_string(&to_path))
            && from_content != to_content
        {
            // Analyze the TypeScript content for detailed changes
            analyze_typescript_changes(&from_content, &to_content, file, &mut changes)?;
        }
    }

    Ok(changes)
}

#[derive(Debug)]
struct CommandInfo {
    parent: String,
    command: String,
}

fn extract_command_from_path(file_path: &str) -> Option<CommandInfo> {
    let path_without_ext = file_path.replace(".ts", "");
    let path_parts: Vec<&str> = path_without_ext.split('/').collect();

    if path_parts.len() == 1 {
        // Top-level command (e.g., "help.ts")
        Some(CommandInfo {
            parent: String::new(),
            command: path_parts[0].to_string(),
        })
    } else if path_parts.len() == 2 {
        // Subcommand (e.g., "secret/create.ts")
        Some(CommandInfo {
            parent: path_parts[0].to_string(),
            command: path_parts[1].to_string(),
        })
    } else if path_parts.len() > 2 {
        // Nested subcommand (e.g., "checkpoint/model/list.ts")
        let parent = path_parts[..path_parts.len() - 1].join(" ");
        let command = path_parts[path_parts.len() - 1].to_string();
        Some(CommandInfo { parent, command })
    } else {
        None
    }
}

fn analyze_typescript_changes(
    from_content: &str,
    to_content: &str,
    file_path: &str,
    changes: &mut Vec<ChangeType>,
) -> Result<(), Box<dyn std::error::Error>> {
    let command_info = extract_command_from_path(file_path);
    let command_path = match &command_info {
        Some(info) => {
            if info.parent.is_empty() {
                info.command.clone()
            } else {
                format!("{} {}", info.parent, info.command)
            }
        }
        None => file_path.replace(".ts", "").replace("/", " "),
    };

    // Extract flags from both versions
    let from_flags = extract_flags_from_typescript(from_content);
    let to_flags = extract_flags_from_typescript(to_content);

    // Create maps for comparison
    let from_flag_map: HashMap<String, TypeScriptFlag> = from_flags
        .into_iter()
        .map(|f| (f.get_signature(), f))
        .collect();

    let to_flag_map: HashMap<String, TypeScriptFlag> = to_flags
        .into_iter()
        .map(|f| (f.get_signature(), f))
        .collect();

    // Find added flags
    for (signature, flag) in &to_flag_map {
        if !from_flag_map.contains_key(signature) {
            changes.push(ChangeType::FlagAdded {
                command: command_path.clone(),
                flag: flag.format_display(),
            });
        }
    }

    // Find removed flags
    for (signature, flag) in &from_flag_map {
        if !to_flag_map.contains_key(signature) {
            changes.push(ChangeType::FlagRemoved {
                command: command_path.clone(),
                flag: flag.format_display(),
            });
        }
    }

    // Compare existing flags
    for (signature, from_flag) in &from_flag_map {
        if let Some(to_flag) = to_flag_map.get(signature) {
            // Compare descriptions
            if from_flag.description != to_flag.description {
                changes.push(ChangeType::FlagDescriptionChanged {
                    command: command_path.clone(),
                    flag: from_flag.format_display(),
                    old_desc: from_flag.description.clone(),
                    new_desc: to_flag.description.clone(),
                });
            }

            // Compare data types
            if from_flag.data_type != to_flag.data_type {
                changes.push(ChangeType::FlagDataTypeChanged {
                    command: command_path.clone(),
                    flag: from_flag.format_display(),
                    old_type: Some(from_flag.data_type.clone()),
                    new_type: Some(to_flag.data_type.clone()),
                });
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct TypeScriptFlag {
    long_name: Option<String>,
    short_name: Option<String>,
    description: String,
    data_type: String,
}

impl TypeScriptFlag {
    fn get_signature(&self) -> String {
        self.long_name
            .clone()
            .or_else(|| self.short_name.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn format_display(&self) -> String {
        match (&self.short_name, &self.long_name) {
            (Some(short), Some(long)) => format!("{}/{}", short, long),
            (Some(short), None) => short.clone(),
            (None, Some(long)) => long.clone(),
            (None, None) => "unknown".to_string(),
        }
    }
}

fn extract_flags_from_typescript(content: &str) -> Vec<TypeScriptFlag> {
    // Use regex-like approach to find the complete FLAGS array
    if let Some(start_pos) = content.find("_FLAGS: CommandFlag[] = [")
        && let Some(bracket_pos) = content[start_pos..].find("= [")
    {
        let array_start = start_pos + bracket_pos + 3; // Skip "= ["

        // Find the matching closing bracket
        let mut bracket_depth = 0;
        let mut brace_depth = 0;
        let mut in_string = false;
        let mut escape_next = false;
        let mut current_quote = '\0';

        for (i, ch) in content[array_start..].char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '\'' | '"' | '`' if !in_string => {
                    in_string = true;
                    current_quote = ch;
                }
                quote if in_string && quote == current_quote => {
                    in_string = false;
                    current_quote = '\0';
                }
                '[' if !in_string => bracket_depth += 1,
                ']' if !in_string => {
                    if bracket_depth == 0 && brace_depth == 0 {
                        // Found the matching closing bracket - we're at the end of the array
                        let array_content = &content[array_start..array_start + i];
                        return parse_flag_objects(array_content);
                    }
                    bracket_depth -= 1;
                }
                '{' if !in_string => brace_depth += 1,
                '}' if !in_string => brace_depth -= 1,
                _ => {}
            }
        }
    }

    Vec::new()
}

fn parse_flag_objects(flags_content: &str) -> Vec<TypeScriptFlag> {
    let mut flags = Vec::new();

    // Split by object boundaries - look for patterns like "{\n" to "}"
    let mut current_object = String::new();
    let mut brace_count = 0;
    let mut in_object = false;

    for char in flags_content.chars() {
        match char {
            '{' => {
                brace_count += 1;
                in_object = true;
                current_object.push(char);
            }
            '}' => {
                brace_count -= 1;
                current_object.push(char);
                if in_object && brace_count == 0 {
                    if let Some(flag) = parse_single_flag(&current_object) {
                        flags.push(flag);
                    }
                    current_object.clear();
                    in_object = false;
                }
            }
            _ => {
                if in_object {
                    current_object.push(char);
                }
            }
        }
    }

    flags
}

fn parse_single_flag(object_str: &str) -> Option<TypeScriptFlag> {
    let mut long_name = None;
    let mut short_name = None;
    let mut description = String::new();
    let mut data_type = String::new();

    // Extract longName
    if let Some(long_match) = extract_property_value(object_str, "longName") {
        long_name = Some(long_match);
    }

    // Extract shortName
    if let Some(short_match) = extract_property_value(object_str, "shortName") {
        short_name = Some(short_match);
    }

    // Extract description
    if let Some(desc_match) = extract_property_value(object_str, "description") {
        description = desc_match;
    }

    // Extract valueDataType
    if let Some(type_match) = extract_property_value(object_str, "valueDataType") {
        data_type = type_match;
    }

    // Return flag if we have at least a name and description
    if (long_name.is_some() || short_name.is_some()) && !description.is_empty() {
        Some(TypeScriptFlag {
            long_name,
            short_name,
            description,
            data_type,
        })
    } else {
        None
    }
}

fn extract_property_value(object_str: &str, property: &str) -> Option<String> {
    let pattern = format!("{}:", property);
    if let Some(start) = object_str.find(&pattern) {
        let after_colon = &object_str[start + pattern.len()..];

        // Skip whitespace
        let trimmed = after_colon.trim_start();

        if trimmed.starts_with('\'') || trimmed.starts_with('"') {
            // String value
            let quote_char = trimmed.chars().next().unwrap();
            let after_quote = &trimmed[1..];
            if let Some(end_quote) = after_quote.find(quote_char) {
                return Some(after_quote[..end_quote].to_string());
            }
        } else {
            // Enum or other value
            let value_end = trimmed
                .find(',')
                .or_else(|| trimmed.find('\n'))
                .or_else(|| trimmed.find('}'))
                .unwrap_or(trimmed.len());

            let value = trimmed[..value_end].trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    None
}

fn get_ts_files(dir: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    get_ts_files_recursive(dir, dir, &mut files)?;
    Ok(files)
}

fn get_ts_files_recursive(
    dir: &Path,
    base_dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                get_ts_files_recursive(&path, base_dir, files)?;
            } else if let Some(extension) = path.extension()
                && extension == "ts"
                && let Ok(relative_path) = path.strip_prefix(base_dir)
            {
                files.push(relative_path.to_string_lossy().to_string());
            }
        }
    }

    Ok(())
}
