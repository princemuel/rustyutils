use ::clap::Parser;
use ::list_sorter::Args;
use ::list_sorter::run;

fn main() {
    let _ = run(Args::parse());
}
