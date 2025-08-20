use crate::{models::*, usage_parser::parse_usage_line};
use regex::Regex;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::process::Command;

fn execute_full_command(command: &str) -> Value {
    let command_vec: Vec<&str> = command.split_whitespace().collect();
    match Command::new(command_vec[0])
        .args(&command_vec[1..])
        .output()
    {
        Ok(output) => json!({
            "stdout": String::from_utf8_lossy(&output.stdout).trim().to_string(),
            "stderr": String::from_utf8_lossy(&output.stderr).trim().to_string(),
            "status": output.status.code().unwrap_or(-1)
        }),
        Err(e) => json!({
            "stdout": "",
            "stderr": format!("Error executing command: {}", e),
            "status": -1
        }),
    }
}

fn get_program_version(program_name: &str) -> String {
    let version_output = execute_full_command(&format!("{} version", program_name));
    version_output
        .get("stdout")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string()
}

fn is_header_line(line: &str) -> bool {
    if line.starts_with(" ") {
        false
    } else if line.contains(":") {
        true
    } else {
        !line.trim().is_empty()
    }
}

fn get_flag_line(raw_flag_vec: Vec<&str>, section_header_name: &str) -> LineFlag {
    let mut short: Option<&str> = None;
    let mut long: Option<&str> = None;

    let mut flag_part: Vec<&str> = Vec::new();
    let mut description_parts: Vec<&str> = Vec::new();

    for (i, part) in raw_flag_vec.iter().enumerate() {
        if part.starts_with("-") || part.contains(",") {
            flag_part.push(*part);
        } else {
            description_parts.extend(&raw_flag_vec[i..]);
            break;
        }
    }

    let flag_definitions_string = flag_part.join(" ");
    let flag_definitions: Vec<&str> = flag_definitions_string.split(", ").collect();
    let flags_and_possible_type_vec: Vec<&str> = flag_definitions
        .iter()
        .flat_map(|s| s.split_whitespace())
        .collect();

    let actual_flags_vec: Vec<&str> = flags_and_possible_type_vec
        .iter()
        .filter(|f| f.starts_with("-"))
        .copied()
        .collect();

    let data_type_vec: Vec<&str> = flags_and_possible_type_vec
        .iter()
        .filter(|f| !f.starts_with("-") && !f.trim().is_empty())
        .copied()
        .collect();

    let data_type = data_type_vec.first().copied();

    if actual_flags_vec.len() == 1 {
        if actual_flags_vec[0].starts_with("--") {
            long = Some(actual_flags_vec[0]);
        } else {
            short = Some(actual_flags_vec[0]);
        }
    } else if actual_flags_vec.len() >= 2 {
        let mut sorted_flags = actual_flags_vec.clone();
        sorted_flags.sort_by_key(|f| f.len());

        for flag in sorted_flags {
            if flag.starts_with("--") && long.is_none() {
                long = Some(flag);
            } else if flag.starts_with("-") && !flag.starts_with("--") && short.is_none() {
                short = Some(flag);
            }
        }
    }

    let description = if !description_parts.is_empty() {
        Some(description_parts.join(" "))
    } else {
        None
    };

    LineFlag {
        short: short.map(|s| s.to_string()),
        long: long.map(|s| s.to_string()),
        data_type: data_type.map(|s| s.to_string()),
        description,
        parent_header: section_header_name.to_string(),
    }
}

fn parse_child_line(
    command: &str,
    line: &str,
    section_header_name: Option<&str>,
) -> Option<ChildLine> {
    let section_header = section_header_name.unwrap_or("None");
    let trimmed_line = line.trim();

    let flag_re = Regex::new(r"^\s*(-{1,2}\S+)").unwrap();
    if flag_re.is_match(trimmed_line) {
        let re = Regex::new(r"\s+").unwrap();
        let line_components: Vec<&str> = re.split(trimmed_line).collect();
        return Some(ChildLine {
            line_type: OutputLine::Flag(get_flag_line(line_components, section_header)),
        });
    }

    let re = Regex::new(r"\s+").unwrap();
    let line_components: Vec<&str> = re.split(trimmed_line).collect();

    if line_components.len() == 1 {
        let parse_usage_line = parse_usage_line(trimmed_line, command);
        let usage_components = parse_usage_line;
        if usage_components.is_empty() {
            return None;
        }
        return Some(ChildLine {
            line_type: OutputLine::Other(LineOther {
                line_contents: line_components[0].to_string(),
                parent_header: section_header.to_string(),
                components: Some(usage_components),
            }),
        });
    }

    if line_components.len() >= 2 {
        if section_header.to_lowercase().contains("usage") {
            let usage_components = parse_usage_line(trimmed_line, command);
            return Some(ChildLine {
                line_type: OutputLine::Usage(LineUsage {
                    usage_string: trimmed_line.to_string(),
                    parent_header: section_header.to_string(),
                    usage_components,
                }),
            });
        }

        if section_header.to_lowercase().contains("example") {
            return Some(ChildLine {
                line_type: OutputLine::Other(LineOther {
                    line_contents: line.to_string(),
                    parent_header: section_header.to_string(),
                    components: None,
                }),
            });
        }

        let command_headers = ["commands", "available commands", "subcommands"];
        if command_headers
            .iter()
            .any(|&h| section_header.to_lowercase().contains(h))
        {
            let name = line_components[0].to_string();
            let description = line_components[1..].join(" ");

            return Some(ChildLine {
                line_type: OutputLine::Command(LineCommand {
                    name,
                    description,
                    parent_header: section_header.to_string(),
                    children: vec![],
                    parent: command.to_string(),
                }),
            });
        }

        return Some(ChildLine {
            line_type: OutputLine::Other(LineOther {
                line_contents: line.to_string(),
                parent_header: section_header.to_string(),
                components: None,
            }),
        });
    }

    None
}

fn handle_child_line(
    command: &str,
    section_header_name: &str,
    line: &str,
) -> Option<(ChildLineType, String)> {
    let child_line = parse_child_line(command, line, Some(section_header_name))?;

    match child_line.line_type {
        OutputLine::Usage(usage) => Some((
            ChildLineType::Usage,
            serde_json::to_string(&usage).expect("Failed to serialize usage line"),
        )),
        OutputLine::Command(command) => Some((
            ChildLineType::Command,
            serde_json::to_string(&command).expect("Failed to serialize command line"),
        )),
        OutputLine::Flag(flag) => Some((
            ChildLineType::Flag,
            serde_json::to_string(&flag).expect("Failed to serialize flag line"),
        )),
        OutputLine::Other(other) => Some((
            ChildLineType::Other,
            serde_json::to_string(&other).expect("Failed to serialize 'other' line"),
        )),
    }
}

fn parse_help_output_dynamic(
    _base_command: &str,
    command: &str,
    output: &str,
    visited: &mut HashSet<String>,
    depth: usize,
    command_path: &str,
) -> Value {
    if depth > 5 {
        return json!({ "children": {} });
    }

    if visited.contains(command) {
        return json!({ "children": {} });
    }
    visited.insert(command.to_string());

    let lines: Vec<String> = output.lines().map(|s| s.to_string()).collect();
    let mut description: Option<String> = None;
    let mut components = json!({ "COMMAND": {}, "FLAG": [], "USAGE": [], "OTHER": [] });
    let mut previous_section_header: Option<String> = None;
    let mut current_section_header: Option<String> = None;

    if !lines.is_empty() && !lines[0].starts_with(" ") {
        description = Some(lines[0].clone());
    }

    for line in &lines {
        if line.trim().is_empty() {
            continue;
        }
        let trimmed_line = line.trim();

        if is_header_line(line) {
            current_section_header = trimmed_line
                .strip_suffix(":")
                .map(|s| s.to_string())
                .or(Some(trimmed_line.to_string()));
            if current_section_header != previous_section_header {
                previous_section_header = current_section_header.clone();
            }
            continue;
        } else if let Some(section) = &current_section_header
            && (line.starts_with("  ") || line.starts_with("\t"))
            && let Some((line_type, child_json_str)) = handle_child_line(
                command.split_whitespace().last().unwrap_or(""),
                section,
                line,
            )
        {
            let mut child_value: Value = serde_json::from_str(&child_json_str).unwrap();

            match line_type {
                ChildLineType::Command => {
                    if let Some(cmd_name) = child_value.get("name").and_then(|v| v.as_str()) {
                        let cmd_name = cmd_name.to_string();
                        let parent_command = format!("{} {}", command, cmd_name);
                        let child_command_path = format!("{} {}", command_path, cmd_name);

                        if visited.contains(&parent_command) {
                            if let Some(obj) = child_value.as_object_mut() {
                                obj.insert(
                                    "children".to_string(),
                                    json!({
                                        "COMMAND": {}, "FLAG": [], "USAGE": [], "OTHER": []
                                    }),
                                );
                                obj.insert("depth".to_string(), json!(depth + 1));
                                obj.insert("command_path".to_string(), json!(child_command_path));
                            }

                            if let Some(command_map) =
                                components.get_mut("COMMAND").and_then(Value::as_object_mut)
                            {
                                command_map.insert(cmd_name.clone(), child_value);
                            }
                            continue;
                        }

                        if let Some(obj) = child_value.as_object_mut() {
                            obj.insert(
                                "children".to_string(),
                                json!({
                                    "COMMAND": {}, "FLAG": [], "USAGE": [], "OTHER": []
                                }),
                            );
                        }

                        if let Some(command_map) =
                            components.get_mut("COMMAND").and_then(Value::as_object_mut)
                        {
                            command_map.insert(cmd_name.clone(), child_value);
                        }

                        if depth < 5 {
                            let help_output =
                                execute_full_command(&format!("{} --help", parent_command));

                            if help_output
                                .get("status")
                                .and_then(|s| s.as_i64())
                                .unwrap_or(-1)
                                == 0
                            {
                                let parsed_children = parse_help_output_dynamic(
                                    _base_command,
                                    &parent_command,
                                    help_output["stdout"].as_str().unwrap_or_default(),
                                    visited,
                                    depth + 1,
                                    &child_command_path,
                                );

                                if let Some(command_map) =
                                    components.get_mut("COMMAND").and_then(Value::as_object_mut)
                                    && let Some(cmd_obj) = command_map.get_mut(&cmd_name)
                                    && let Some(cmd_obj_map) = cmd_obj.as_object_mut()
                                {
                                    cmd_obj_map.insert(
                                        "children".to_string(),
                                        parsed_children
                                            .get("children")
                                            .cloned()
                                            .unwrap_or_default(),
                                    );
                                    cmd_obj_map.insert(
                                        "outputs".to_string(),
                                        json!({
                                            "help_page": help_output,
                                        }),
                                    );
                                    if let Some(parsed_description) =
                                        parsed_children.get("description")
                                        && !parsed_description.as_str().unwrap_or("").is_empty()
                                    {
                                        cmd_obj_map.insert(
                                            "description".to_string(),
                                            parsed_description.clone(),
                                        );
                                    }
                                    cmd_obj_map.insert("depth".to_string(), json!(depth + 1));
                                    cmd_obj_map.insert(
                                        "command_path".to_string(),
                                        json!(child_command_path),
                                    );
                                }
                            }
                        }
                    }
                }
                ChildLineType::Flag => {
                    components["FLAG"].as_array_mut().unwrap().push(child_value);
                }
                ChildLineType::Usage => {
                    components["USAGE"]
                        .as_array_mut()
                        .unwrap()
                        .push(child_value);
                }
                ChildLineType::Other => {
                    components["OTHER"]
                        .as_array_mut()
                        .unwrap()
                        .push(child_value);
                }
            }
        }
    }

    json!({
        "description": description.unwrap_or_default(),
        "children": components
    })
}

pub fn extract_cli_structure(base_command: &str, command_name: Option<String>) -> Value {
    let current_command_name = match command_name {
        Some(name) => format!("{} {}", base_command, name),
        None => base_command.to_string(),
    };

    let mut structure = json!({
        "name":  current_command_name,
        "description": "",
        "children": {},
        "outputs": {},
        "version": get_program_version(base_command),
        "depth": 0,
        "command_path": current_command_name
    });

    let help_output = execute_full_command(&format!("{} --help", current_command_name));

    structure["outputs"] = json!({
        "help_page": help_output,
    });

    let mut visited = HashSet::new();
    let parsed = parse_help_output_dynamic(
        base_command,
        &current_command_name,
        structure["outputs"]["help_page"]["stdout"]
            .as_str()
            .unwrap_or_default(),
        &mut visited,
        0,
        &current_command_name,
    );

    structure["description"] = parsed.get("description").cloned().unwrap_or(json!(""));
    structure["children"] = parsed.get("children").cloned().unwrap_or(json!({}));

    structure
}
