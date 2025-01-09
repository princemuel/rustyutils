use clap::Parser;
use list_sorter::run;
use list_sorter::Args;

fn main() {
    let _ = run(Args::parse());
}
