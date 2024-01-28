extern crate simple_logging;
extern crate log;

use tokio::runtime::Runtime;
use std::fs::File;
use std::process;
use std::io::{Read};
use std::io::{self};
use atty::Stream;
use clap::{Arg, App};
use log::LevelFilter;

mod prompts;
mod utilities;
mod parsers;
mod models;

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
    s.chars()
        .collect::<Vec<char>>()
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

fn search_and_extract<'a>(
    document: &'a str,
    index: usize,
    search_forward: bool,
    target_substring: &str,
    delimiter_substring: &str
) -> Option<&'a str> {
    if search_forward {
        if let Some(start_pos) = document[index..].find(target_substring) {
            let start_pos = start_pos + index + target_substring.len();
            if let Some(end_pos) = document[start_pos..].find(delimiter_substring) {
                let end_pos = start_pos + end_pos;
                return Some(&document[start_pos..end_pos]);
            }
        }
    } else {
        if let Some(end_pos) = document[..index].rfind(target_substring) {
            if let Some(start_pos) = document[..end_pos].rfind(delimiter_substring) {
                return Some(&document[(start_pos + delimiter_substring.len())..end_pos]);
            }
        }
    }

    None
}

fn document_to_chat(document: String) {
    log::trace!("In document_to_chat");

    let chunks = chunk_string(&document, 20000);
    log::debug!("number of chunks: {}", chunks.len());

    let chunk = &chunks[3];

    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let chat_parser = parsers::get_chat_parser(chunk).await.unwrap();
        log::debug!("{:?}", chat_parser);


        let content_prefix = &chat_parser.content.prefix;
        log::debug!("{}", content_prefix);
        let content_suffix = &chat_parser.content.suffix;
        log::debug!("{}", content_suffix);

        let mut chat_posts = Vec::new();
        let current = document.clone();
        let mut start_offset = 0;

        loop {
            // Fix Rust utf boundary issue when slicing with offset
            let fixed_index = current.char_indices()
                .map(|(i, _)| i)
                .take_while(|&i| i <= start_offset)
                .last()
                .unwrap_or(0);

            let current_slice = &current[fixed_index..];

            if let Some(start_index) = current_slice.find(content_prefix) {

                if let Some(end_index) = current[start_offset + start_index..].find(content_suffix) {

                    let mut content = &current[start_offset + start_index..start_offset + start_index + end_index];
                    content = &content[content_prefix.len()..content.len()];






                    let id_start_index = start_offset + start_index;

                    if chat_parser.id.relative == "before" {

                        if let Some(id) = search_and_extract(&document, id_start_index, false, &chat_parser.id.suffix, &chat_parser.id.prefix) {





                            if chat_parser.parent_id.relative == "before" {

                                if let Some(parent_id) = search_and_extract(&document, id_start_index, false, &chat_parser.parent_id.suffix, &chat_parser.parent_id.prefix) {



                                    let chat_post = models::ChatPost {
                                        parent_id: parent_id.to_string(),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    chat_posts.push(chat_post);



                                } else {
                                    let chat_post = models::ChatPost {
                                        parent_id: String::from(""),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    chat_posts.push(chat_post);
                                }

                            } else {

                                if let Some(parent_id) = search_and_extract(&document, id_start_index, true, &chat_parser.parent_id.prefix, &chat_parser.parent_id.suffix) {

                                    let chat_post = models::ChatPost {
                                        parent_id: parent_id.to_string(),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    chat_posts.push(chat_post);



                                } else {
                                    let chat_post = models::ChatPost {
                                        parent_id: String::from(""),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    chat_posts.push(chat_post);
                                }

                            }








                        } else {
                            log::error!("Could not find content id");
                        }
                    } else {

                        if let Some(id) = search_and_extract(&document, id_start_index, true, &chat_parser.id.prefix, &chat_parser.id.suffix) {





                            if chat_parser.parent_id.relative == "before" {

                                if let Some(parent_id) = search_and_extract(&document, id_start_index, false, &chat_parser.parent_id.suffix, &chat_parser.parent_id.prefix) {



                                    let chat_post = models::ChatPost {
                                        parent_id: parent_id.to_string(),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    chat_posts.push(chat_post);



                                } else {
                                    let chat_post = models::ChatPost {
                                        parent_id: String::from(""),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    chat_posts.push(chat_post);
                                }

                            } else {

                                if let Some(parent_id) = search_and_extract(&document, id_start_index, true, &chat_parser.parent_id.prefix, &chat_parser.parent_id.suffix) {

                                    let chat_post = models::ChatPost {
                                        parent_id: parent_id.to_string(),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    chat_posts.push(chat_post);



                                } else {
                                    let chat_post = models::ChatPost {
                                        parent_id: String::from(""),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    chat_posts.push(chat_post);
                                }

                            }








                        } else {
                            log::error!("Could not find content id");
                        }
                    }




                    start_offset = start_offset + start_index + end_index + content_suffix.len();

                } else {
                    break;
                }
            } else {
                break;
            }
        }

        log::debug!("posts: {}", chat_posts.len());


        //  * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * *
        //  * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * *
        //  * * * * * * * * * * * * * *

        let final_output = serde_json::to_string(&chat_posts).expect("Failed to serialize to JSON");
        println!("{}", final_output);

        //  * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * *
        //  * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * *
        //  * * * * * * * * * * * * * *
    });

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
        log::debug!("File not provided");
    }

    if document.trim().is_empty() {
        log::debug!("Document not provided, aborting...");
        return;
    }

    if let Some(data_type) = matches.value_of("type") {
        log::debug!("data_type: {}", data_type);

        match data_type {
            "chat" => document_to_chat(document),
            _ => log::error!("Unexpected data type: {}", data_type),
        }
        return;
    } else {
        log::info!("Data type not provided, aborting...");
        return;
    }
}
