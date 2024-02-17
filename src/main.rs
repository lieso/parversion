extern crate simple_logging;
extern crate log;

use serde::Serialize;
use tokio::runtime::Runtime;
use std::fs::{OpenOptions, File};
use std::process;
use std::io::{Read};
use std::io::{self};
use atty::Stream;
use clap::{Arg, App};
use log::LevelFilter;
use std::io::Write;

mod prompts {
    pub mod chat;
    pub mod list;
}
mod utilities;
mod parsers {
    pub mod chat;
    pub mod list;
}
mod models {
    pub mod chat;
    pub mod list;
}
mod transformers {
    pub mod chat;
    pub mod list;
}

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
        .arg(Arg::with_name("type")
             .short('t')
             .long("type")
             .value_name("TYPE")
             .required(true))
        .arg(Arg::with_name("file")
             .short('f')
             .long("file")
             .value_name("FILE")
             .help("Provide file as document for processing"))
        .get_matches();



    //let maybe_parsers_json = get_parsers_input(matches.value_of("parsers"));



    if let Some(document_type) = matches.value_of("type") {
        log::debug!("document_type: {}", document_type);


        let result = match matches.value_of("file") {
            Some(file_name) => {
                log::debug!("file_name: {}", file_name);
                parversion::file_to_json(file_name, document_type)
            }
            None => {
                log::info!("File not provided");
                parversion::string_to_json(document, document_type)
            }
        };

        if let Ok(result) = result {
            let serialized = serde_json::to_string(&result).expect("Failed to serialize to JSON");
            println!("{}", serialized);
        } else {
            println!("error");
        }

    } else {
        log::info!("Data type not provided, aborting...");
    }
}

