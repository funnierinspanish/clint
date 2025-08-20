use crate::models::FileOutputFormat;
use cli_parser::extract_cli_structure;
use keyword_extractor::extract_keywords_from_json;
use serde_json::json;
use std::path::PathBuf;
use std::env;
use std::fs;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};
use dialoguer::{Select, Confirm};
use warp::Filter;

use crate::cli_parser;
use crate::keyword_extractor;
use crate::models::OutputFile;
use crate::replicator;
use crate::summary_generator::generate_summary;

fn get_config_dir_path(program_name: &str, program_version: &str) -> PathBuf {
    let home_dir = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .expect("Could not find home directory");
    
    let config_dir = PathBuf::from(home_dir)
        .join(".config")
        .join("clint")
        .join("parsed")
        .join(program_name);
    
    fs::create_dir_all(&config_dir)
        .expect("Failed to create config directory");
    
    let version_suffix = if program_version.is_empty() || program_version == "Unknown" {
        "unknown".to_string()
    } else {
        program_version
            .replace('/', "_")
            .replace('\\', "_")
            .replace(':', "_")
            .replace('<', "_")
            .replace('>', "_")
            .replace('"', "_")
            .replace('|', "_")
            .replace('?', "_")
            .replace('*', "_")
            .replace(' ', "_")
    };
    
    config_dir.join(format!("{}-{}.json", program_name, version_suffix))
}

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
    fs::create_dir_all(&templates_dir)
        .expect("Failed to create templates directory");
    
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
    
    fs::create_dir_all(&default_template_dir)
        .expect("Failed to create default template directory");
    
    println!("Getting web interface files to: {}", default_template_dir.display());
    
    match download_template_from_github(&default_template_dir) {
        Ok(()) => {
            println!("\nWeb interface template download complete!");
            println!("Files saved to: {}", default_template_dir.display());
            println!("Tip: These files can be customized. The serve command will use your custom template when available.");
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
            file_path.exists() && fs::metadata(&file_path).map_or(false, |meta| meta.len() > 0)
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
    println!("");
    
    // Check if we're in an interactive terminal
    let is_interactive = atty::is(atty::Stream::Stdin);
    
    let should_download = if is_interactive {
        match Confirm::new()
            .with_prompt("Would you like to download the default template from GitHub?")
            .default(true)
            .interact() {
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
            .args(&["-fsSL", &url, "-o", target_path.to_str().unwrap()])
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        
        if !download_success {
            // Try wget as fallback
            let wget_success = Command::new("wget")
                .args(&["-q", &url, "-O", target_path.to_str().unwrap()])
                .status()
                .map(|status| status.success())
                .unwrap_or(false);
            
            if !wget_success {
                return Err(format!("Failed to download {} (tried curl and wget)", filename).into());
            }
        }
        
        // Verify the file was downloaded and is not empty
        if !target_path.exists() || fs::metadata(&target_path)?.len() == 0 {
            return Err(format!("Downloaded file {} is empty or missing", filename).into());
        }
    }
    
    Ok(())
}

fn show_manual_template_download_instructions(target_dir: &PathBuf) {
    println!("");
    println!("Manual template download instructions:");
    println!("1. Download the template files from:");
    println!("   https://github.com/funnierinspanish/clint/tree/main/src/web");
    println!("2. Save them to this directory:");
    println!("   {}", target_dir.display());
    println!("3. Required files:");
    println!("   - index.html");
    println!("   - script.js");
    println!("   - cli-command-card.js");
    println!("");
    println!("Alternatively, you can run 'clint get-template' to download the default template.");
    println!("");
}

pub fn run_cli_parser(command: &str, output_path: Option<&PathBuf>) {
    let structure: serde_json::Value = extract_cli_structure(command, None);
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
    
    let out_path = match output_path {
        Some(path) => {
            println!("Using custom output path: {:?}", path);
            path.clone()
        },
        None => {
            let default_path = get_config_dir_path(program_name, program_version);
            println!("Using default config directory: {:?}", default_path);
            default_path
        }
    };
    
    let out_file: OutputFile = OutputFile::new(&out_path, FileOutputFormat::Json);

    out_file.write_json_output_file(structure);

    println!("CLI structure JSON file saved successfully!");
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
    }
}

pub fn run_interactive_serve(template: Option<&String>, port: Option<u16>, input_file: Option<&PathBuf>) {
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
    
    if !input_path.extension().map_or(false, |ext| ext == "json") {
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
    
    let (template_path, template_source) = if custom_template_path.exists() && template_name != "default" {
        // Use custom template
        (custom_template_path, format!("custom template: {}", template_name))
    } else if template_name == "default" {
        // For default template, check and offer to download if needed
        match check_and_offer_template_download() {
            Some(default_path) => (default_path, "downloaded template".to_string()),
            None => {
                println!("Cannot serve without web templates. Please:");
                println!("1. Run 'clint get-template' to download templates");
                println!("2. Or manually download files from GitHub to ~/.config/clint/templates/default/");
                return;
            }
        }
    } else {
        // Requested template doesn't exist
        println!("Template '{}' not found: {}", template_name, custom_template_path.display());
        println!("Available templates:");
        let templates_dir = custom_template_path.parent().unwrap();
        if let Ok(entries) = fs::read_dir(templates_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                    if let Some(name) = entry.file_name().to_str() {
                        println!("  - {}", name);
                    }
                }
            }
        } else {
            println!("  (no templates directory found)");
        }
        println!("Cannot serve without templates.");
        return;
    };
    
    // Extract app name and version from file path/name for display
    let file_name = input_path.file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let version = extract_version_from_filename(file_name);
    let app_name = if let Some(dash_pos) = file_name.rfind('-') {
        &file_name[..dash_pos]
    } else {
        file_name
    };
    
    println!("Starting HTTP server for {} version {}...", app_name, version);
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
            if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                let app_dir = entry.path();
                if let Some(app_name) = app_dir.file_name().and_then(|n| n.to_str()) {
                    // Check if directory contains JSON files
                    if let Ok(json_files) = fs::read_dir(&app_dir) {
                        let json_count = json_files
                            .flatten()
                            .filter(|file| {
                                file.path()
                                    .extension()
                                    .map_or(false, |ext| ext == "json")
                            })
                            .filter(|file| {
                                // Check if file is non-empty
                                file.metadata().map_or(false, |meta| meta.len() > 0)
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
        .map(|(name, _, count)| format!("{} ({} version{})", name, count, if *count == 1 { "" } else { "s" }))
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
            if entry.path().extension().map_or(false, |ext| ext == "json") {
                if let Some(filename) = entry.file_name().to_str() {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.len() > 0 {
                            json_files.push((filename.to_string(), entry.path(), metadata));
                        }
                    }
                }
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
            },
            (Some(_), None) => std::cmp::Ordering::Less, // Semver comes first
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => {
                // Fall back to creation date (descending)
                b.2.created().unwrap_or(SystemTime::UNIX_EPOCH)
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
            
            if parse_semver(&version).is_some() {
                format!("{} ({})", version, created)
            } else {
                format!("{} ({})", version, created)
            }
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
            println!("2. Or manually download files from GitHub to ~/.config/clint/templates/default/");
            return;
        }
    };
    let template_source = "downloaded template";
    
    // Start HTTP server with selected JSON data
    println!("Starting HTTP server for {} version {}...", selected_app, selected_version);
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
        None => "unknown path".to_string()
    };
    
    // Create filter for serving the CLI structure JSON
    let json_content_filter = warp::any().map(move || json_content.clone());
    let cli_structure = warp::path("cli-structure.json")
        .and(warp::get())
        .and(json_content_filter)
        .map(move |content: String| {
            println!("Redirecting client request:\n  cli-structure.json --> {}\n", json_to_serve_path);
            warp::reply::with_header(content, "content-type", "application/json")
        });

    // Create routes using filesystem templates
    let static_files = warp::fs::dir(template_path.clone())
        .with(warp::log("template_files"));

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
    println!("Open your browser and navigate to: http://localhost:{}", server_port);
    println!("Serving: {} version {}", app_name, version);
    if using_custom_template {
        println!("Using custom template: {}", template_path.display());
    } else {
        println!("Using default template");
    }
    println!("Press Ctrl+C to stop the server");
    println!("");
    
    // Start the server
    warp::serve(routes)
        .run(([127, 0, 0, 1], server_port))
        .await;
}

fn find_available_port(start_port: u16) -> Option<u16> {
    use std::net::TcpListener;
    use std::collections::HashSet;
    
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
    if parts.len() >= 3 {
        if let (Ok(major), Ok(minor), Ok(patch)) = (
            parts[0].parse::<u32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            return Some((major, minor, patch));
        }
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
