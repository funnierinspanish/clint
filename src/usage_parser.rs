use regex::Regex;

use crate::models::{UsageComponent, ComponentType};

pub fn parse_usage_line(child_line: &str, command_name: &str) -> Vec<UsageComponent> {
    let mut line = child_line.trim();

    // Remove "Usage:" and command path prefix
    if let Some(idx) = line.find(command_name) {
        line = &line[idx + command_name.len()..];
    }

    let line = line.trim();
    if line.starts_with('-') {
        return vec![];
    }
    parse_tokens(line)
}

fn parse_tokens(line: &str) -> Vec<UsageComponent> {
    let mut components = Vec::new();
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.peek() {
        match c {
            '[' => {
                chars.next(); // consume '['
                let group_str = extract_until_matching(&mut chars, '[', ']');
                let children = parse_tokens(&group_str);
                components.push(UsageComponent {
                    component_type: ComponentType::Group,
                    name: String::new(),
                    required: false,
                    repeatable: group_str.ends_with("..."),
                    key_value: false,
                    alternatives: vec![],
                    children,
                });
            }
            '(' => {
                chars.next(); // consume '('
                let group_str = extract_until_matching(&mut chars, '(', ')');
                let children = parse_alternatives(&group_str);
                if children.is_empty() {
                    continue;
                }
                components.push(UsageComponent {
                    component_type: ComponentType::AlternativeGroup,
                    name: String::new(),
                    required: true,
                    repeatable: group_str.ends_with("..."),
                    key_value: false,
                    alternatives: children,
                    children: vec![],
                });
            }
            _ => {
                let token = extract_token(&mut chars);
                if token.is_empty() {
                    continue;
                }

                let repeatable = token.ends_with("...");
                let token_clean = if repeatable {
                    token.trim_end_matches("...").trim().to_string()
                } else {
                    token.clone()
                };

                let key_value_re = Regex::new(r"^<[^>]+>=<[^>]+>$").unwrap();
                let (name, key_value) = if key_value_re.is_match(&token_clean) {
                    (token_clean, true)
                } else {
                    (token_clean, token.contains('='))
                };

                let component_type = if name.starts_with("--") {
                    ComponentType::Flag
                } else if name.starts_with("<") && name.ends_with(">") || key_value {
                    ComponentType::Argument
                } else {
                    ComponentType::Keyword
                };

                components.push(UsageComponent {
                    component_type,
                    name,
                    required: true,
                    repeatable,
                    key_value,
                    alternatives: vec![],
                    children: vec![],
                });
            }
        }
    }

    components
}

fn extract_token<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> String {
  chars
    .by_ref()
    .take_while(|&c| !matches!(c, ' ' | '[' | ']' | '(' | ')' | '|'))
    .collect::<String>()
    .trim_end()
    .to_string()
}

fn extract_until_matching<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>, open: char, close: char) -> String {
    let mut content = String::new();
    let mut depth = 1;

    while let Some(c) = chars.next() {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        content.push(c);
    }
    content.trim().to_string()
}

fn parse_alternatives(group: &str) -> Vec<UsageComponent> {
    group
        .split('|')
        .map(|part| parse_tokens(part.trim()))
        .flatten()
        .collect()
}
