use crate::models::FileOutputFormat;
use cli_parser::extract_cli_structure;
use dialoguer::{Confirm, Select};
use keyword_extractor::extract_keywords_from_json;
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use warp::Filter;

use crate::cli_parser;
use crate::comparison;
use crate::keyword_extractor;
use crate::models::OutputFile;
use crate::replicator;
use crate::summary_generator::generate_summary;

pub fn run_get_template_web_files(force: bool) {
    let home_dir = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .expect("Could not find home directory");

    let templates_dir = PathBuf::from(home_dir)
        .join(".config")
        .join("clint")
        .join("templates");

    let default_template_dir = templates_dir.join("default");

    // Create the templates directory if it doesn't exist
    fs::create_dir_all(&templates_dir).expect("Failed to create templates directory");

    if default_template_dir.exists() && !force {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_nanos();

        let mut hasher = DefaultHasher::new();
        timestamp.hash(&mut hasher);
        let hash = hasher.finish();
        let backup_hash = format!("{:06x}", hash % 0x1000000);

        let backup_dir = templates_dir.join(format!("default_backup_{}", backup_hash));

        println!("WARNING: Default template directory already exists");
        println!("Creating backup: {}", backup_dir.display());

        fs::rename(&default_template_dir, &backup_dir)
            .expect("Failed to create backup of existing default template");
    }

    fs::create_dir_all(&default_template_dir).expect("Failed to create default template directory");

    println!(
        "Getting web interface files to: {}",
        default_template_dir.display()
    );

    match download_template_from_github(&default_template_dir) {
        Ok(()) => {
            println!("\nWeb interface template download complete!");
            println!("Files saved to: {}", default_template_dir.display());
            println!(
                "Tip: These files can be customized. The serve command will use your custom template when available."
            );
        }
        Err(e) => {
            println!("✗ Failed to download template: {}", e);
            show_manual_template_download_instructions(&default_template_dir);
        }
    }
}

fn check_and_offer_template_download() -> Option<PathBuf> {
    let home_dir = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .expect("Could not find home directory");

    let templates_dir = PathBuf::from(home_dir)
        .join(".config")
        .join("clint")
        .join("templates");

    let default_template_dir = templates_dir.join("default");

    // Check if default template directory exists and has files
    let template_exists = if default_template_dir.exists() {
        // Check if directory has the required files
        let required_files = ["index.html", "script.js", "cli-command-card.js"];
        required_files.iter().all(|&file| {
            let file_path = default_template_dir.join(file);
            file_path.exists() && fs::metadata(&file_path).is_ok_and(|meta| meta.len() > 0)
        })
    } else {
        false
    };

    if template_exists {
        return Some(default_template_dir);
    }

    // Template doesn't exist or is incomplete
    println!("\nDefault web template not found or incomplete.");
    println!("The serve command needs web interface files to display CLI data.");
    println!();

    // Check if we're in an interactive terminal
    let is_interactive = atty::is(atty::Stream::Stdin);

    let should_download = if is_interactive {
        match Confirm::new()
            .with_prompt("Would you like to download the default template from GitHub?")
            .default(true)
            .interact()
        {
            Ok(response) => response,
            Err(_) => {
                println!("Unable to get user input, defaulting to manual template download.");
                false
            }
        }
    } else {
        println!("Non-interactive environment detected.");
        println!("To download templates automatically, run: clint get-template");
        println!("Or manually download from GitHub (see instructions below).");
        false
    };

    if should_download {
        // Create the templates directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&templates_dir) {
            println!("Failed to create templates directory: {}", e);
            return None;
        }

        // Download template files from GitHub
        match download_template_from_github(&default_template_dir) {
            Ok(()) => {
                println!("✓ Template downloaded successfully!");
                Some(default_template_dir)
            }
            Err(e) => {
                println!("✗ Failed to download template: {}", e);
                show_manual_template_download_instructions(&default_template_dir);
                None
            }
        }
    } else {
        show_manual_template_download_instructions(&default_template_dir);
        None
    }
}

fn download_template_from_github(target_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    println!("Downloading template files from GitHub...");

    // Create target directory
    fs::create_dir_all(target_dir)?;

    let base_url = "https://raw.githubusercontent.com/funnierinspanish/clint/main/src/web";
    let files = [
        ("index.html", "index.html"),
        ("script.js", "script.js"),
        ("cli-command-card.js", "cli-command-card.js"),
    ];

    for (filename, url_path) in &files {
        let url = format!("{}/{}", base_url, url_path);
        let target_path = target_dir.join(filename);

        println!("  Downloading {}...", filename);

        // Try using curl first, then wget as fallback
        let download_success = Command::new("curl")
            .args(["-fsSL", &url, "-o", target_path.to_str().unwrap()])
            .status()
            .map(|status| status.success())
            .unwrap_or(false);

        if !download_success {
            // Try wget as fallback
            let wget_success = Command::new("wget")
                .args(["-q", &url, "-O", target_path.to_str().unwrap()])
                .status()
                .map(|status| status.success())
                .unwrap_or(false);

            if !wget_success {
                return Err(
                    format!("Failed to download {} (tried curl and wget)", filename).into(),
                );
            }
        }

        // Verify the file was downloaded and is not empty
        if !target_path.exists() || fs::metadata(&target_path)?.len() == 0 {
            return Err(format!("Downloaded file {} is empty or missing", filename).into());
        }
    }

    Ok(())
}

fn show_manual_template_download_instructions(target_dir: &Path) {
    println!();
    println!("Manual template download instructions:");
    println!("1. Download the template files from:");
    println!("   https://github.com/funnierinspanish/clint/tree/main/src/web");
    println!("2. Save them to this directory:");
    println!("   {}", target_dir.display());
    println!("3. Required files:");
    println!("   - index.html");
    println!("   - script.js");
    println!("   - cli-command-card.js");
    println!();
    println!("Alternatively, you can run 'clint get-template' to download the default template.");
    println!();
}

pub fn run_cli_parser(
    command: &str,
    output_path: Option<&PathBuf>,
    format: Option<&String>,
    tag: Option<&String>,
) {
    use crate::models::ParseOutputFormat;

    // First try to load existing JSON file, fall back to re-parsing if not found
    let structure: serde_json::Value = {
        let json_filename = format!("{}.json", command.split('/').next_back().unwrap_or("cli"));
        let json_path = Path::new(&json_filename);
        if json_path.exists() {
            let json_content = fs::read_to_string(json_path).expect("Failed to read JSON file");
            serde_json::from_str(&json_content).expect("Failed to parse JSON file")
        } else {
            extract_cli_structure(command, None)
        }
    };
    let program_name = structure
        .get("name")
        .expect("Failed to get program name")
        .as_str()
        .expect("Failed to convert program name to string");
    let program_version = structure
        .get("version")
        .expect("Failed to get program version")
        .as_str()
        .expect("Failed to convert program version to string");

    // Determine output format
    let output_format = match format {
        Some(fmt) => ParseOutputFormat::from_str(fmt).unwrap_or_else(|| {
            println!("Warning: Unknown format '{}', defaulting to JSON", fmt);
            ParseOutputFormat::Json
        }),
        None => ParseOutputFormat::Json,
    };

    // Determine output path with appropriate extension
    let out_path = match output_path {
        Some(path) => {
            match tag {
              Some(t) => {
                path.join(program_name).join(t).join(format!("parsed.{}", output_format.get_file_extension()))
            },
            None => {
                println!("Using custom output path: {:?}", path);
                path.clone()
            }
          }
        }
        None => {
            // Use new default structure: ./out/<program_name>/<version_or_tag>/
            let version_or_tag = tag.cloned().unwrap_or_else(|| {
                if program_version.is_empty() || program_version == "Unknown" {
                    "latest".to_string()
                } else {
                    program_version.to_string()
                }
            });

            let base_dir = PathBuf::from("./out").join(program_name).join(version_or_tag);

            // Create directory if it doesn't exist
            if let Err(e) = fs::create_dir_all(&base_dir)
                && e.kind() != std::io::ErrorKind::AlreadyExists
            {
                panic!("Failed to create output directory: {}", e);
            }

            let filename = match output_format {
                ParseOutputFormat::TypeScriptDirectory => program_name.to_string(),
                _ => format!("parsed.{}", output_format.get_file_extension()),
            };

            base_dir.join(filename)
        }
    };

    match output_format {
        ParseOutputFormat::Json => {
            let out_file: OutputFile = OutputFile::new(&out_path, FileOutputFormat::Json);
            out_file.write_json_output_file(structure);
            println!("CLI structure JSON file saved successfully!");
        }
        ParseOutputFormat::JsonSchema => {
            generate_json_schema(&out_path);
            println!("JSON Schema file saved successfully!");
        }
        ParseOutputFormat::ZodSchema => {
            generate_zod_schema(&out_path);
            println!("Zod TypeScript schema file saved successfully!");
        }
        ParseOutputFormat::TypeScriptDirectory => {
            generate_typescript_directory(&structure, &out_path, program_version);
            println!("TypeScript directory structure created successfully!");
        }
    }

    println!("Location: {}", out_path.display());

    if output_path.is_none() {
        println!("Tip: Files are organized by program name and version in ~/.config/clint/parsed/");
    }
}

pub fn run_keyword_extractor(
    input_json: &PathBuf,
    output_path: &std::path::Path,
    format: FileOutputFormat,
) {
    let keywords = extract_keywords_from_json(input_json).expect("Failed to analyze CLI JSON");
    let out_file: OutputFile = OutputFile::new(output_path, format);

    match out_file.format {
        FileOutputFormat::Markdown => {
            let keywords_md = format!(
                "# `{}`\n\n## First level commands\n\n{}\n\n## All subcommands\n\n{}\n\n## Short flags\n\n{}\n\n## Long flags\n\n{}",
                keywords.base_program,
                keywords
                    .commands
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join("\n- "),
                keywords
                    .subcommands
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
                keywords
                    .short_flags
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
                keywords
                    .long_flags
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            out_file.write_markdown_output(&keywords_md.to_string());
        }
        FileOutputFormat::Json => {
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
                keywords
                    .commands
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join("\n- "),
                keywords
                    .subcommands
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
                keywords
                    .short_flags
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
                keywords
                    .long_flags
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            out_file.write_plain_output(&keywords_txt.to_string());
        }
        FileOutputFormat::Csv => {
            let mut csv_content = String::new();
            csv_content.push_str("type,value\n");

            // Add base program
            csv_content.push_str(&format!("base_program,{}\n", keywords.base_program));

            // Add commands
            for command in &keywords.commands {
                csv_content.push_str(&format!("command,{}\n", command));
            }

            // Add subcommands
            for subcommand in &keywords.subcommands {
                csv_content.push_str(&format!("subcommand,{}\n", subcommand));
            }

            // Add short flags
            for flag in &keywords.short_flags {
                csv_content.push_str(&format!("short_flag,{}\n", flag));
            }

            // Add long flags
            for flag in &keywords.long_flags {
                csv_content.push_str(&format!("long_flag,{}\n", flag));
            }

            out_file.write_csv_output(&csv_content);
        }
    }
}

pub fn run_summary_generator(
    input_json: &PathBuf,
    output_path: &std::path::Path,
    format: FileOutputFormat,
) {
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
        FileOutputFormat::Json => {
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
        FileOutputFormat::Csv => {
            let csv_content = format!(
                "metric,value\nunique_keywords_count,{}\nunique_command_count,{}\nunique_subcommand_count,{}\nunique_short_flag_count,{}\nunique_long_flag_count,{}\ntotal_command_count,{}\ntotal_subcommand_count,{}\ntotal_short_flag_count,{}\ntotal_long_flag_count,{}\n",
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
            out_file.write_csv_output(&csv_content);
        }
    }
}

pub fn run_interactive_serve(
    template: Option<&String>,
    port: Option<u16>,
    input_file: Option<&PathBuf>,
) {
    let home_dir = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .expect("Could not find home directory");

    // Check if specific input file is provided
    if let Some(input_path) = input_file {
        serve_specific_file(input_path, template, port);
        return;
    }

    // Show interactive selection (default behavior)
    let parsed_dir = PathBuf::from(home_dir.clone())
        .join(".config")
        .join("clint")
        .join("parsed");

    if !parsed_dir.exists() {
        println!("No parsed CLI data found");
        println!("\n  Run 'clint parse <program>' first to create some CLI data");
        return;
    }

    serve_with_interactive_selection(&parsed_dir, port);
}

fn serve_specific_file(input_path: &PathBuf, template: Option<&String>, port: Option<u16>) {
    let home_dir = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .expect("Could not find home directory");

    // Validate that the input file exists and is not empty
    if !input_path.exists() {
        println!("Input file not found: {}", input_path.display());
        return;
    }

    let metadata = match fs::metadata(input_path) {
        Ok(meta) => meta,
        Err(e) => {
            println!("Failed to read file metadata: {}", e);
            return;
        }
    };

    if metadata.len() == 0 {
        println!("Input file is empty: {}", input_path.display());
        return;
    }

    if input_path.extension().is_none_or(|ext| ext != "json") {
        println!("Input file must be a JSON file: {}", input_path.display());
        return;
    }

    // Validate JSON content
    match fs::read_to_string(input_path) {
        Ok(content) => {
            if let Err(e) = serde_json::from_str::<serde_json::Value>(&content) {
                println!("Invalid JSON file: {}", e);
                return;
            }
        }
        Err(e) => {
            println!("Failed to read file: {}", e);
            return;
        }
    }

    // Determine template to use - check for custom template, then default template, then embedded
    let template_name = template.map(|s| s.as_str()).unwrap_or("default");
    let custom_template_path = PathBuf::from(home_dir)
        .join(".config")
        .join("clint")
        .join("templates")
        .join(template_name);

    let (template_path, template_source) = if custom_template_path.exists()
        && template_name != "default"
    {
        // Use custom template
        (
            custom_template_path,
            format!("custom template: {}", template_name),
        )
    } else if template_name == "default" {
        // For default template, check and offer to download if needed
        match check_and_offer_template_download() {
            Some(default_path) => (default_path, "downloaded template".to_string()),
            None => {
                println!("Cannot serve without web templates. Please:");
                println!("1. Run 'clint get-template' to download templates");
                println!(
                    "2. Or manually download files from GitHub to ~/.config/clint/templates/default/"
                );
                return;
            }
        }
    } else {
        // Requested template doesn't exist
        println!(
            "Template '{}' not found: {}",
            template_name,
            custom_template_path.display()
        );
        println!("Available templates:");
        let templates_dir = custom_template_path.parent().unwrap();
        if let Ok(entries) = fs::read_dir(templates_dir) {
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|ft| ft.is_dir())
                    && let Some(name) = entry.file_name().to_str()
                {
                    println!("  - {}", name);
                }
            }
        } else {
            println!("  (no templates directory found)");
        }
        println!("Cannot serve without templates.");
        return;
    };

    // Extract app name and version from file path/name for display
    let file_name = input_path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let version = extract_version_from_filename(file_name);
    let app_name = if let Some(dash_pos) = file_name.rfind('-') {
        &file_name[..dash_pos]
    } else {
        file_name
    };

    println!(
        "Starting HTTP server for {} version {}...",
        app_name, version
    );
    println!("Template: {}", template_source);
    println!("JSON file: {}", input_path.display());

    // Start HTTP server with specific JSON file
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(start_http_server(
        input_path.clone(),
        template_path,
        app_name.to_string(),
        version,
        port,
    ));
}

fn serve_with_interactive_selection(parsed_dir: &PathBuf, port: Option<u16>) {
    // Get all directories with JSON files
    let mut apps_with_data = Vec::new();

    if let Ok(entries) = fs::read_dir(parsed_dir) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                let app_dir = entry.path();
                if let Some(app_name) = app_dir.file_name().and_then(|n| n.to_str()) {
                    // Check if directory contains JSON files
                    if let Ok(json_files) = fs::read_dir(&app_dir) {
                        let json_count = json_files
                            .flatten()
                            .filter(|file| file.path().extension().is_some_and(|ext| ext == "json"))
                            .filter(|file| {
                                // Check if file is non-empty
                                file.metadata().is_ok_and(|meta| meta.len() > 0)
                            })
                            .count();

                        if json_count > 0 {
                            apps_with_data.push((app_name.to_string(), app_dir, json_count));
                        }
                    }
                }
            }
        }
    }

    if apps_with_data.is_empty() {
        println!("No CLI applications with JSON data found");
        println!("Run 'clint parse <program>' first to create some CLI data");
        return;
    }

    // Sort apps alphabetically
    apps_with_data.sort_by(|a, b| a.0.cmp(&b.0));

    // Present app selection menu
    let app_options: Vec<String> = apps_with_data
        .iter()
        .map(|(name, _, count)| {
            format!(
                "{} ({} version{})",
                name,
                count,
                if *count == 1 { "" } else { "s" }
            )
        })
        .collect();

    println!("Select a CLI application to serve:");
    let app_selection = Select::new()
        .items(&app_options)
        .default(0)
        .interact()
        .expect("Failed to get user selection");

    let (selected_app, selected_app_dir, _) = &apps_with_data[app_selection];

    // Get JSON files for selected app
    let mut json_files = Vec::new();
    if let Ok(entries) = fs::read_dir(selected_app_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|ext| ext == "json")
                && let Some(filename) = entry.file_name().to_str()
                && let Ok(metadata) = entry.metadata()
                && metadata.len() > 0
            {
                json_files.push((filename.to_string(), entry.path(), metadata));
            }
        }
    }

    // Sort versions
    json_files.sort_by(|a, b| {
        let version_a = extract_version_from_filename(&a.0);
        let version_b = extract_version_from_filename(&b.0);

        match (parse_semver(&version_a), parse_semver(&version_b)) {
            (Some(v_a), Some(v_b)) => {
                // Compare major, then minor, then patch (descending order)
                match v_b.0.cmp(&v_a.0) {
                    std::cmp::Ordering::Equal => match v_b.1.cmp(&v_a.1) {
                        std::cmp::Ordering::Equal => v_b.2.cmp(&v_a.2),
                        other => other,
                    },
                    other => other,
                }
            }
            (Some(_), None) => std::cmp::Ordering::Less, // Semver comes first
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => {
                // Fall back to creation date (descending)
                b.2.created()
                    .unwrap_or(SystemTime::UNIX_EPOCH)
                    .cmp(&a.2.created().unwrap_or(SystemTime::UNIX_EPOCH))
            }
        }
    });

    // Present version selection menu
    let version_options: Vec<String> = json_files
        .iter()
        .map(|(filename, _, metadata)| {
            let version = extract_version_from_filename(filename);
            let created = metadata
                .created()
                .ok()
                .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|duration| {
                    let secs = duration.as_secs();
                    format!("created {}", format_timestamp(secs))
                })
                .unwrap_or_else(|| "unknown date".to_string());

            format!("{} ({})", version, created)
        })
        .collect();

    println!("\nSelect a version of {} to serve:", selected_app);
    let version_selection = Select::new()
        .items(&version_options)
        .default(0)
        .interact()
        .expect("Failed to get user selection");

    let selected_json_path = &json_files[version_selection].1;
    let selected_version = extract_version_from_filename(&json_files[version_selection].0);

    // Check and offer to download default template if needed
    let template_path = match check_and_offer_template_download() {
        Some(path) => path,
        None => {
            println!("Cannot serve without web templates. Please:");
            println!("1. Run 'clint get-template' to download templates");
            println!(
                "2. Or manually download files from GitHub to ~/.config/clint/templates/default/"
            );
            return;
        }
    };
    let template_source = "downloaded template";

    // Start HTTP server with selected JSON data
    println!(
        "Starting HTTP server for {} version {}...",
        selected_app, selected_version
    );
    println!("Template: {}", template_source);

    // Run the async server
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(start_http_server(
        selected_json_path.clone(),
        template_path,
        selected_app.clone(),
        selected_version,
        port,
    ));
}

async fn start_http_server(
    json_path: PathBuf,
    template_path: PathBuf,
    app_name: String,
    version: String,
    port: Option<u16>,
) {
    // Read the JSON content
    let json_content = match fs::read_to_string(&json_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Failed to read JSON file: {}", e);
            return;
        }
    };
    let json_to_serve_path = match json_path.clone().to_str() {
        Some(path) => path.to_string(),
        None => "unknown path".to_string(),
    };

    // Create filter for serving the CLI structure JSON
    let json_content_filter = warp::any().map(move || json_content.clone());
    let cli_structure = warp::path("cli-structure.json")
        .and(warp::get())
        .and(json_content_filter)
        .map(move |content: String| {
            println!(
                "Redirecting client request:\n  cli-structure.json --> {}\n",
                json_to_serve_path
            );
            warp::reply::with_header(content, "content-type", "application/json")
        });

    // Create routes using filesystem templates
    let static_files = warp::fs::dir(template_path.clone()).with(warp::log("template_files"));

    // Add a root redirect to index.html
    let root_redirect = warp::path::end()
        .map(|| warp::redirect::redirect(warp::http::Uri::from_static("/index.html")));

    // Combine routes: JSON first, then root redirect, then static files
    let routes = cli_structure
        .or(root_redirect)
        .or(static_files)
        .with(warp::log("clint_server"))
        .boxed();

    // Use provided port or find an available one starting from 8899
    let server_port = match port {
        Some(p) => {
            // If user specified a port, try to use it directly
            use std::net::TcpListener;
            if TcpListener::bind(("127.0.0.1", p)).is_ok() {
                p
            } else {
                eprintln!("Port {} is not available", p);
                eprintln!("Please try a different port with --port <PORT>");
                return;
            }
        }
        None => {
            // Find an available port starting from 8899
            match find_available_port(8899) {
                Some(p) => p,
                None => {
                    eprintln!("Could not find an available port after 5 attempts");
                    eprintln!("Please specify an available port with --port <PORT>");
                    return;
                }
            }
        }
    };

    let using_custom_template = !template_path.ends_with("templates/default");

    println!("Server starting...");
    println!(
        "Open your browser and navigate to: http://localhost:{}",
        server_port
    );
    println!("Serving: {} version {}", app_name, version);
    if using_custom_template {
        println!("Using custom template: {}", template_path.display());
    } else {
        println!("Using default template");
    }
    println!("Press Ctrl+C to stop the server");
    println!();

    // Start the server
    warp::serve(routes).run(([127, 0, 0, 1], server_port)).await;
}

fn find_available_port(start_port: u16) -> Option<u16> {
    use std::collections::HashSet;
    use std::net::TcpListener;

    // First, try the preferred start port
    if TcpListener::bind(("127.0.0.1", start_port)).is_ok() {
        return Some(start_port);
    }

    // If start port is busy, try up to 4 more random ports (5 attempts total)
    let mut used_ports = HashSet::new();
    used_ports.insert(start_port);

    for _ in 0..4 {
        // Generate a random port in the range 8000-9999 (common development ports)
        let random_port = 8000 + (rand::random::<u16>() % 2000);

        // Skip if we already tried this port
        if used_ports.contains(&random_port) {
            continue;
        }
        used_ports.insert(random_port);

        if TcpListener::bind(("127.0.0.1", random_port)).is_ok() {
            return Some(random_port);
        }
    }

    // No available port found after 5 attempts
    None
}

fn extract_version_from_filename(filename: &str) -> String {
    // Remove .json extension
    let without_ext = filename.trim_end_matches(".json");

    // Look for pattern like "appname-version"
    if let Some(dash_pos) = without_ext.rfind('-') {
        without_ext[dash_pos + 1..].to_string()
    } else {
        "unknown".to_string()
    }
}

fn parse_semver(version: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 3
        && let (Ok(major), Ok(minor), Ok(patch)) = (
            parts[0].parse::<u32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        )
    {
        return Some((major, minor, patch));
    }
    None
}

fn format_timestamp(timestamp: u64) -> String {
    // Simple timestamp formatting - in a real app you'd use chrono
    let days_ago = (SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - timestamp)
        / 86400;

    if days_ago == 0 {
        "today".to_string()
    } else if days_ago == 1 {
        "yesterday".to_string()
    } else {
        format!("{} days ago", days_ago)
    }
}

pub fn run_cli_replicator(
    input_json: &PathBuf,
    output_path: &PathBuf,
    keep_help_flags: bool,
    keep_verbose_flags: bool,
) {
    replicator::replicate(input_json, output_path, keep_help_flags, keep_verbose_flags)
        .expect("Failed to replicate CLI");
}

fn generate_json_schema(output_path: &PathBuf) {
    // Read the existing JSON schema file from the project
    let schema_content = include_str!("schemas/cobra/cobra_cli_structure.schema.json");

    // Create output directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).expect("Failed to create output directory");
    }

    // Write the schema file
    fs::write(output_path, schema_content).expect("Failed to write JSON schema file");
}

fn generate_zod_schema(output_path: &PathBuf) {
    // Read the existing Zod schema file from the project
    let zod_content = include_str!("schemas/cobra/cobra_cli_structure.zod.ts");

    // Create output directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).expect("Failed to create output directory");
    }

    // Write the Zod schema file
    fs::write(output_path, zod_content).expect("Failed to write Zod schema file");
}

fn generate_typescript_directory(structure: &serde_json::Value, output_path: &PathBuf, program_version: &str) {
    // Create the main directory
    fs::create_dir_all(output_path).expect("Failed to create output directory");

    // Generate the main schema file
    let main_schema_content = include_str!("schemas/cobra/cobra_cli_structure.zod.ts");
    let main_schema_path = output_path.join("schema.ts");
    fs::write(&main_schema_path, main_schema_content).expect("Failed to write main schema file");

    // Generate naming convention file
    let naming_convention_content = include_str!("schemas/cobra/naming-convention.ts");
    let naming_convention_path = output_path.join("naming-convention.ts");
    fs::write(&naming_convention_path, naming_convention_content).expect("Failed to write naming convention file");

    // Generate command components file
    let command_components_content = include_str!("schemas/cobra/command-components.ts");
    let command_components_path = output_path.join("command-components.ts");
    fs::write(&command_components_path, command_components_content).expect("Failed to write command components file");

    // Generate index file with exports
    let mut index_content = String::new();
    index_content.push_str("// Auto-generated command exports\n");
    index_content.push_str("export * from './schema';\n\n");
    index_content.push_str(format!("export const version = '{}';\n", program_version).as_str());
    

    // Extract program info
    let program_name = structure
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("cli");

    // Process commands recursively
    if let Some(children) = structure.get("children").and_then(|v| v.as_object())
        && let Some(command_map) = children.get("COMMAND").and_then(|v| v.as_object())
    {
        for (command_name, command_data) in command_map {
            generate_command_file(
                command_name,
                command_data,
                output_path,
                &mut index_content,
                program_name,
                "",
            );
        }
    }

    // Write index file
    let index_path = output_path.join("index.ts");
    fs::write(&index_path, index_content).expect("Failed to write index file");
}

fn generate_command_file(
    command_name: &str,
    command_data: &serde_json::Value,
    base_path: &PathBuf,
    index_content: &mut String,
    _program_name: &str,
    parent_path: &str,
) {
    let safe_command_name = sanitize_filename(command_name);
    let file_path = if parent_path.is_empty() {
        base_path.join(format!("{}.ts", safe_command_name))
    } else {
        let dir_path = base_path.join(parent_path);
        fs::create_dir_all(&dir_path).expect("Failed to create subdirectory");
        dir_path.join(format!("{}.ts", safe_command_name))
    };

    let mut content = String::new();

    // Import schema
    let import_path = if parent_path.is_empty() {
        "./schema"
    } else {
        "../schema"
    };
    content.push_str(&format!(
        "import type {{ Command, CommandFlag }} from '{}';\n",
        import_path
    ));
    
    // Check if we need CommandComponentDataType (used by flags and arguments)
    let needs_data_type = if let Some(children) = command_data.get("children").and_then(|v| v.as_object()) {
        // Check for flags
        let has_flags = children.get("FLAG").and_then(|v| v.as_array()).map_or(false, |flags| !flags.is_empty());
        
        // Check for arguments 
        let has_arguments = has_usage_arguments(children) || children.get("ARGUMENT").and_then(|v| v.as_array()).map_or(false, |args| !args.is_empty());
        
        has_flags || has_arguments
    } else {
        false
    };
    
    if needs_data_type {
        content.push_str(&format!(
            "import {{ CommandComponentDataType }} from '{}';\n",
            import_path
        ));
    }
    
    // Check if we need NamingConventions (only used by arguments)
    let needs_naming = if let Some(children) = command_data.get("children").and_then(|v| v.as_object()) {
        has_usage_arguments(children) || children.get("ARGUMENT").and_then(|v| v.as_array()).map_or(false, |args| !args.is_empty())
    } else {
        false
    };
    
    if needs_naming {
        let naming_import_path = if parent_path.is_empty() {
            "./naming-convention"
        } else {
            "../naming-convention"
        };
        content.push_str(&format!("import {{ NamingConventions }} from '{}';\n", naming_import_path));
    }

    // Collect subcommand imports
    let mut subcommand_imports = Vec::new();
    if let Some(children) = command_data.get("children").and_then(|v| v.as_object())
        && let Some(subcommands) = children.get("COMMAND").and_then(|v| v.as_object())
    {
        for subcommand_name in subcommands.keys() {
            let subcommand_interface = format!("{}Command", to_pascal_case(subcommand_name));
            let import_path = if parent_path.is_empty() {
                format!(
                    "./{}/{}",
                    safe_command_name,
                    sanitize_filename(subcommand_name)
                )
            } else {
                format!(
                    "../{}/{}",
                    safe_command_name,
                    sanitize_filename(subcommand_name)
                )
            };
            subcommand_imports.push((subcommand_interface, import_path));
        }
    }

    // Add subcommand imports
    for (interface_name, import_path) in &subcommand_imports {
        content.push_str(&format!(
            "import {{ {} }} from '{}';\n",
            interface_name, import_path
        ));
    }
    content.push('\n');

    // First, generate the flags constant if there are flags
    if let Some(children) = command_data.get("children").and_then(|v| v.as_object())
        && let Some(flags) = children.get("FLAG").and_then(|v| v.as_array())
        && !flags.is_empty()
    {
        let flag_constant_content = generate_flags_constant(children, &safe_command_name);
        content.push_str(&flag_constant_content);
        content.push('\n');
    }

    // Generate command object (not interface)
    let command_name_pascal = format!("{}Command", to_pascal_case(&safe_command_name));
    content.push_str(&format!("export const {}: Command = {{\n", command_name_pascal));
    content.push_str(&format!("  name: '{}',\n", command_name));

    // Add description if available
    if let Some(description) = command_data.get("description").and_then(|v| v.as_str()) {
        content.push_str(&format!(
            "  description: '{}',\n",
            escape_string(description)
        ));
    }

    // Extract usage from USAGE array
    if let Some(children) = command_data.get("children").and_then(|v| v.as_object())
        && let Some(usage_array) = children.get("USAGE").and_then(|v| v.as_array())
        && let Some(first_usage) = usage_array.first()
        && let Some(usage_string) = first_usage.get("usage_string").and_then(|v| v.as_str())
    {
        content.push_str(&format!("  usage: '{}',\n", escape_string(usage_string)));
    }

    // Add arguments if available (from usage components)
    if let Some(children) = command_data.get("children").and_then(|v| v.as_object()) {
        let mut arguments = Vec::new();
        
        // Extract arguments from USAGE array
        if let Some(usage_array) = children.get("USAGE").and_then(|v| v.as_array()) {
            for usage in usage_array {
                if let Some(usage_components) = usage.get("usage_components").and_then(|v| v.as_array()) {
                    for component in usage_components {
                        if let Some(component_type) = component.get("component_type").and_then(|v| v.as_str())
                            && let Some(name) = component.get("name").and_then(|v| v.as_str())
                            && component_type == "Keyword"
                            && name.chars().all(|c| c.is_uppercase() || c == '_')
                            && name != "FLAGS" // Exclude FLAGS keyword
                        {
                            let is_required = component.get("required").and_then(|v| v.as_bool()).unwrap_or(true);
                            arguments.push((name.to_string(), is_required));
                        }
                    }
                }
            }
        }
        
        // Also check legacy ARGUMENT array for backwards compatibility
        if let Some(args) = children.get("ARGUMENT").and_then(|v| v.as_array()) {
            for (i, arg) in args.iter().enumerate() {
                if let Some(arg_str) = arg.as_str() {
                    arguments.push((arg_str.to_string(), true));
                } else {
                    arguments.push((format!("arg{}", i), true));
                }
            }
        }
        
        if !arguments.is_empty() {
            content.push_str("  arguments: {\n");
            for (arg_name, is_required) in arguments {
                let arg_variable_name = sanitize_js_variable_name(&arg_name.to_lowercase());
                content.push_str(&format!("    {}: {{\n", arg_variable_name));
                content.push_str(&format!("      description: 'The {} to use.',\n", arg_name.to_lowercase()));
                content.push_str(&format!("      required: {},\n", is_required));
                content.push_str("      valueDataType: CommandComponentDataType.STRING,\n");
                content.push_str("      formats: [\n");
                content.push_str("        {\n");
                content.push_str("          namingConvention: NamingConventions.ResourceName(),\n");
                content.push_str("          examples: [],\n");
                content.push_str("        }\n");
                content.push_str("      ]\n");
                content.push_str("    },\n");
            }
            content.push_str("  },\n");
        }

        // Add flags reference
        if let Some(flags) = children.get("FLAG").and_then(|v| v.as_array())
            && !flags.is_empty()
        {
            let const_name = format!(
                "{}_FLAGS",
                sanitize_js_variable_name(&safe_command_name).to_uppercase()
            );
            content.push_str(&format!("  flags: {},\n", const_name));
        }

        // Process subcommands
        if let Some(subcommands) = children.get("COMMAND").and_then(|v| v.as_object())
            && !subcommands.is_empty()
        {
            content.push_str("  subcommands: {\n");
            for subcommand_name in subcommands.keys() {
                content.push_str(&format!(
                    "    '{}': {}Command;\n",
                    subcommand_name,
                    to_pascal_case(subcommand_name)
                ));
            }
            content.push_str("  };\n");

            // Generate subcommand files
            let subdir_path = if parent_path.is_empty() {
                safe_command_name.clone()
            } else {
                format!("{}/{}", parent_path, safe_command_name)
            };

            for (subcommand_name, subcommand_data) in subcommands {
                generate_command_file(
                    subcommand_name,
                    subcommand_data,
                    base_path,
                    index_content,
                    _program_name,
                    &subdir_path,
                );
            }
        }
    }

    content.push_str("};\n\n");

    // Write the file
    fs::write(&file_path, content).expect("Failed to write command file");

    // Add export to index
    let export_path = if parent_path.is_empty() {
        format!("./{}", safe_command_name)
    } else {
        format!("./{}/{}", parent_path, safe_command_name)
    };
    index_content.push_str(&format!("export * from '{}';\n", export_path));
}

fn has_usage_arguments(children: &serde_json::Map<String, serde_json::Value>) -> bool {
    if let Some(usage_array) = children.get("USAGE").and_then(|v| v.as_array()) {
        for usage in usage_array {
            if let Some(usage_components) = usage.get("usage_components").and_then(|v| v.as_array()) {
                for component in usage_components {
                    if let Some(component_type) = component.get("component_type").and_then(|v| v.as_str())
                        && let Some(name) = component.get("name").and_then(|v| v.as_str())
                        && component_type == "Keyword"
                        && name.chars().all(|c| c.is_uppercase() || c == '_')
                        && name != "FLAGS" // Exclude FLAGS keyword
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn generate_flags_constant(children: &serde_json::Map<String, serde_json::Value>, safe_command_name: &str) -> String {
    let mut content = String::new();
    
    if let Some(flags) = children.get("FLAG").and_then(|v| v.as_array())
        && !flags.is_empty()
    {
        // Get usage string for docopt analysis
        let usage_string =
            if let Some(usage_array) = children.get("USAGE").and_then(|v| v.as_array()) {
                usage_array
                    .first()
                    .and_then(|u| u.get("usage_string"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
            } else {
                ""
            };
        // Collect and sort flags alphabetically by long flag name
        let mut flag_objects = Vec::new();
        for flag in flags {
            if let Some(flag_obj) = flag.as_object()
                && let Some(long_flag) = flag_obj.get("long").and_then(|v| v.as_str())
            {
                let short_flag = flag_obj.get("short").and_then(|v| v.as_str()).unwrap_or("");
                let description = flag_obj
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let data_type = flag_obj
                    .get("data_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // Extract and clean description by removing data type prefix
                let (clean_description, extracted_data_type) =
                    extract_data_type_from_description(description);

                // Determine data type based on data_type field, extracted type, or patterns
                let data_type_enum = if !data_type.is_empty() {
                    match data_type {
                        "stringArray" => {
                            // Check if it's actually key-value mapping based on description
                            if is_key_value_mapping(&clean_description) {
                                "CommandComponentDataType.KEY_VALUE_MAPPING"
                            } else {
                                "[CommandComponentDataType.STRING]"
                            }
                        }
                        "stringToString" => "CommandComponentDataType.KEY_VALUE_MAPPING",
                        "uint" | "int" => "CommandComponentDataType.INTEGER",
                        "bool" => "CommandComponentDataType.BOOLEAN",
                        "string" => "CommandComponentDataType.STRING",
                        "float" => "CommandComponentDataType.FLOAT",
                        _ => "CommandComponentDataType.STRING",
                    }
                } else if !extracted_data_type.is_empty() {
                    match extracted_data_type.as_str() {
                        "stringArray" => {
                            // Check if it's actually key-value mapping based on description
                            if is_key_value_mapping(&clean_description) {
                                "CommandComponentDataType.KEY_VALUE_MAPPING"
                            } else {
                                "[CommandComponentDataType.STRING]"
                            }
                        }
                        "stringToString" => "CommandComponentDataType.KEY_VALUE_MAPPING",
                        "uint" | "int" => "CommandComponentDataType.INTEGER",
                        "string" => "CommandComponentDataType.STRING",
                        "bool" => "CommandComponentDataType.BOOLEAN",
                        "float" => "CommandComponentDataType.FLOAT",
                        _ => "CommandComponentDataType.STRING",
                    }
                } else if long_flag == "--help"
                    || clean_description.starts_with("help for")
                    || long_flag.starts_with("--no-")
                {
                    "CommandComponentDataType.BOOLEAN"
                } else {
                    // Default fallback
                    "CommandComponentDataType.STRING"
                };

                // Extract default value from description
                let default_value = extract_default_value(&clean_description);

                // Extract examples (if any) from description
                let examples = extract_examples(&clean_description);

                // Determine if flag is required based on description patterns and usage string
                let is_required = if description.contains("(default")
                    || description.to_lowercase().contains("default is")
                    || description.to_lowercase().contains("defaults to")
                    || long_flag == "--help"
                    || description.starts_with("help for")
                {
                    // Flags with default values or help flags are optional
                    false
                } else if description.to_lowercase().contains("required")
                    || description.to_lowercase().contains("mandatory")
                    || description.to_lowercase().contains("must be provided")
                    || description.to_lowercase().contains("must specify")
                    || description.ends_with("(required)")
                    || description.ends_with("(mandatory)")
                {
                    // Explicitly marked as required
                    true
                } else {
                    // Check docopt patterns in usage string
                    check_flag_in_usage_string(usage_string, long_flag, short_flag)
                };

                flag_objects.push((
                    long_flag.to_string(),
                    short_flag.to_string(),
                    data_type_enum.to_string(),
                    clean_description,
                    default_value,
                    examples,
                    is_required,
                ));
            }
        }

        // Sort flags alphabetically by long flag name (without --)
        flag_objects.sort_by(|a, b| {
            let a_name = a.0.trim_start_matches('-');
            let b_name = b.0.trim_start_matches('-');
            a_name.cmp(b_name)
        });

        let const_name = format!(
            "{}_FLAGS",
            sanitize_js_variable_name(&safe_command_name).to_uppercase()
        );
        content.push_str(&format!("export const {}: CommandFlag[] = [\n", const_name));
        for (
            long_flag,
            short_flag,
            data_type_enum,
            clean_description,
            default_value,
            examples,
            is_required,
        ) in &flag_objects
        {
            content.push_str("  {\n");

            // Long name
            content.push_str(&format!("    longName: '{}',\n", long_flag));

            // Short name (optional)
            if !short_flag.is_empty() {
                content.push_str(&format!("    shortName: '{}',\n", short_flag));
            }

            // Value data type
            content.push_str(&format!("    valueDataType: {},\n", data_type_enum));

            // Default value (optional)
            if let Some(default_val) = default_value {
                content.push_str(&format!(
                    "    defaultValue: '{}',\n",
                    escape_string(default_val)
                ));
            }

            // Description
            content.push_str(&format!(
                "    description: '{}',\n",
                escape_string(clean_description)
            ));

            // Required flag
            content.push_str(&format!("    required: {},\n", is_required));

            // Examples (always include, empty array if none)
            content.push_str("    examples: [");
            if !examples.is_empty() {
                content.push_str("\n");
                for example in examples {
                    content.push_str(&format!("      '{}',\n", escape_string(example)));
                }
                content.push_str("    ],\n");
            } else {
                content.push_str("],\n");
            }

            // Naming convention (add basic naming convention)
            content.push_str("    namingConvention: 'String'\n");

            content.push_str("  },\n");
        }
        content.push_str("];\n\n");
    }

    content
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn sanitize_js_variable_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn to_pascal_case(s: &str) -> String {
    s.split(&['-', '_', ' '][..])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect()
}

fn escape_string(s: &str) -> String {
    s.replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn extract_data_type_from_description(description: &str) -> (String, String) {
    // Extract data type prefix from description like "uint Number of servers..."
    let data_type_prefixes = [
        "stringArray",
        "stringToString",
        "uint",
        "int",
        "string",
        "bool",
        "float",
    ];

    for prefix in &data_type_prefixes {
        if description.starts_with(&format!("{} ", prefix)) {
            let clean_desc = description
                .strip_prefix(&format!("{} ", prefix))
                .unwrap_or(description);
            return (clean_desc.to_string(), prefix.to_string());
        }
    }

    (description.to_string(), String::new())
}

fn extract_default_value(description: &str) -> Option<String> {
    // Extract default value from patterns like "(default 1)" or "default is nvidia"
    if let Some(start) = description.find("(default ")
        && let Some(end) = description[start..].find(')')
    {
        let default_part = &description[start + 9..start + end]; // Skip "(default "
        return Some(default_part.to_string());
    }

    if let Some(start) = description.find("default is ") {
        let default_part = description[start + 11..]
            .split_whitespace()
            .next()
            .unwrap_or("");
        if !default_part.is_empty() {
            return Some(default_part.to_string());
        }
    }

    None
}

fn extract_examples(_description: &str) -> Vec<String> {
    // Extract examples from description - this could be enhanced based on patterns found
    // For now, return empty as most descriptions don't have explicit examples
    Vec::new()
}

fn is_key_value_mapping(description: &str) -> bool {
    // Check if description indicates key-value mapping patterns
    description.contains("key=value")
        || description.contains("Key/Value")
        || description.contains("key/value")
        || description.contains("=")
            && (description.contains("pair") || description.contains("mapping"))
        || description.contains("key:value")
}

fn check_flag_in_usage_string(usage_string: &str, long_flag: &str, short_flag: &str) -> bool {
    // Check docopt patterns in usage string
    // In docopt:
    // <argument> = required
    // [optional] = optional
    // (required) = required (grouped)
    // --flag = appears directly (context-dependent)
    // [--flag] = explicitly optional
    // <--flag> = explicitly required (rare but possible)

    // Look for the flag in various docopt patterns
    let long_without_dashes = long_flag.trim_start_matches('-');

    // Check if flag appears in required context: <--flag> or (--flag)
    if usage_string.contains(&format!("<{}>", long_flag))
        || usage_string.contains(&format!("({})", long_flag))
        || (!short_flag.is_empty() && usage_string.contains(&format!("<{}>", short_flag)))
        || (!short_flag.is_empty() && usage_string.contains(&format!("({})", short_flag)))
    {
        return true;
    }

    // Check if flag appears in optional context: [--flag]
    if usage_string.contains(&format!("[{}]", long_flag))
        || usage_string.contains(&format!("[{}]", short_flag))
        || usage_string.contains("[flags]")
    {
        return false;
    }

    // Check for uppercase placeholder patterns that indicate required flags
    // e.g., "my_cli checkpoint export STUFF_NAME --storage-provider PROVIDER_NAME --destination-path PATH"
    // Look for UPPERCASE words that correspond to flag names
    if usage_string.contains(&format!(
        "{} {}",
        long_flag,
        long_without_dashes.to_uppercase()
    )) || usage_string.contains(&format!(
        "{} {}",
        long_flag,
        &long_without_dashes.replace('-', "_").to_uppercase()
    )) || usage_string.contains(&format!("{} PATH", long_flag))
        || usage_string.contains(&format!("{} NAME", long_flag))
        || usage_string.contains(&format!("{} ID", long_flag))
    {
        return true;
    }

    // Default to optional for ambiguous cases
    false
}

/// Compare two parsed CLI structures and display differences
pub fn run_cli_compare(
    program_name: &str,
    from_tag: Option<&String>,
    to_tag: Option<&String>,
    format: Option<&String>,
) {
    use crate::models::ParseOutputFormat;

    // Determine format for comparison
    let compare_format = match format {
        Some(fmt) => ParseOutputFormat::from_str(fmt).unwrap_or_else(|| {
            println!("Warning: Unknown format '{}', defaulting to JSON", fmt);
            ParseOutputFormat::Json
        }),
        None => ParseOutputFormat::Json,
    };

    // Get available versions/tags for the program
    let base_dir = PathBuf::from("./out").join(program_name);

    if !base_dir.exists() {
        println!("Error: No parsed data found for program '{}'", program_name);
        println!(
            "Run 'clint parse {}' first to generate parsed data.",
            program_name
        );
        return;
    }

    // List all available versions/tags
    let mut available_versions = Vec::new();
    if let Ok(entries) = fs::read_dir(&base_dir) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|ft| ft.is_dir())
                && let Some(name) = entry.file_name().to_str()
            {
                available_versions.push(name.to_string());
            }
        }
    }

    if available_versions.is_empty() {
        println!("Error: No versions found for program '{}'", program_name);
        return;
    }

    // Sort versions (latest first)
    available_versions.sort();
    available_versions.reverse();

    // Determine which versions to compare
    let from_version = from_tag
        .cloned()
        .or_else(|| available_versions.first().cloned())
        .unwrap_or_else(|| {
            println!("Error: No versions available for comparison");
            std::process::exit(1);
        });

    let to_version = to_tag
        .cloned()
        .or_else(|| available_versions.get(1).cloned())
        .unwrap_or_else(|| {
            println!("Error: Need at least two versions for comparison");
            println!("Available versions: {:?}", available_versions);
            std::process::exit(1);
        });

    println!(
        "Comparing {} versions: {} -> {}",
        program_name, from_version, to_version
    );
    println!();

    // Build file paths
    let from_path = match compare_format {
        ParseOutputFormat::TypeScriptDirectory => base_dir.join(&from_version).join(program_name),
        _ => base_dir
            .join(&from_version)
            .join(format!("parsed.{}", compare_format.get_file_extension())),
    };

    let to_path = match compare_format {
        ParseOutputFormat::TypeScriptDirectory => base_dir.join(&to_version).join(program_name),
        _ => base_dir
            .join(&to_version)
            .join(format!("parsed.{}", compare_format.get_file_extension())),
    };

    // Check if files exist
    if !from_path.exists() {
        println!(
            "Error: Source version '{}' not found at: {}",
            from_version,
            from_path.display()
        );
        return;
    }

    if !to_path.exists() {
        println!(
            "Error: Target version '{}' not found at: {}",
            to_version,
            to_path.display()
        );
        return;
    }

    match compare_format {
        ParseOutputFormat::TypeScriptDirectory => {
            compare_typescript_directories(&from_path, &to_path, &from_version, &to_version);
        }
        _ => {
            compare_json_files(&from_path, &to_path, &from_version, &to_version);
        }
    }
}

/// Compare two JSON files and display differences
fn compare_json_files(
    from_path: &PathBuf,
    to_path: &PathBuf,
    from_version: &str,
    to_version: &str,
) {
    match comparison::compare_json_structures(from_path, to_path) {
        Ok(changes) => {
            if changes.is_empty() {
                println!(
                    "No differences found between {} and {}",
                    from_version, to_version
                );
            } else {
                println!("Changes found between {} and {}:", from_version, to_version);
                println!();

                for change in &changes {
                    println!("{}", change.format());
                }

                println!();
                println!("Summary: {} changes detected", changes.len());

                println!();
                println!("Tip: Use a JSON diff tool for raw comparison:");
                println!(
                    "  diff <(jq . {}) <(jq . {})",
                    from_path.display(),
                    to_path.display()
                );
            }
        }
        Err(e) => {
            println!("Error comparing JSON structures: {}", e);
            println!("Falling back to simple file comparison...");
            println!();

            // Fallback to simple file comparison
            let from_content = fs::read_to_string(from_path).unwrap_or_default();
            let to_content = fs::read_to_string(to_path).unwrap_or_default();

            if from_content == to_content {
                println!(
                    "No differences found between {} and {}",
                    from_version, to_version
                );
            } else {
                println!("Files differ between {} and {}", from_version, to_version);
                println!();
                println!("Tip: Use diff or a JSON tool for detailed comparison:");
                println!("  diff {} {}", from_path.display(), to_path.display());
            }
        }
    }
}

/// Compare two TypeScript directories and display differences
fn compare_typescript_directories(from_path: &Path, to_path: &Path, from_version: &str, to_version: &str) {
    println!("Analyzing CLI structure changes...");
    println!();

    match comparison::compare_typescript_directories(from_path, to_path) {
        Ok(changes) => {
            if changes.is_empty() {
                println!(
                    "No differences found between {} and {}",
                    from_version, to_version
                );
            } else {
                println!("Changes found between {} and {}:", from_version, to_version);
                println!();

                for change in &changes {
                    println!("{}", change.format());
                }

                println!();
                println!("Summary: {} changes detected", changes.len());

                println!();
                println!("Tip: Use git diff for file-level comparison:");
                println!("  diff -r {} {}", from_path.display(), to_path.display());
            }
        }
        Err(e) => {
            println!("Error comparing TypeScript directories: {}", e);
        }
    }
}
