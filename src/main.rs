extern crate simple_logging;
extern crate log;

use std::io::{Read};
use std::io::{self};
use atty::Stream;
use clap::{Arg, App};
use log::LevelFilter;

pub mod parsers;
pub mod models;
pub mod transformers;
pub mod prompts;
pub mod utilities;
pub mod adapters;

fn load_stdin() -> io::Result<String> {
    log::trace!("In load_stdin");

    if atty::is(Stream::Stdin) {
        return Err(io::Error::new(io::ErrorKind::Other, "stdin not redirected"));
    }
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    return Ok(buffer);
}

fn main() {
    log::trace!("In main");

    let _ = simple_logging::log_to_file("debug.log", LevelFilter::Trace);

    let mut document = String::new();

    match load_stdin() {
        Ok(stdin) => {
            document = stdin;
        }
        Err(_e) => {
            log::debug!("Did not receive input from stdin");
        }
    }

    let matches = App::new("parversion")
        .arg(Arg::with_name("parsers")
             .short('p')
             .long("parsers")
             .value_name("PARSERS")
             .required(false))
        .arg(Arg::with_name("file")
             .short('f')
             .long("file")
             .value_name("FILE")
             .help("Provide file as document for processing"))
        .get_matches();

    let result = match matches.value_of("file") {
        Some(file_name) => {
            log::debug!("file_name: {}", file_name);
            parversion::file_to_json(file_name)
        }
        None => {
            log::info!("File not provided");
            let result = parversion::string_to_json(document);
            panic!("dev");
        }
    };

    if let Ok(result) = result {
        log::debug!("result: {:?}", result);

        let serialized = serde_json::to_string(&result).expect("Failed to serialize to JSON");
        println!("{}", serialized);
    } else {
        println!("error");
    }
}

