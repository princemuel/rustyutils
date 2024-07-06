use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

pub struct Config {
    pub input_file_path: PathBuf,
    pub output_file_path: PathBuf,
}

impl Config {
    pub fn build(
        args: impl Iterator<Item = String>,
    ) -> Result<Config, Box<dyn Error>> {
        let (input_file_path, output_file_path) = Config::parse_args(args)?;

        Config::validate_input_path(&input_file_path)?;
        Config::create_output_dir_if_needed(output_file_path.clone())?;

        Ok(Config { input_file_path, output_file_path })
    }

    fn parse_args(
        mut args: impl Iterator<Item = String>,
    ) -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
        args.next(); // Skip the first argument (program name)

        let mut input_file_path = None;
        let mut output_file_path = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--input" | "-i" => {
                    if let Some(path) = args.next() {
                        input_file_path = Some(PathBuf::from(path));
                    } else {
                        return Err("Missing input file path".into());
                    }
                },
                "--output" | "-o" => {
                    if let Some(path) = args.next() {
                        output_file_path = Some(PathBuf::from(path));
                    } else {
                        return Err("Missing output file path".into());
                    }
                },
                _ => {
                    return Err(format!("Unexpected argument: {}", arg).into());
                },
            }
        }

        let input_file_path =
            input_file_path.ok_or("Missing input file path")?;
        let output_file_path =
            output_file_path.ok_or("Missing output file path")?;

        Ok((input_file_path, output_file_path))
    }

    fn validate_input_path(path: &PathBuf) -> Result<(), Box<dyn Error>> {
        if !path.exists() {
            return Err(format!("Input file does not exist: {:?}", path).into());
        }
        if !path.is_file() {
            return Err(format!("Input path is not a file: {:?}", path).into());
        }
        Ok(())
    }

    fn create_output_dir_if_needed(
        path: PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        // Create parent directory for output file if necessary
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        Ok(())
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string(config.input_file_path)?;

    let mut results = parse_lines(&content);

    results.sort();
    results.dedup();

    let mut output_file = File::create(config.output_file_path)?;
    for result in results {
        writeln!(output_file, "{}", result)?;
    }

    Ok(())
}

pub fn parse_lines(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| {
            line.split_once('.')
                .map(|x| x.1)
                .unwrap_or(line)
                .trim()
                .to_lowercase()
        })
        .collect()
}
