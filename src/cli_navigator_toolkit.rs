use std::path::Path;
use std::path::PathBuf;
use cli_parser::extract_cli_structure;
use keyword_extractor::extract_keywords_from_json;
use serde_json::json;
use crate::models::FileOutputFormat;

use crate::cli_parser;
use crate::keyword_extractor;
use crate::models::OutputFile;
use crate::replicator;
use crate::summary_generator::generate_summary;

pub fn run_cli_parser(command: &str, output_path: &PathBuf) {
    let structure: serde_json::Value = extract_cli_structure(command, None);
    let program_name = structure.get("name").expect("Failed to get program name").as_str().expect("Failed to convert program name to string");
    let program_version = structure.get("version").expect("Failed to get program version").as_str().expect("Failed to convert program version to string");
    let out_path = if output_path.exists() {
        output_path
    } else {
        &PathBuf::from(format!("./out/{}/{}/{}-structure.json", program_name, program_version, program_name))
    };
    let out_file: OutputFile = OutputFile::new(out_path, FileOutputFormat::JSON);

    out_file.write_json_output_file(structure);

    println!("CLI structure JSON file saved to: {:?}", out_path.to_str());
}

pub fn run_keyword_extractor(input_json: &PathBuf, output_path: &PathBuf, format: FileOutputFormat) {
    let keywords = extract_keywords_from_json(input_json).expect("Failed to analyze CLI JSON");
    let out_file: OutputFile = OutputFile::new(output_path, format);

    
    
    match out_file.format {
        FileOutputFormat::Markdown => {
            let keywords_md = format!(
                "# `{}`\n\n## First level commands\n\n{}\n\n## All subcommands\n\n{}\n\n## Short flags\n\n{}\n\n## Long flags\n\n{}",
                keywords.base_program,
                keywords.commands.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("\n- "),
                keywords.subcommands.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "),
                keywords.short_flags.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "),
                keywords.long_flags.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ")
            );
            out_file.write_markdown_output(&keywords_md.to_string());
        }
        FileOutputFormat::JSON => {
            let keywords_json = json!({
                "base_program": keywords.base_program,
                "commands": keywords.commands,
                "subcommands": keywords.subcommands,
                "short_flags": keywords.short_flags,
                "long_flags": keywords.long_flags,
            });
            out_file.write_json_output_file(keywords_json);
        }
        FileOutputFormat::Text => {
            let keywords_txt = format!(
                "{}:\n\nFirst level commands:\n{}\n\nAll subcommands:\n{}\n\nShort flags:\n{}\n\nLong flags:\n{}",
                keywords.base_program,
                keywords.commands.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("\n- "),
                keywords.subcommands.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "),
                keywords.short_flags.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "),
                keywords.long_flags.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ")
            );
            out_file.write_plain_output(&keywords_txt.to_string());
        }
    }
}

pub fn run_summary_generator(input_json: &PathBuf, output_path: &PathBuf, format: FileOutputFormat) {
    let summary = generate_summary(input_json).expect("Failed to analyze CLI JSON");
    let out_file: OutputFile = OutputFile::new(output_path, format);

    
    match out_file.format {
        FileOutputFormat::Markdown => {
            let summary_md = format!(
                "# CLI Summary\n\n## Unique Keywords Count\n\n{}\n\n## Unique Command Count\n\n{}\n\n## Unique Subcommand Count\n\n{}\n\n## Unique Short Flag Count\n\n{}\n\n## Unique Long Flag Count\n\n{}\n\n## Total Command Count\n\n{}\n\n## Total Subcommand Count\n\n{}\n\n## Total Short Flag Count\n\n{}\n\n## Total Long Flag Count\n\n{}",
                summary.unique_keywords_count,
                summary.unique_command_count,
                summary.unique_subcommand_count,
                summary.unique_short_flag_count,
                summary.unique_long_flag_count,
                summary.total_command_count,
                summary.total_subcommand_count,
                summary.total_short_flag_count,
                summary.total_long_flag_count
            );
            out_file.write_markdown_output(&summary_md.to_string());
        }
        FileOutputFormat::JSON => {
            let summary_json = json!({
                "unique_keywords_count": summary.unique_keywords_count,
                "unique_command_count": summary.unique_command_count,
                "unique_subcommand_count": summary.unique_subcommand_count,
                "unique_short_flag_count": summary.unique_short_flag_count,
                "unique_long_flag_count": summary.unique_long_flag_count,
                "total_command_count": summary.total_command_count,
                "total_subcommand_count": summary.total_subcommand_count,
                "total_short_flag_count": summary.total_short_flag_count,
                "total_long_flag_count": summary.total_long_flag_count,
            });
            out_file.write_json_output_file(summary_json);
        }
        FileOutputFormat::Text => {
            let summary_txt = format!(
                "Unique Keywords Count: {}\n\nUnique Command Count: {}\n\nUnique Subcommand Count: {}\n\nUnique Short Flag Count: {}\n\nUnique Long Flag Count: {}\n\nTotal Command Count: {}\n\nTotal Subcommand Count: {}\n\nTotal Short Flag Count: {}\n\nTotal Long Flag Count: {}",
                summary.unique_keywords_count,
                summary.unique_command_count,
                summary.unique_subcommand_count,
                summary.unique_short_flag_count,
                summary.unique_long_flag_count,
                summary.total_command_count,
                summary.total_subcommand_count,
                summary.total_short_flag_count,
                summary.total_long_flag_count
            );
            out_file.write_plain_output(&summary_txt.to_string());
        }
    }
    
}

pub fn run_webpage_generator(input_json: &PathBuf, output_path: &PathBuf) {
    // Copy input JSON file to output path
    std::fs::copy(input_json, output_path.join("cli-structure.json")).expect("Failed to copy file");
   
    // Copy the contents of ./src/web to the output path
    let src_path = Path::new("./src/web");
    for entry in std::fs::read_dir(src_path).expect("Failed to read directory") {
        let entry = entry.expect("Failed to read entry");
        let src_file_path = entry.path();
        let dest_file_path = output_path.join(src_file_path.file_name().expect("Failed to get file name"));
        std::fs::copy(&src_file_path, &dest_file_path).expect("Failed to copy file");
    }
    
}

pub fn run_cli_replicator(input_json: &PathBuf, output_path: &PathBuf, keep_help_flags: bool, keep_verbose_flags: bool) {
    replicator::replicate(input_json, output_path, keep_help_flags, keep_verbose_flags).expect("Failed to replicate CLI");
}