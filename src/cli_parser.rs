use crate::{models::*, usage_parser::parse_usage_line};
use regex::Regex;
use serde_json::{json, Value};
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
      })
  }
}

fn get_program_version(program_name: &str) -> String {
  let version_output = execute_full_command(&format!("{} version", program_name));
  version_output.get("stdout").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string()
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
    let mut data_type: Option<&str> = None;
    let mut description: Option<&str> = None;

    let mut raw_flag_vec_clone = raw_flag_vec.clone();
    let flag_definitions: Vec<&str> = raw_flag_vec_clone[0].split(", ").collect();
    raw_flag_vec_clone.splice(0..1, flag_definitions.clone());
    let flags_and_possible_type_vec: Vec<&str> =
        flag_definitions.iter().flat_map(|s| s.split(" ")).collect();
    let actual_flags_vec: Vec<&str> = flags_and_possible_type_vec
        .iter()
        .filter(|f| f.starts_with("-"))
        .copied()
        .collect();
    let data_type_vec: Vec<&str> = flags_and_possible_type_vec
        .iter()
        .filter(|f| f.starts_with("-") == false)
        .copied()
        .collect();

    if data_type_vec.len() > 0 {
        data_type = Some(data_type_vec[0]);
    } else {
        data_type = None;
    }

    if actual_flags_vec.len() == 1 {
        if actual_flags_vec[0].starts_with("--") {
            short = None;
            long = Some(actual_flags_vec[0]);
        } else {
            short = Some(actual_flags_vec[0]);
            long = None;
        }
    } else if actual_flags_vec.len() == 2 {
        short = Some(actual_flags_vec[0]);
        long = Some(actual_flags_vec[1]);
    }

    if raw_flag_vec.len() == 2 {
        description = Some(raw_flag_vec[1]);
    } else {
        description = None;
    }

    return LineFlag {
        short: short.map(|s| s.to_string()),
        long: long.map(|s| s.to_string()),
        data_type: data_type.map(|s| s.to_string()),
        description: description.map(|s| s.to_string()),
        parent_header: section_header_name.to_string()
    };
}

fn parse_child_line(command: &str, line: &str, section_header_name: Option<&str>) -> Option<ChildLine> {
    let re = Regex::new(r"\s{2,}|\t+").unwrap();
    let line_components: Vec<&str> = re.split(line.trim()).collect();
    let section_header = match section_header_name {
        Some(name) => name,
        None => "None",
    };

    // If only one component, treat as USAGE
    if line_components.len() == 1 {
        let parse_usage_line = parse_usage_line(line.trim(), command);
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

    // If 2 or more components, try to classify
    if line_components.len() >= 2 {
        let command_components: Vec<&str> = command.split_whitespace().collect();
        let first_elem: Vec<&str> = line_components[0].split_whitespace().collect();

        if command_components
            .split_last()
            .expect("Failed to get slice last elem from command")
            .1
            == line_components
                .split_last()
                .expect("Failed to get slice last elem from line")
                .1
        {
            let usage_components = parse_usage_line(line.trim(), command);
 
            return Some(ChildLine {
                line_type: OutputLine::Usage(LineUsage {
                    usage_string: line.trim().to_string(),
                    parent_header: section_header.to_string(),
                    usage_components: usage_components,
                }),
            });
        }

        if let Some(first) = first_elem.get(0) {
            let flag_re = Regex::new(r"^-{1,2}\S+").unwrap();

            if flag_re.is_match(first) {
                return Some(ChildLine {
                    line_type: OutputLine::Flag(get_flag_line(line_components, section_header)),
                });
            }

            // Treat lines under Examples (or similar) as OTHER
            if section_header.to_lowercase().contains("example") {
                return Some(ChildLine {
                    line_type: OutputLine::Other(LineOther {
                        line_contents: line.to_string(),
                        parent_header: section_header.to_string(),
                        components: None,
                    }),
                });
            }

            // If line looks like a command with description
            if line_components.len() >= 2 {
                return Some(ChildLine {
                    line_type: OutputLine::Command(LineCommand {
                        name: line_components[0].to_string(),
                        description: line_components[1].to_string(),
                        parent_header: section_header.to_string(),
                        children: vec![],
                        parent: command.to_string(),
                    }),
                });
            }
        }

        // Fallback to OTHER
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

// fn is_header_line(line: &str) -> bool {
//     if line.starts_with(" ") {
//         return false;
//     } else {
//         if line.contains(":") {
//             return true;
//         } else {
//             let stripped_line = line.trim();
//             if stripped_line.is_empty() {
//                 return false;
//             } else {
//                 return true;
//             }
//         }
//     }
// }

fn handle_child_line(command: &str, section_header_name: &str, line: &str) -> Option<(ChildLineType, String)> {
    let mut line_type_group: Option<(ChildLineType, String)> = None;

        let child_line = parse_child_line(command, line, Some(section_header_name));
        match child_line {
            Some(line) => {
                match line.line_type {
                    OutputLine::Usage(usage) => {
                        line_type_group = Some((ChildLineType::USAGE, serde_json::to_string(&usage).expect("Failed to serialize usage line")));
                    }
                    OutputLine::Command(command) => {
                        line_type_group = Some((ChildLineType::COMMAND, serde_json::to_string(&command).expect("Failed to serialize command line")));
                    }
                    OutputLine::Flag(flag) => {
                        line_type_group = Some((ChildLineType::FLAG, serde_json::to_string(&flag).expect("Failed to serialize flag line")));
                    }
                    OutputLine::Other(other) => {
                        line_type_group = Some((ChildLineType::OTHER, serde_json::to_string(&other).expect("Failed to serialize 'other' line")));
                    }
                }
            },
            None => ()
        }

        line_type_group

}

fn parse_help_output_dynamic(
  base_command: &str,
  command: &str,
  output: &str,
  visited: &mut HashSet<String>
) -> Value {
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
          current_section_header = trimmed_line.strip_suffix(":")
              .map(|s| s.to_string())
              .or(Some(trimmed_line.to_string()));
          if current_section_header != previous_section_header {
              previous_section_header = current_section_header.clone();
          }
          continue;
      } else if let Some(section) = &current_section_header {
          if line.starts_with("  ") || line.starts_with("\t") {
              if let Some((line_type, child_json_str)) = handle_child_line(
                  command.split_whitespace().last().unwrap_or(""),
                  section,
                  line,
              ) {
                  let mut child_value: Value = serde_json::from_str(&child_json_str).unwrap();

                  match line_type {
                      ChildLineType::COMMAND => {
                          if let Some(cmd_name) = child_value.get("name").and_then(|v| v.as_str()) {
                              let cmd_name = cmd_name.to_string();

                              if let Some(obj) = child_value.as_object_mut() {
                                  obj.insert("children".to_string(), json!({
                                      "COMMAND": {}, "FLAG": [], "USAGE": [], "OTHER": []
                                  }));
                              }

                              if let Some(command_map) = components.get_mut("COMMAND").and_then(Value::as_object_mut) {
                                  command_map.insert(cmd_name.clone(), child_value);
                              }

                              // Recursively parse children with output
                              let parent_command = format!("{} {}", command, cmd_name);
                              let help_output = execute_full_command(&format!("{} --help", parent_command));
                            //   let raw_output = execute_full_command(&parent_command);
                              let parsed_children = parse_help_output_dynamic(base_command, &parent_command, &help_output["stdout"].as_str().unwrap_or_default(), visited);

                              if let Some(command_map) = components.get_mut("COMMAND").and_then(Value::as_object_mut) {
                                  if let Some(cmd_obj) = command_map.get_mut(&cmd_name) {
                                      if let Some(cmd_obj_map) = cmd_obj.as_object_mut() {
                                          cmd_obj_map.insert("children".to_string(), parsed_children.get("children").cloned().unwrap_or_default());
                                          cmd_obj_map.insert("outputs".to_string(), json!({
                                              "help_page": help_output,
                                            //   "_": raw_output
                                          }));
                                      }
                                  }
                              }
                          }
                      },
                      ChildLineType::FLAG => {
                          components["FLAG"].as_array_mut().unwrap().push(child_value);
                      },
                      ChildLineType::USAGE => {
                          components["USAGE"].as_array_mut().unwrap().push(child_value);
                      },
                      ChildLineType::OTHER => {
                          components["OTHER"].as_array_mut().unwrap().push(child_value);
                      }
                  }
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
      "version": get_program_version(base_command)
  });

  let help_output = execute_full_command(&format!("{} --help", current_command_name));
//   let raw_output = execute_full_command(&current_command_name);

  structure["outputs"] = json!({
      "help_page": help_output,
    //   "_": raw_output
  });

  let mut visited = HashSet::new();
  let parsed = parse_help_output_dynamic(base_command, &current_command_name, &structure["outputs"]["help_page"]["stdout"].as_str().unwrap_or_default(), &mut visited);

  structure["description"] = parsed.get("description").cloned().unwrap_or(json!(""));
  structure["children"] = parsed.get("children").cloned().unwrap_or(json!({}));

  structure
}
