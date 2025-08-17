use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::models::CLIKeywords;

pub fn extract_keywords_from_json(
    path: &PathBuf,
) -> Result<CLIKeywords, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(path).expect("Failed to read file");
    let json: Value = serde_json::from_str(&raw).expect("Failed to read file as JSON");

    let base_program = json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let mut commands = Vec::new();
    let mut subcommands = HashSet::new();
    let mut short_flags = HashSet::new();
    let mut long_flags = HashSet::new();

    if let Some(children) = json.get("children")
        && let Some(command_map) = children.get("COMMAND").and_then(|v| v.as_object())
    {
        for (cmd_name, cmd_obj) in command_map {
            commands.push(cmd_name.clone());

            // Recursively walk and collect subcommands and flags
            if let Some(grandchildren) = cmd_obj.get("children") {
                walk_commands_recursively(
                    cmd_name,
                    grandchildren,
                    &mut subcommands,
                    &mut short_flags,
                    &mut long_flags,
                );
            }
        }
    }

    Ok(CLIKeywords {
        base_program,
        commands,
        subcommands: subcommands.into_iter().collect(),
        short_flags: short_flags.into_iter().collect(),
        long_flags: long_flags.into_iter().collect(),
    })
}

fn walk_commands_recursively(
    _parent_command: &str,
    node: &Value,
    subcommands: &mut HashSet<String>,
    short_flags: &mut HashSet<String>,
    long_flags: &mut HashSet<String>,
) {
    if let Some(command_map) = node.get("COMMAND").and_then(|v| v.as_object()) {
        for (subcmd_name, subcmd_obj) in command_map {
            // Only mark as subcommand if parent is a command (not root)
            subcommands.insert(subcmd_name.clone());

            // Recurse if sub-subcommands exist
            if let Some(grandchildren) = subcmd_obj.get("children") {
                walk_commands_recursively(
                    subcmd_name,
                    grandchildren,
                    subcommands,
                    short_flags,
                    long_flags,
                );
            }
        }
    }

    if let Some(flags) = node.get("FLAG").and_then(|v| v.as_array()) {
        for flag in flags {
            if let Some(s) = flag.get("short").and_then(|v| v.as_str()) {
                short_flags.insert(s.to_string());
            }
            if let Some(l) = flag.get("long").and_then(|v| v.as_str()) {
                long_flags.insert(l.to_string());
            }
        }
    }
}
