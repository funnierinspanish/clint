use clap::Parser;
use serde::Deserialize;
use std::{collections::HashMap, fs, path::PathBuf, process::Command as ShellCommand, env};

/// CLI generator arguments
#[derive(Parser)]
#[command(author, version, about = "Generate a Rust CLI project using clap derive from a JSON spec")]
struct Args {
    /// Path to the CLI structure JSON file
    #[arg(short, long)]
    input: PathBuf,
    /// Directory to create the new Rust project in
    #[arg(short, long)]
    output: PathBuf,
    /// Keep the original clap-generated help flags/subcommand
    #[arg(long, default_value_t = false)]
    keep_help_flags: bool,
    /// Keep the original clap-generated verbose flags
    #[arg(long, default_value_t = false)]
    keep_verbose_flags: bool,
}

/// JSON schema definitions
#[derive(Deserialize)]
struct CliSpec {
    name: String,
    description: String,
    version: String,
    children: ChildrenSpec,
}

#[derive(Deserialize)]
struct ChildrenSpec {
    COMMAND: HashMap<String, CommandSpec>,
    FLAG: Vec<FlagSpec>,
    USAGE: Vec<UsageSpec>,
    OTHER: Vec<OtherSpec>,
}

#[derive(Deserialize)]
struct CommandSpec {
    name: String,
    description: String,
    parent: String,
    parent_header: Option<String>,
    children: ChildrenSpec,
}

#[derive(Deserialize)]
struct FlagSpec {
    short: Option<String>,
    long: Option<String>,
    data_type: Option<String>,
    description: Option<String>,
    parent_header: String,
    required: Option<bool>,
}

#[derive(Deserialize)]
struct UsageSpec { usage_string: String, parent_header: String }
#[derive(Deserialize)]
struct OtherSpec { line_contents: String, parent_header: String }

pub fn replicate(input_json: &PathBuf, output_path: &PathBuf, keep_help_flags: bool, keep_verbose_flags: bool) -> Result<(), Box<dyn std::error::Error>> {
    let json = fs::read_to_string(input_json).expect("Failed to read CLI Structure JSON file");
    let spec: CliSpec = serde_json::from_str(&json).expect("Failed to parse CLI Structure JSON file. Make sure it is valid JSON.");
    let output_dir: PathBuf = PathBuf::from(output_path);
    // Scaffold new Rust project
    fs::create_dir_all(&output_dir).expect("Failed to create or find the output directory. Make sure it is a writable path.");
    ShellCommand::new("cargo")
        .args(["init", "--bin", "--name", &spec.name, output_dir.to_str().expect("Failed to convert output directory path to string")])
        .status().expect("Failed to create new Rust project. Make sure you have cargo installed and available in your PATH.");

    // Generate code files
    let cli_code = generate_cli_builder(&spec, keep_help_flags, keep_verbose_flags);
    let main_code = generate_main_builder(&spec, keep_help_flags, keep_verbose_flags);
    let output_src_dir = output_dir.join("src");
    fs::write(output_src_dir.join("cli.rs"), cli_code).expect("Failed to write cli.rs");
    fs::write(output_src_dir.join("main.rs"), main_code).expect("Failed to write main.rs");

    // Generate command handler files
    generate_command_handler_files(&output_src_dir, &spec).expect("Failed to generate command handler files");

    // Change into project dir & add dependencies
    env::set_current_dir(&output_dir).expect("Failed to change into the project directory");
    ShellCommand::new("cargo")
        .args(["add", "clap"])
        .status()
        .expect("Failed to add `clap` as a dependency with cargo. Make sure you have cargo installed and available in your PATH.");
    match ShellCommand::new("cargo")
        .args(["build", "--release"])
        .status() {
        Ok(status) => {
            
            match status.code() {
                Some(0) => {
                    let binary_path = output_dir.join("target").join("release").join(&spec.name);
                    println!("\nBuild successful! Your replica of the {} CLI app can be found at:\n", spec.name);
                    println!("  \x1b[93m{}\x1b[0m", binary_path.display());
                }
                Some(code) => println!("Build failed with code: {}", code),
                None => println!("Build process terminated by signal"),
            }
        },
        Err(e) => println!("Failed to build the {} CLI app replica: {}", spec.name, e),
    }
    Ok(())
}

fn generate_command_handler_files(src_dir: &PathBuf, spec: &CliSpec) -> Result<(), Box<dyn std::error::Error>> {
    // Create handlers for each command in commands/ directory
    let cmd_dir = src_dir.join("commands");
    fs::create_dir_all(&cmd_dir).expect("Failed to create commands directory");
    for cmd_spec in spec.children.COMMAND.values() {
        let mut file = String::new();
        file.push_str("use clap::ArgMatches;\n\n");
        // main handler
        file.push_str(&format!(
            "pub fn handle_{cmd}(matches: &ArgMatches, print_help: impl Fn()) {{",
cmd = cmd_spec.name
));
        file.push_str("    if matches.args_present() {\n");
        file.push_str(&format!(
            "        println!(\"Called {cmd} with args: {{:?}}\", matches);\n",
cmd = cmd_spec.name
));
        file.push_str("    } else {\n        print_help();\n    }\n}\n\n");
        // subcommands
        for sub in cmd_spec.children.COMMAND.values() {
            file.push_str(&format!(
                "pub fn handle_{cmd}_{sub}(matches: &ArgMatches, print_help: impl Fn()) {{",
cmd = cmd_spec.name,
sub = sub.name
));
            file.push_str("    if matches.args_present() {\n");
            file.push_str(&format!(
                "        println!(\"Called {cmd} {sub} with args: {{:?}}\", matches);\n",
cmd = cmd_spec.name,
sub = sub.name
));
            file.push_str("    } else {\n        print_help();\n    }\n}\n\n");
        }
        fs::write(cmd_dir.join(format!("{}.rs", cmd_spec.name)), file)
            .expect("Failed to write command handler file");
    }
    Ok(())
}

/// Build `cli.rs` using clap's builder API
fn generate_cli_builder(spec: &CliSpec, keep_help: bool, keep_verbose: bool) -> String {
    let mut cli_file_contents_string = String::new();
    cli_file_contents_string.push_str("use clap::{Command, Arg, ArgAction};\n\n");
    cli_file_contents_string.push_str("pub fn build_cli() -> Command {\n");

    // Root command
    cli_file_contents_string.push_str(&format!(
        "    let mut cmd = Command::new(\"{}\").version(\"{}\").about(\"{}\");\n",
        spec.name,
        spec.version,
        spec.description.replace('"', "\\\"")
    ));

    // Optionally disable auto-help
    if !keep_help {
        cli_file_contents_string.push_str("    cmd = cmd.disable_help_subcommand(true);\n");
    }
    cli_file_contents_string.push_str("\n    // Global flags\n");

    // Global flags
    for flag in &spec.children.FLAG {
        let name = flag.long.as_deref().unwrap_or_else(|| flag.short.as_deref().expect("Flag must have either short or long name"));
        let key = name.trim_start_matches('-');
        if (!keep_help && key == "help") || (!keep_verbose && key == "verbose") {
            continue;
        }
        let short_call = flag.short.as_deref().map_or(String::new(), |s| format!(".short('{}')", s.trim_start_matches('-')));
        let long_call = flag.long.as_deref().map_or(String::new(), |l| format!(".long(\"{}\")", l.trim_start_matches('-')));
        let help = flag.description.as_deref().unwrap_or_default().replace('"', "\\\"");
        let required = flag.required.unwrap_or(false);
        let action = match flag.data_type.as_deref() {
            Some("string") | Some("stringArray") => "ArgAction::Set",
            Some("uint") | Some("uint32") => "ArgAction::Set",
            _ => "ArgAction::Count",
        };
        cli_file_contents_string.push_str(&format!(
            "    cmd = cmd.arg(Arg::new(\"{key}\"){short}{long}.help(\"{help}\").action({action}).required({required}));\n",
            key = key,
            short = short_call,
            long = long_call,
            help = help,
            action = action,
            required = required
        ));
    }

    // Subcommands
    cli_file_contents_string.push_str("\n    cmd = cmd.subcommands(vec![\n");
    for cmd_spec in spec.children.COMMAND.values() {
        let mut builder = format!(
            "Command::new(\"{}\").about(\"{}\")",
            cmd_spec.name,
            cmd_spec.description.replace('"', "\\\"")
        );
        // if !keep_help {
        //     builder.push_str(".disable_help_flag(true)");
        // }

        // Flags for this command
        for flag in &cmd_spec.children.FLAG {
            let name = flag.long.as_deref().unwrap_or_else(|| flag.short.as_deref().expect("Flag must have either short or long name"));
            let key = name.trim_start_matches('-');
            if (!keep_help && key == "help") || (!keep_verbose && key == "verbose") {
                continue;
            }
            let short_call = flag.short.as_deref().map_or(String::new(), |short_form| format!(".short('{}')", short_form.trim_start_matches('-')));
            let long_call = flag.long.as_deref().map_or(String::new(), |long_form| format!(".long(\"{}\")", long_form.trim_start_matches('-')));
            let help = flag.description.as_deref().unwrap_or_default().replace('"', "\\\"");
            let required = flag.required.unwrap_or(false);
            let action = match flag.data_type.as_deref() {
                Some("string") | Some("stringArray") => "ArgAction::Set",
                Some("uint") | Some("uint32") => "ArgAction::Set",
                _ => "ArgAction::Count",
            };
            builder.push_str(&format!(
                ".arg(Arg::new(\"{key}\"){short}{long}.help(\"{help}\").action({action}).required({required}))",
                key = key,
                short = short_call,
                long = long_call,
                help = help,
                action = action,
                required = required
            ));
        }

        // Nested subcommands
        if !cmd_spec.children.COMMAND.is_empty() {
            builder.push_str(".subcommands(vec![");
            for sub in cmd_spec.children.COMMAND.values() {
                let mut sub_b = format!(
                    "Command::new(\"{}\").about(\"{}\")",
                    sub.name,
                    sub.description.replace('"', "\\\"")
                );

                for flag in &sub.children.FLAG {
                    let name = flag.long.as_deref().unwrap_or_else(|| flag.short.as_deref().expect("Flag must have either short or long name"));
                    let key = name.trim_start_matches('-');
                    if (!keep_help && key == "help") || (!keep_verbose && key == "verbose") {
                        continue;
                    }
                    let short_call = flag.short.as_deref().map_or(String::new(), |short_form| format!(".short('{}')", short_form.trim_start_matches('-')));
                    let long_call = flag.long.as_deref().map_or(String::new(), |long_form| format!(".long(\"{}\")", long_form.trim_start_matches('-')));
                    let help = flag.description.as_deref().unwrap_or_default().replace('"', "\\\"");
                    let required = flag.required.unwrap_or(false);
                    let action = match flag.data_type.as_deref() {
                        Some("string") | Some("stringArray") => "ArgAction::Set",
                        Some("uint") | Some("uint32") => "ArgAction::Set",
                        _ => "ArgAction::Count",
                    };
                    sub_b.push_str(&format!(
                        ".arg(Arg::new(\"{key}\"){short}{long}.help(\"{help}\").action({action}).required({required}))",
                        key = key,
                        short = short_call,
                        long = long_call,
                        help = help,
                        action = action,
                        required = required
                    ));
                }
                builder.push_str(&format!("{},", sub_b));
            }
            builder.push_str("])");
        }
        cli_file_contents_string.push_str(&format!("        {},\n", builder));
    }
    cli_file_contents_string.push_str("    ]);\n    cmd\n}\n");
    cli_file_contents_string
}

/// Build `main.rs` with dispatch and defaulted flag extraction
fn generate_main_builder(spec: &CliSpec, keep_help: bool, keep_verbose: bool) -> String {
    let mut lines = Vec::new();
    lines.push("mod cli;".into());
    lines.push("use cli::build_cli;".into());
    lines.push("\nfn main() {".into());
    lines.push("    let mut cmd = build_cli();".into());
    lines.push("    let matches = cmd.clone().try_get_matches().unwrap_or_else(|e| e.exit());".into());
    lines.push("    match matches.subcommand() {".into());

    for cmd_spec in spec.children.COMMAND.values() {
        let cmd_name = &cmd_spec.name;
        lines.push(format!("        Some((\"{}\", sub_m)) => {{", cmd_name));
        
        // extract this command's flags
        for flag in &cmd_spec.children.FLAG {
            let key = flag.long.as_deref().unwrap_or_else(|| flag.short.as_deref().expect("Flag must have either short or long name"));
            let flag_name = key.trim_start_matches('-');
            let var = flag_name.replace('-', "_");
            if (!keep_help && var == "help") || (!keep_verbose && var == "verbose") {
                continue;
            }

            let extract = match flag.data_type.as_deref() {
                Some("string") => format!("            let {v}: String = sub_m.get_one::<String>(\"{k}\").cloned().unwrap_or_else(|| \"mock_value\".to_string());", v=var, k=flag_name),
                Some("stringArray") => format!("            let {v}: Vec<String> = sub_m.get_many::<String>(\"{k}\").map(|vals| vals.cloned().collect()).unwrap_or_else(|| vec![\"mock_value\".to_string()]);", v=var, k=flag_name),
                Some("uint") | Some("uint32") => format!("            let {v}: u32 = sub_m.get_one::<u32>(\"{k}\").copied().unwrap_or(0);", v=var, k=flag_name),
                _ => format!("            let {v}: bool = sub_m.get_flag(\"{k}\");", v=var, k=flag_name),
            };
            lines.push(extract);
        }
        if !cmd_spec.children.COMMAND.is_empty() {
            lines.push("            match sub_m.subcommand() {".into());
            for sub in cmd_spec.children.COMMAND.values() {
                lines.push(format!("                Some((\"{}\", sub_sub_m)) => {{", sub.name));
                for flag in &sub.children.FLAG {
                    let key = flag.long.as_deref().unwrap_or_else(|| flag.short.as_deref().expect("Flag must have either short or long name"));
                    let flag_name = key.trim_start_matches('-');
                    let var = flag_name.replace('-', "_");
                    if (!keep_help && var == "help") || (!keep_verbose && var == "verbose") {
                        continue;
                    }
                    let extract = match flag.data_type.as_deref() {
                        Some("string") => format!("                    let {v}: String = sub_sub_m.get_one::<String>(\"{k}\").cloned().unwrap_or_default();", v=var, k=flag_name),
                        Some("stringArray") => format!("                    let {v}: Vec<String> = sub_sub_m.get_many::<String>(\"{k}\").map(|vals| vals.cloned().collect()).unwrap_or_default();", v=var, k=flag_name),
                        Some("uint") | Some("uint32") => format!("                    let {v}: u32 = sub_sub_m.get_one::<u32>(\"{k}\").copied().unwrap_or_default();", v=var, k=flag_name),
                        _ => format!("                    let {v}: bool = sub_sub_m.get_flag(\"{k}\");", v=var, k=flag_name),
                    };
                    lines.push(extract);
                }
                lines.push(format!("                    println!(\"Called {} {} with args: {{:?}}\", sub_sub_m);", cmd_name, sub.name));
                lines.push("                }".into());
            }
            lines.push(format!("                                _ => {{
                    match cmd.clone().find_subcommand_mut(\"{}\") {{
                        Some(c) => {{ c.print_help().expect(\"Failed to print help\"); }},
                        _ => std::process::exit(1),
                    }}
                }}", cmd_name));
            lines.push("            }".into());
        } else {
            lines.push(format!("            println!(\"Called {} with args: {{:?}}\", sub_m);", cmd_name));
        }
        lines.push("        }".into());
    }
    lines.push("        _ => { cmd.print_help().expect(\"Failed to print help\"); }".into());
    lines.push("    }".into());
    lines.push("}".into());

    lines.join("\n")
}
