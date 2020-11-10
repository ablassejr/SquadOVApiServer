use structopt::StructOpt;
use serde_json;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use squadov_common::hearthstone::{HearthstoneRawLog, power_parser::HearthstonePowerLogParser};

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short, long)]
    file: String,
    #[structopt(short, long)]
    output: String
}

fn main() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    let opts = Options::from_args();
    println!("FILE: {}", opts.file);
    let file = File::open(opts.file).unwrap();
    let reader = BufReader::new(file);
    let raw_json_logs : serde_json::Value = serde_json::from_reader(reader).unwrap();
    let power_logs: Vec<HearthstoneRawLog> = serde_json::from_value(raw_json_logs).unwrap();
    let mut parser = HearthstonePowerLogParser::new(true);
    parser.parse(&power_logs).unwrap();

    println!("OUTPUT: {}", opts.output);
    let file = File::create(opts.output).unwrap();
    let mut writer = BufWriter::new(file);
    writeln!(&mut writer, "============== LOGS ==============").unwrap();
    writeln!(&mut writer, "{}", parser.fsm.raw_logs_to_string()).unwrap();
    writeln!(&mut writer, "=================== BASIC STATE ====================").unwrap();
    writeln!(&mut writer, "{}", parser.state).unwrap();
    writeln!(&mut writer, "=================== SNAPSHOTS ====================").unwrap();
    for snap in &parser.fsm.game.borrow().snapshots {
        writeln!(&mut writer, "{}", snap).unwrap();
    }
    writeln!(&mut writer, "=================== ACTIONS ====================").unwrap();
    for action in &parser.fsm.game.borrow().actions {
        writeln!(&mut writer, "{}", action).unwrap();
    }
}