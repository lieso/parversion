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

#[derive(Debug, Serialize)]
struct Output<T, U> {
    parsers: Vec<T>,
    data: Vec<U>,
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

fn chunk_string(s: &str, chunk_size: usize) -> Vec<String> {
    log::trace!("In chunk_string");

    s.chars()
        .collect::<Vec<char>>()
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

fn save_parser_to_file<T>(parser: &T) where T: Serialize {
    log::trace!("In save_parser_to_file");

    let serialized = serde_json::to_string_pretty(parser).expect("Serialization failed");

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("parsers.json")
        .expect("Failed to open file");

    file.write_all(serialized.as_bytes()).expect("Failed to write to file");
    file.write_all("\n".as_bytes()).expect("Failed to write to file");
}

fn document_to_chat(document: String) {
    log::trace!("In document_to_chat");

    let chunks = chunk_string(&document, 20000);
    log::debug!("number of chunks: {}", chunks.len());

    let sample = &chunks[0];

    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let parser = parsers::chat::get_chat_parser(sample).await.unwrap();
        log::debug!("parser: {:?}", parser);

        save_parser_to_file(&parser);

        let chat: models::chat::Chat = transformers::chat::transform_document_to_chat(document, parser);
        log::debug!("chat: {:?}", chat);

        let output = serde_json::to_string(&chat).expect("Failed to serialize to JSON");
        println!("{}", output);
    });
}

fn document_to_list(document: String, maybe_parsers: Option<serde_json::Value>) {
    log::trace!("In document_to_list");

    let chunks = chunk_string(&document, 20000);
    log::debug!("number of chunks: {}", chunks.len());

    let sample = &chunks[0];

    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let mut parsers: Vec<models::list::ListParser> = Vec::new();

        if let Some(parser_input) = maybe_parsers {
            let Some(parser_input_array) = parser_input.as_array() else {
                panic!("Parsers input is not array");
            };

            for parser_input_array_item in parser_input_array.iter() {
                let Some(patterns) = parser_input_array_item["patterns"].as_object() else {
                    panic!("Parsers input array item is not object");
                };

                let mut parser = models::list::ListParser::new();

                for (key, value) in patterns.iter() {
                    log::debug!("key: {}, value: {}", key, value);

                    match remove_first_and_last(value.to_string()) {
                        Some(fixed_value) => {
                            let fixed_value2 = fixed_value.to_string().replace("\\\\\\", "");
                            log::debug!("fixed_value2: {}", fixed_value2);

                            parser.insert(key.to_string(), fixed_value2.to_string());
                        }
                        None => {
                            log::debug!("string less than two characters");
                        }
                    }

                }

                parsers.push(parser);
            }
        } else {
            parsers = parsers::list::get_list_parser(sample).await.unwrap();
        }
        log::debug!("parsers: {:?}", parsers);

        let mut output = Output {
            parsers: parsers.clone(),
            data: Vec::new(),
        };

        for parser in parsers.iter() {
            let list: models::list::List = transformers::list::transform_document_to_list(document.clone(), &parser);
            log::debug!("list: {:?}", list);

            output.data.push(list);
        }

        let serialized = serde_json::to_string(&output).expect("Failed to serialize to JSON");
        println!("{}", serialized);
    });
}

fn get_parsers_input(maybe_filename: Option<&str>) -> Option<serde_json::Value> {
    log::trace!("In get_parsers_input");

    match maybe_filename {
        Some(filename) => {
            log::debug!("filename: {}", filename);

            let mut parsers = String::new();

            let mut file = File::open(filename).unwrap_or_else(|err| {
                log::error!("Failed to open file: {}", err);
                process::exit(1);
            });

            file.read_to_string(&mut parsers).unwrap_or_else(|err| {
                log::error!("Failed to read file: {}", err);
                process::exit(1);
            });

            let json: serde_json::Value = serde_json::from_str(&parsers).expect("Failed to parse json string");
            log::debug!("json: {:?}", json);

            Some(json)
        }
        None => {
            log::info!("Parsers not provided in input");
            None
        }
    }
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

    let matches = App::new("document-to-json")
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

    if let Some(file_name) = matches.value_of("file") {
        log::debug!("file_name: {}", file_name);

        let mut file = File::open(file_name).unwrap_or_else(|err| {
            eprintln!("Failed to open file: {}", err);
            process::exit(1);
        });

        file.read_to_string(&mut document).unwrap_or_else(|err| {
            eprintln!("Failed to read file: {}", err);
            process::exit(1);
        });
    } else {
        log::info!("File not provided");
    }

    if document.trim().is_empty() {
        log::info!("Document not provided, aborting...");
        return;
    }


    let maybe_parsers_json = get_parsers_input(matches.value_of("parsers"));


    if let Some(data_type) = matches.value_of("type") {
        log::debug!("data_type: {}", data_type);

        match data_type {
            "chat" => document_to_chat(document),
            "list" => document_to_list(document, maybe_parsers_json),
            _ => log::error!("Unexpected data type: {}", data_type),
        }
        return;
    } else {
        log::info!("Data type not provided, aborting...");
        return;
    }
}

fn remove_first_and_last(s: String) -> Option<String> {
     let chars: Vec<char> = s.chars().collect();
     if chars.len() <= 2 {
         None
     } else {
         Some(chars[1..chars.len() - 1].iter().collect())
     }
 }
