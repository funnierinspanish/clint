use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ComponentType {
    Flag,
    Argument,
    Keyword,
    Group,
    AlternativeGroup,
    KeyValuePair,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UsageComponent {
    pub component_type: ComponentType,
    pub name: String,
    pub required: bool,
    pub repeatable: bool,
    pub key_value: bool,
    pub alternatives: Vec<UsageComponent>,
    pub children: Vec<UsageComponent>,
}

#[derive(Serialize, Debug)]
pub struct CLIKeywords {
    pub base_program: String,
    pub commands: Vec<String>,
    pub subcommands: Vec<String>,
    pub short_flags: Vec<String>,
    pub long_flags: Vec<String>,
}

pub struct CLISummary {
    pub unique_keywords_count: usize,
    pub unique_command_count: usize,
    pub unique_subcommand_count: usize,
    pub unique_short_flag_count: usize,
    pub unique_long_flag_count: usize,
    pub total_command_count: usize,
    pub total_subcommand_count: usize,
    pub total_short_flag_count: usize,
    pub total_long_flag_count: usize,
}

pub enum FileOutputFormat {
    Markdown,
    JSON,
    Text,
}

impl FileOutputFormat {
    pub fn from_str(format: Option<&String>) -> Option<Self> {
        match format {
            Some(f) => match f.to_lowercase().as_str() {
                "json" => Some(FileOutputFormat::JSON),
                "md" => Some(FileOutputFormat::Markdown),
                "text" => Some(FileOutputFormat::Text),
                _ => Some(FileOutputFormat::Text),
            },
            None => Some(FileOutputFormat::JSON),
        }
    }
    pub fn from_file_ext(format: &str) -> Option<Self> {
        match format.to_lowercase().as_str() {
            "json" => Some(FileOutputFormat::JSON),
            "md" => Some(FileOutputFormat::Markdown),
            "txt" => Some(FileOutputFormat::Text),
            _ => Some(FileOutputFormat::Text),
        }
    }
}
pub struct OutputFile {
    pub path: PathBuf,
    pub format: FileOutputFormat,
}

impl OutputFile {
    pub fn new(path: &PathBuf, format: FileOutputFormat) -> Self {
        OutputFile { path: path.clone(), format }
    }
    pub fn write_json_output_file(&self, content: Value) {
        self.write(&serde_json::to_string_pretty(&content).expect("Failed to serialize JSON"));
    }
    pub fn write_markdown_output(&self, content: &str) {
        std::fs::write(&self.path, content).expect("Failed to write output file");
    }
    pub fn write_plain_output(&self, content: &str) {
        std::fs::write(&self.path, content).expect("Failed to write output file");
    }

    fn write(&self, content: &str) {
        fs::create_dir_all(self.path.parent().expect("Failed to create path")).expect("Failed to create directory");
        fs::write(&self.path, content).expect("Failed to write output file");
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Serialize)]
pub enum ChildLineType {
    FLAG,
    COMMAND,
    USAGE,
    OTHER,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LineCommand {
    pub name: String,
    pub description: String,
    pub children: Vec<LineCommand>,
    pub parent_header: String,
    pub parent: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LineFlag {
    pub short: Option<String>,
    pub long: Option<String>,
    pub data_type: Option<String>,
    pub description: Option<String>,
    pub parent_header: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LineUsage {
    pub usage_string: String,
    pub parent_header: String,
    pub usage_components: Vec<UsageComponent>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LineOther {
    pub line_contents: String,
    pub parent_header: String,
    pub components: Option<Vec<UsageComponent>>,
}

pub enum OutputLine {
    Other(LineOther),
    Usage(LineUsage),
    Command(LineCommand),
    Flag(LineFlag),
}

pub struct ChildLine {
    pub line_type: OutputLine,
}
