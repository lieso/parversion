use std::io::{Read};
use std::io::{self, Write};
use atty::Stream;
use clap::{Arg, App};
use log::LevelFilter;
use env_logger::Builder;
use std::fs::File;
use std::str::FromStr;

use parversion::basis_graph::{BasisGraph};

mod environment;

fn load_stdin() -> io::Result<String> {
    log::trace!("In load_stdin");

    if atty::is(Stream::Stdin) {
        return Err(io::Error::new(io::ErrorKind::Other, "stdin not redirected"));
    }
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    return Ok(buffer);
}

fn init_logging() -> Builder {
    let mut builder = Builder::from_default_env();

    builder.filter(None, LevelFilter::Off); // disables all logging
    builder.filter_module("parversion", LevelFilter::Trace);

    let log_file = std::fs::File::create("./debug/debug.log").unwrap();
    builder.target(env_logger::Target::Pipe(Box::new(log_file)));

    builder.init();

    builder
}

fn load_basis_graph(file_name: &str) -> Result<BasisGraph, &str> {
    let mut file = match File::open(file_name) {
        Ok(file) => file,
        Err(_e) => return Err("Could not open file"),
    };

    let mut serialized = String::new();
    let _ = file.read_to_string(&mut serialized).map_err(|_e| "Could not read file to string");

    serde_json::from_str::<BasisGraph>(&serialized).map_err(|_e| "Could not deserialize basis graph")
}

fn save_basis_graph(graph: BasisGraph) {
    let serialized = serde_json::to_string(&graph).expect("Could not serialize basis graph");
    let mut file = File::create("./debug/basis_graph").expect("Could not create file");
    file.write_all(serialized.as_bytes()).expect("could not write to file");
}

fn main() {
    let _ = init_logging();

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
            .help("Set output format: JSON, JSON_SCHEMA, or XML"))
        .get_matches();

    let output_format = {
        let format_str = matches.value_of("format").unwrap_or("json");
        parversion::harvest::HarvestFormats::from_str(format_str)
            .expect("Could not initialize output format")
    };

    let basis_graph: Option<BasisGraph> = match matches.value_of("basis") {
        Some(file_name) => {
            log::debug!("basis graph file name: {}", file_name);
            let basis_graph = load_basis_graph(file_name).expect("Could not load basis graph from filesystem");

            Some(basis_graph)
        }
        None => {
            log::info!("Basis graph not provided");
            None
        }
    };

    let normalize_result = match matches.value_of("file") {
        Some(file_name) => {
            log::debug!("file_name: {}", file_name);
            parversion::normalize::normalize_file(file_name, basis_graph)
        }
        None => {
            log::info!("File not provided");
            parversion::normalize::normalize_text(document, basis_graph)
        }
    };

    if let Ok(normalize_result) = normalize_result {
        let serialized = parversion::harvest::serialize_harvest(
            normalize_result.harvest, 
            output_format
        ).expect("Unable to serialize result");

        if environment::is_local() {
            save_basis_graph(normalize_result.basis_graph.clone());
        }

        println!("{}", serialized);
    } else {
        println!("An error occurred while processing document");
    }
}
