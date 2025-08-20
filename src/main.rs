mod cli_navigator_toolkit;
mod cli_parser;
mod keyword_extractor;
mod models;
mod naive_tooltip_content_generator;
mod replicator;
mod summary_generator;
mod typescript_writer;
mod usage_parser;

use cli_navigator_toolkit::{
    run_cli_parser, run_cli_replicator, run_keyword_extractor, run_summary_generator,
    run_install_web_files, run_interactive_serve,
};
use models::FileOutputFormat;
use naive_tooltip_content_generator::write_ts_file;
use std::{env::current_dir, path::PathBuf};

use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Parses a CLI program written with the Cobra library and generates a JSON file with its structure
    Parse {
        /// Target CLI program name
        #[arg(value_name = "PROGRAM_NAME")]
        name: String,
        /// Output path for the parsed CLI file
        #[arg(short, long, value_name = "PATH")]
        output_file: Option<PathBuf>,
    },
    /// Extracts unique keywords (commands, subcommands, and flags) from a parsed JSON file
    UniqueKeywords {
        /// Input JSON file
        #[arg(value_name = "INPUT_JSON")]
        input_json: Option<PathBuf>,

        /// Output path for the keywords file (Default: Current path and <input_file_name>-keywords.json)
        #[arg(short, long, value_name = "OUTPUT_PATH")]
        output_path: Option<PathBuf>,

        /// Output format: markdown, json, text (Default: json)
        #[arg(short, long, value_name = "FORMAT")]
        format: Option<String>,
    },
    /// Generates a summary of the CLI structure
    Summary {
        /// Input JSON file
        #[arg(value_name = "INPUT_JSON")]
        input_json: Option<PathBuf>,

        /// Output path for the summary file (Default: Current path and <input_file_name>-summary.json)
        #[arg(short, long, value_name = "OUTPUT_PATH")]
        output_path: Option<PathBuf>,

        /// Output format: markdown, json, text (Default: json)
        #[arg(short, long, value_name = "FORMAT")]
        format: Option<String>,
    },
    /// Installs web interface files to ~/.config/clint/templates/default
    Install {
        /// Force overwrite of existing files
        #[arg(short, long)]
        force: bool,
    },
    /// Starts an HTTP server to serve the CLI documentation
    Serve {
        /// Template to use (default: "default")
        #[arg(short, long, value_name = "TEMPLATE")]
        template: Option<String>,

        /// Port for the web server (default: 8899)
        #[arg(short, long, value_name = "PORT")]
        port: Option<u16>,

        /// Path to a specific JSON file to serve (skips interactive selection)
        #[arg(short, long, value_name = "JSON_FILE")]
        input: Option<PathBuf>,
    },
    /// Generates a replica of the CLI program in RustLang using the clap library
    Replicate {
        /// Input JSON file
        #[arg(value_name = "INPUT_JSON")]
        input_json: Option<PathBuf>,

        /// Output path for the Rust file
        #[arg(short, long, value_name = "OUTPUT_PATH")]
        output_path: Option<PathBuf>,
        /// Keep the original clap-generated help flags/subcommand
        #[arg(long, default_value_t = false)]
        keep_help_flags: bool,
        /// Keep the original clap-generated verbose flags
        #[arg(long, default_value_t = false)]
        keep_verbose_flags: bool,
    },
    /// Generates the TypeScript file for the NaiveTooltip component
    NaiveTooltip {
        /// Input JSON file
        #[arg(value_name = "INPUT_JSON")]
        input_json: Option<PathBuf>,

        /// Output path for the Rust file
        #[arg(short, long, value_name = "OUTPUT_PATH")]
        output_path: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Parse { name, output_file }) => {
            run_cli_parser(name, output_file.as_ref());
        }
        Some(Commands::Install { force }) => {
            run_install_web_files(*force);
        }
        Some(Commands::UniqueKeywords {
            input_json,
            output_path,
            format,
        }) => {
            let input_json = match input_json {
                Some(path) => path,
                None => {
                    println!("No input JSON file provided.");
                    return;
                }
            };
            let input_file_name = input_json.with_extension("");
            let input_json_file_name = match input_file_name.file_name() {
                Some(name) => name.to_str(),
                None => None,
            };
            let out_path_extension = match output_path {
                Some(path) => path.extension().expect("Failed to get extension").to_str(),
                None => None,
            };
            let output_path = match output_path {
                Some(path) => path,
                None => &current_dir()
                    .expect("Failed to get current directory")
                    .join(format!(
                        "{}-keywords.json",
                        input_json_file_name.unwrap_or("output")
                    )),
            };
            let output_file_format = if output_path.exists() && format.is_none() {
                match out_path_extension {
                    Some(ext) => FileOutputFormat::from_str(ext),
                    None => FileOutputFormat::from_str("txt"),
                }
            } else {
                FileOutputFormat::from_str("txt")
            };

            run_keyword_extractor(
                input_json,
                output_path,
                output_file_format.expect("Failed to get output format"),
            );
        }
        Some(Commands::Summary {
            input_json,
            output_path,
            format,
        }) => {
            let input_json = match input_json {
                Some(path) => path,
                None => {
                    println!("No input JSON file provided.");
                    return;
                }
            };
            let input_file_name = input_json.with_extension("");
            let input_json_file_name = match input_file_name.file_name() {
                Some(name) => name.to_str(),
                None => None,
            };
            let out_path = match output_path {
                Some(path) => path,
                None => &current_dir()
                    .expect("Failed to get current directory")
                    .join(format!(
                        "{}-summary.json",
                        input_json_file_name.unwrap_or("output")
                    )),
            };
            let out_path_extension = match output_path {
                Some(path) => path.extension().expect("Failed to get extension").to_str(),
                None => None,
            };
            let output_path = match output_path {
                Some(path) => path,
                None => &current_dir()
                    .expect("Failed to get current directory")
                    .join(format!(
                        "{}-keywords.json",
                        input_json_file_name.unwrap_or("output")
                    )),
            };
            let output_file_format = if output_path.exists() && format.is_none() {
                match out_path_extension {
                    Some(ext) => FileOutputFormat::from_str(ext),
                    None => FileOutputFormat::from_str("txt"),
                }
            } else {
                FileOutputFormat::from_str("txt")
            };
            run_summary_generator(
                input_json,
                out_path,
                output_file_format.expect("Failed to get output format"),
            );
        }
        Some(Commands::Serve {
            template,
            port,
            input,
        }) => {
            run_interactive_serve(template.as_ref(), *port, input.as_ref());
        }
        Some(Commands::Replicate {
            input_json,
            output_path,
            keep_help_flags,
            keep_verbose_flags,
        }) => {
            let input_json = match input_json {
                Some(path) => path,
                None => {
                    println!("No input JSON file provided.");
                    return;
                }
            };
            let input_file_name = input_json.with_extension("");
            let input_json_file_name = match input_file_name.file_name() {
                Some(name) => name.to_str(),
                None => None,
            };
            let out_path = match output_path {
                Some(path) => path,
                None => &current_dir()
                    .expect("Failed to get current directory")
                    .join(format!(
                        "{}-replica.rs",
                        input_json_file_name.unwrap_or("output")
                    )),
            };
            run_cli_replicator(input_json, out_path, *keep_help_flags, *keep_verbose_flags);
        }
        Some(Commands::NaiveTooltip {
            input_json,
            output_path,
        }) => {
            let input_json = match input_json {
                Some(path) => path,
                None => {
                    println!("No input JSON file provided.");
                    return;
                }
            };
            let input_file_name: PathBuf = input_json.with_extension("");
            let input_json_file_name = match input_file_name.file_name() {
                Some(name) => name.to_str(),
                None => None,
            };
            let out_path = match output_path {
                Some(path) => path,
                None => &current_dir()
                    .expect("Failed to get current directory")
                    .join(format!(
                        "./out/{}-naive_tooltip.ts",
                        input_json_file_name.unwrap_or("output")
                    )),
            };
            write_ts_file(input_json, out_path).expect("Failed to write TypeScript file");
        }
        None => {
            let mut cmd = Cli::command();
            cmd.print_help().expect("Failed to print help");
            println!();
            std::process::exit(0);
        }
    }

    // Continued program logic goes here...
}
