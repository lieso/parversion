use std::io::{Read};
use std::io::{self, Write};
use atty::Stream;
use clap::{Arg, App};
use log::LevelFilter;
use env_logger::Builder;
use std::fs::File;
use std::str::FromStr;
use serde_json::{from_str, to_string, Value};

mod error;
mod llm;
mod node_data;
mod node_data_structure;
mod utility;
mod xml_node;
mod config;
mod constants;
mod macros;
mod environment;
mod normalize;
mod content;
mod graph_node;
mod basis_graph;
mod basis_node;
mod harvest;

use basis_graph::{BasisGraph};

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

    match serde_json::from_str::<BasisGraph>(&serialized) {
        Ok(basis_graph) => Ok(basis_graph),
        Err(_e) => {

            // TODO: serialize basis graph root as unescaped json
            // so we won't have to do the following workaround

            let mut json_value: Value = match from_str(&serialized) {
                Ok(value) => value,
                Err(_e) => return Err("Could not parse JSON"),
            };

            if let Some(root_value) = json_value.get_mut("root") {
                if let Ok(root_str) = to_string(root_value) {
                    log::debug!("root_str: {}", root_str);
                    *root_value = Value::String(root_str);
                } else {
                    return Err("Failed to convert root to string");
                }
            }

            let modified_serialized = match to_string(&json_value) {
                Ok(json_str) => json_str,
                Err(_e) => return Err("Failed to serialize modified JSON"),
            };

            serde_json::from_str::<BasisGraph>(&modified_serialized).map_err(|_e| "Could not deserialize basis graph after modification")
        }
    }
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
        .arg(Arg::with_name("url")
            .short('u')
            .long("url")
            .value_name("URL")
            .help("The full URL that identifies and locates the provided document"))
        .arg(Arg::with_name("graphs")
            .short('g')
            .long("graphs")
            .value_name("GRAPHS")
            .help("Provide file path describing location of an analyzed basis graph to be used for interpretation"))
        .get_matches();

    let output_format = {
        let format_str = matches.value_of("format").unwrap_or("json");
        harvest::HarvestFormats::from_str(format_str)
            .expect("Could not initialize output format")
    };

    let url: Option<&str> = matches.value_of("url");

    let basis_graph: Option<Box<BasisGraph>> = match matches.value_of("basis") {
        Some(file_name) => {
            let basis_graph = load_basis_graph(file_name)
                .expect("Could not load basis graph from filesystem");

            Some(Box::new(basis_graph))
        }
        None => {
            log::info!("Basis graph not provided");
            None
        }
    };

    let other_basis_graphs: Vec<BasisGraph> = match matches.value_of("graphs") {
        Some(path) => {
            let basis_graph = load_basis_graph(path)
                .expect("Could not load basis graph from filesystem");
            vec![basis_graph]
        }
        None => {
            log::info!("Other basis graphs not provided");
            Vec::new()
        }
    };

    let normalize_result = match matches.value_of("file") {
        Some(file_name) => {
            normalize::normalize_file(
                url.map(|s| s.to_string()),
                file_name.to_string(),
                basis_graph,
                other_basis_graphs
            )
        }
        None => {
            log::info!("File not provided");
            normalize::normalize_text(
                url.map(|s| s.to_string()),
                document,
                basis_graph,
                other_basis_graphs
            )
        }
    };

    if let Ok(normalize_result) = normalize_result {
        let serialized = harvest::serialize_harvest(
            normalize_result.harvest, 
            output_format
        ).expect("Unable to serialize result");

        if environment::is_local() {
            save_basis_graph(normalize_result.output_basis_graph.clone());
            log::info!("Saved basis graph to filesystem");
        }

        println!("{}", serialized);
    } else {
        println!("An error occurred while processing document");
    }
}
