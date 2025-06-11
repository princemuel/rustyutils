use ::clap::Parser;
use ::parser::parse_lines;
use ::path_utils::resolve_path;

use ::std::{error::Error, fs, path::PathBuf};

mod parser;
mod path_utils;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = "This program processes a text file by reading its contents, sorting all lines in alphabetical order, and removing any numbering or leading indices at the beginning of each line. It creates a new file with the sorted contents or prints the result to stdout if no output file is specified."
)]
pub struct Args {
    /// Source file path (input file)
    #[arg(
        short,
        long,
        required = true,
        help = "(PathBuf, required) Path to the input file containing the raw text to process"
    )]
    source_file: PathBuf,

    /// Result file path (output file)
    #[arg(
        short,
        long,
        required = false,
        help = "(PathBuf, optional) Path to the output file where the sorted content will be saved. If not provided, prints to stdout"
    )]
    result_file: Option<PathBuf>,

    /// Flag to enable case-insensitive processing
    #[arg(
        short = 'i',
        long = "case-insensitive",
        default_value = "false",
        help = "Process the lines in a case-insensitive manner"
    )]
    case_insensitive: bool,
}

pub fn run(config: Args) -> Result<(), Box<dyn Error>> {
    let file_path = resolve_path(config.source_file)?;

    let content = fs::read_to_string(file_path)?;

    let processed_lines = parse_lines(&content, config.case_insensitive);

    let result = processed_lines.into_iter().collect::<Vec<_>>().join("\n");

    match config.result_file {
        Some(output_path) => fs::write(output_path, result)?,
        None => println!("{result}"),
    }

    Ok(())
}
