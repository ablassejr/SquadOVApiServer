use structopt::StructOpt;
use squadov_common::csgo::parser::CsgoDemoParser;

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short, long)]
    file: String,
}

fn main() {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let opts = Options::from_args();
    println!("FILE: {}", opts.file);

    let demo = CsgoDemoParser::from_path(std::path::Path::new(&opts.file)).unwrap();
    println!("Demo: {:?}", demo);
}