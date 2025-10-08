mod cli_navigator_toolkit;
mod cli_parser;
mod comparison;
mod keyword_extractor;
mod models;
mod naive_tooltip_content_generator;
mod replicator;
mod summary_generator;
mod usage_parser;

use cli_navigator_toolkit::{
    run_cli_compare, run_cli_parser, run_cli_replicator, run_get_template_web_files,
    run_interactive_serve, run_keyword_extractor, run_summary_generator,
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
    /// Parses a CLI program written with the Cobra library and generates output in the specified format
    Parse {
        #[arg(value_name = "PROGRAM_NAME")]
        name: String,
        #[arg(short = 'o', long = "output", value_name = "PATH")]
        output_file: Option<PathBuf>,
        #[arg(
            short,
            long,
            value_name = "FORMAT",
            help = "Output format: json (default), zod, json-schema, or ts-dir"
        )]
        format: Option<String>,
        #[arg(
            short,
            long,
            value_name = "TAG",
            help = "Custom tag for organizing different versions/states of the CLI"
        )]
        tag: Option<String>,
    },
    /// Extracts unique keywords (commands, subcommands, and flags) from a parsed JSON file (outputs as CSV)
    UniqueKeywords {
        #[arg(value_name = "INPUT_JSON")]
        input_json: Option<PathBuf>,
        #[arg(short, long, value_name = "OUTPUT_PATH")]
        output_path: Option<PathBuf>,
    },
    /// Generates a summary of the CLI structure
    Summary {
        #[arg(value_name = "INPUT_JSON")]
        input_json: Option<PathBuf>,
        #[arg(short, long, value_name = "OUTPUT_PATH")]
        output_path: Option<PathBuf>,
        #[arg(short, long, value_name = "FORMAT")]
        format: Option<String>,
    },
    /// Downloads web interface templates to ~/.config/clint/templates/default for customization (optional - embedded templates used by default)
    GetTemplate {
        #[arg(short, long)]
        force: bool,
    },
    /// Starts an HTTP server to serve the CLI documentation
    Serve {
        #[arg(short, long, value_name = "TEMPLATE")]
        template: Option<String>,
        #[arg(short, long, value_name = "PORT")]
        port: Option<u16>,
        #[arg(short, long, value_name = "JSON_FILE")]
        input: Option<PathBuf>,
    },
    /// Generates a replica of the CLI program in RustLang using the clap library
    Replicate {
        #[arg(value_name = "INPUT_JSON")]
        input_json: Option<PathBuf>,
        #[arg(short, long, value_name = "OUTPUT_PATH")]
        output_path: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        keep_help_flags: bool,
        #[arg(long, default_value_t = false)]
        keep_verbose_flags: bool,
    },
    /// Generates the TypeScript file for the NaiveTooltip component
    NaiveTooltip {
        #[arg(value_name = "INPUT_JSON")]
        input_json: Option<PathBuf>,
        #[arg(short, long, value_name = "OUTPUT_PATH")]
        output_path: Option<PathBuf>,
    },
    /// Compares two parsed CLI structures and displays differences
    Compare {
        #[arg(value_name = "PROGRAM_NAME")]
        name: String,
        #[arg(
            long,
            value_name = "TAG1",
            help = "First tag/version to compare (defaults to latest)"
        )]
        from: Option<String>,
        #[arg(
            long,
            value_name = "TAG2",
            help = "Second tag/version to compare (defaults to second latest)"
        )]
        to: Option<String>,
        #[arg(
            short,
            long,
            value_name = "FORMAT",
            help = "Output format to compare: json (default), ts-dir"
        )]
        format: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Parse {
            name,
            output_file,
            format,
            tag,
        }) => {
            run_cli_parser(name, output_file.as_ref(), format.as_ref(), tag.as_ref());
        }
        Some(Commands::GetTemplate { force }) => {
            run_get_template_web_files(*force);
        }
        Some(Commands::UniqueKeywords {
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
            let input_file_name = input_json.with_extension("");
            let input_json_file_name = match input_file_name.file_name() {
                Some(name) => name.to_str(),
                None => None,
            };
            let output_path = match output_path {
                Some(path) => path,
                None => &current_dir()
                    .expect("Failed to get current directory")
                    .join(format!(
                        "{}-keywords.csv",
                        input_json_file_name.unwrap_or("output")
                    )),
            };

            run_keyword_extractor(input_json, output_path, FileOutputFormat::Csv);
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
        Some(Commands::Compare {
            name,
            from,
            to,
            format,
        }) => {
            run_cli_compare(name, from.as_ref(), to.as_ref(), format.as_ref());
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
