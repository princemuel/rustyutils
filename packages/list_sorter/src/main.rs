extern crate list_sorter;

use list_sorter::Config;
use std::{env, process};

fn main() {
    let config = Config::build(env::args()).unwrap_or_else(|exception| {
        eprintln!("Problem parsing arguments: {exception}");
        process::exit(1);
    });

    println!(
        "Formatting and Sorting the file {:?} into file {:?}",
        config.input_file_path, config.output_file_path
    );

    if let Err(exception) = list_sorter::run(config) {
        eprintln!("Application Error: {exception}");
        process::exit(1);
    };
}
