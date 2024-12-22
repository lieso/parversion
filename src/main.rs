use std::io::{Read};
use std::io::{self, Write};
use atty::Stream;
use clap::{Arg, App};
use log::LevelFilter;
use std::fs::File;
use std::str::FromStr;
use serde_json::{from_str, to_string, Value};
use std::io::stdout;
use fern::Dispatch;
use tokio::runtime::Runtime;

mod analysis;
mod basis_network;
mod basis_node;
mod basis_graph;
mod config;
mod context;
mod data_node;
mod document;
mod environment;
mod hash;
mod id;
mod lineage;
mod macros;
mod model;
mod normalize;
mod organize;
mod runtimes;
mod transformation;
mod translate;
mod types;

use crate::config::{CONFIG};

fn load_stdin() -> io::Result<String> {
    log::trace!("In load_stdin");

    if atty::is(Stream::Stdin) {
        return Err(io::Error::new(io::ErrorKind::Other, "stdin not redirected"));
    }
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    return Ok(buffer);
}

fn init_logging() {
    let path = format!("{}/{}", read_lock!(CONFIG).dev.debug_dir, "debug.log");
    let log_file = File::create(path).expect("Could not create log file");

    Dispatch::new()
        .level(LevelFilter::Off)
        .level_for("parversion", LevelFilter::Trace)
        .chain(stdout())
        .chain(log_file)
        .apply()
        .expect("Could not initialize logging");
}

fn main() {
    Runtime::new().unwrap().block_on(async {
        init_logging();

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
            .arg(Arg::with_name("file")
                 .short('f')
                 .long("file")
                 .value_name("FILE")
                 .help("Provide file as document for processing"))
            .arg(Arg::with_name("basis")
                 .short('b')
                 .long("basis")
                 .value_name("BASIS")
                 .help("Provide basis graph"))
            .arg(Arg::with_name("format")
                .short('o')
                .long("output-format")
                .value_name("FORMAT")
                .help("Set output format: JSON, HTML, XML, text"))
            .arg(Arg::with_name("url")
                .short('u')
                .long("url")
                .value_name("URL")
                .help("The full URL that identifies and locates the provided document"))
            .get_matches();

        let url: Option<&str> = matches.value_of("url");

        unimplemented!()
    });
}
