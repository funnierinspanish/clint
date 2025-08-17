use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::models::{CLIKeywords, CLISummary};

pub fn generate_summary(path: &PathBuf) -> Result<CLISummary, Box<dyn std::error::Error>> {
    let data = match extract_data(path) {
        Some(data) => data,
        None => {
            return Err("Failed to extract data from JSON".into());
        }
    };

    let summary = CLIKeywords {
        base_program: data.base_program,
        commands: data.commands,
        subcommands: data.subcommands,
        short_flags: data.short_flags,
        long_flags: data.long_flags,
    };

    let total_command_count = summary.commands.len();
    let total_subcommand_count = summary.subcommands.len();
    let total_short_flag_count = summary.short_flags.len();
    let total_long_flag_count = summary.long_flags.len();
    let unique_command_count = summary.commands.iter().collect::<HashSet<_>>().len();
    let unique_subcommand_count = summary.subcommands.iter().collect::<HashSet<_>>().len();
    let unique_short_flag_count = summary.short_flags.iter().collect::<HashSet<_>>().len();
    let unique_long_flag_count = summary.long_flags.iter().collect::<HashSet<_>>().len();
    let unique_keywords_count = unique_command_count
        + unique_subcommand_count
        + unique_short_flag_count
        + unique_long_flag_count;

    Ok(CLISummary {
        unique_keywords_count,
        unique_command_count,
        unique_subcommand_count,
        unique_short_flag_count,
        unique_long_flag_count,
        total_command_count,
        total_subcommand_count,
        total_short_flag_count,
        total_long_flag_count,
    })
}

fn extract_data(path: &PathBuf) -> Option<CLIKeywords> {
    let raw = fs::read_to_string(path).expect("Failed to read file");
    let json: Value = serde_json::from_str(&raw).expect("Failed to read file as JSON");
    let base_program = json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let mut commands = Vec::new();
    let mut subcommands = vec![];
    let mut short_flags = vec![];
    let mut long_flags = vec![];

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

    Some(CLIKeywords {
        base_program,
        commands,
        subcommands,
        short_flags,
        long_flags,
    })
}

fn walk_commands_recursively(
    _parent_command: &str,
    node: &Value,
    subcommands: &mut Vec<String>,
    short_flags: &mut Vec<String>,
    long_flags: &mut Vec<String>,
) {
    if let Some(command_map) = node.get("COMMAND").and_then(|v| v.as_object()) {
        for (subcmd_name, subcmd_obj) in command_map {
            // Only mark as subcommand if parent is a command (not root)
            subcommands.push(subcmd_name.clone());

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
                short_flags.push(s.to_string());
            }
            if let Some(l) = flag.get("long").and_then(|v| v.as_str()) {
                long_flags.push(l.to_string());
            }
        }
    }
}
