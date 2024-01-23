extern crate simple_logging;
extern crate log;

use tokio::runtime::Runtime;
use std::env;
use std::fs::File;
use std::process;
use std::io::{Read};
use std::io::{self, BufRead};
use reqwest::header;
use serde_json::json;
use atty::Stream;
use clap::{Arg, App};
use log::LevelFilter;
use std::io::{Error, ErrorKind};
use serde::{Serialize};

#[derive(Debug)]
struct ConversationParserParentId {
    prefix: String,
    suffix: String,
    relative: String,
}

#[derive(Debug)]
struct ConversationParserId {
    prefix: String,
    suffix: String,
    relative: String,
}

#[derive(Debug)]
struct ConversationParserContent {
    prefix: String,
    suffix: String,
}

#[derive(Debug)]
struct ConversationParser {
    parent_id: ConversationParserParentId,
    id: ConversationParserId,
    content: ConversationParserContent,
}

#[derive(Serialize)]
struct ConversationPost {
    parent_id: String,
    id: String,
    content: String,
}

#[derive(Serialize)]
struct Conversation {
    posts: Vec<ConversationPost>,
}


async fn llm_parse(content: String) -> Result<serde_json::Value, io::Error> {
    log::debug!("{}", content);

    if let Ok(openai_api_key) = env::var("OPENAI_API_KEY") {
        let request_json = json!({
            "model":  "gpt-4-1106-preview",
            "temperature":  0.7,
            "messages":  [
                {
                    "role": "user",
                    "content": content
                }
            ]
        });
        
        let url = "https://api.openai.com/v1/chat/completions";
        let authorization = format!("Bearer {}", openai_api_key);

        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .json(&request_json)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, authorization)
            .send()
            .await;


        match response {
            Ok(success_response) => {
                let json_response = success_response.json::<serde_json::Value>().await;
                match json_response {
                    Ok(json_data) => {
                        log::debug!("{:?}", json_data);
                        Ok(json_data)
                    }
                    Err(err) => {
                        return Err(Error::new(ErrorKind::InvalidData, "error"));
                    }
                }


            },
            Err(err) => {
                return Err(Error::new(ErrorKind::InvalidData, "error"));
            }
        }


    } else {
        log::error!("OPENAI_API_KEY could not be found in environment!");
        process::exit(1);
    }
}

fn remove_codeblock_delimiters(s: &str) -> &str {
     if s.starts_with("```") && s.ends_with("```") {
         s.trim_start_matches("```").trim_end_matches("```")
     } else {
         s
     }
}

fn remove_text_until_brace(s: &str) -> &str {
    if let Some(index) = s.find('{') {
        &s[index..]
    } else {
        s
    }
}

async fn get_conversation_parser(document: &str) -> Result<ConversationParser, io::Error> {
    log::trace!("In get_conversation_parser");


    let conversation_parser_parent_id = get_conversation_parser_parent_id(document).await.unwrap();
    let conversation_parser_id = get_conversation_parser_id(document).await.unwrap();
    let conversation_parser_content = get_conversation_parser_content(document).await.unwrap();

    let conversation_parser = ConversationParser {
        parent_id: conversation_parser_parent_id,
        id: conversation_parser_id,
        content: conversation_parser_content,
    };

    return Ok(conversation_parser)
}

async fn get_conversation_parser_parent_id(document: &str) -> Result<ConversationParserParentId, io::Error> {
    log::trace!("In get_conversation_parser_parent_id");

    let prompt = r##"
Hi ChatGPT. Please examine the subsequent text and do your best to identify posts/comments like that people leave on websites such as discussion forums. If present, try to then see if these posts have a parent id like when a person replies to another post. If these parent references are present, extract the common text that directly precedes and follows a post's parent identifier (id) associated with the post content, as the prefix and suffix, respectively. Additionally, determine whether the parent id comes 'before' or 'after' the post content and label this value 'relative'. If you do not see any posts in the text, respond only with the digit '0'. Otherwise print your response based on the following json:
   {"prefix":"prefix string","suffix":"suffix string","relative": "before or after"}
When populating the prefix or suffix string, ensure newline escape characters are double-escaped. Do not include triple-backticks or anything signifying a code block.
Please do not include any introduction or final summary in your response. Thank you.
"##;
    let content = format!("{} {}", prompt, document);

    let maybeOpenAiResponse = llm_parse(content).await;

    match maybeOpenAiResponse {
        Ok(openAiResponse) => {
            let Some(choices) = openAiResponse["choices"].as_array() else {
                log::error!("Could not get choices array from OpenAI response");
                return Err(Error::new(ErrorKind::InvalidData, "error"));
            };

            let choice = &choices[0];
            log::debug!("{:?}", &choice);

            let message = &choice["message"];
            log::debug!("message: {:?}", message);

            let response_content = &message["content"].as_str().unwrap();
            log::debug!("-----response_content: {}", response_content);



            // remove code-block formatting
            let without_backticks = remove_codeblock_delimiters(response_content);
            log::debug!("without_backticks: {}", without_backticks);
            let without_label = remove_text_until_brace(without_backticks);
            log::debug!("without_label: {}", without_label);



            let prefix_suffix_relative: serde_json::Value = serde_json::from_str(&without_label).expect("Failed to parse json string");
            log::debug!("{:?}", prefix_suffix_relative);




            let prefix = &prefix_suffix_relative["prefix"].as_str().unwrap();
            log::debug!("prefix: {}", prefix);

            let suffix = &prefix_suffix_relative["suffix"].as_str().unwrap();
            log::debug!("suffix: {}", suffix);

            let relative = &prefix_suffix_relative["relative"].as_str().unwrap();
            log::debug!("relative: {}", relative);


            let conversation_parser_parent_id = ConversationParserParentId {
                prefix: prefix.to_string(),
                suffix: suffix.to_string(),
                relative: relative.to_string(),
            };

            return Ok(conversation_parser_parent_id)

        }
        Err(e) => {
            log::debug!("Did not receive response from open ai");
            return Err(Error::new(ErrorKind::InvalidData, "error"));
        }
    }


}

async fn get_conversation_parser_id(document: &str) -> Result<ConversationParserId, io::Error> {
    log::trace!("In get_conversation_parser_id");
    
    let prompt = r##"
    Hi ChatGPT. Please examine the subsequent text and do your best to identify posts/comments like that people leave on websites such as discussion forums. If present, extract
    the common text that directly precedes and follows the identifier (id) associated with the post content, as the prefix and suffix, respectively. Additionally, determine whet
    her the identifier comes 'before' or 'after' the post content and label this value 'relative'. If you do not see any posts in the text, respond only with the digit '0'. Othe
    rwise print your response based on the following json:
    {"prefix":"prefix string","suffix":"suffix string","relative": "before or after"}
    When populating the prefix or suffix string, ensure newline escape characters are double-escaped. Do not include triple-backticks or anything signifying a code block. Please
     do not include any introduction or final summary in your response. Thank you.
    "##;
    let content = format!("{} {}", prompt, document);

    let maybeOpenAiResponse = llm_parse(content).await;

    match maybeOpenAiResponse {
        Ok(openAiResponse) => {
            let Some(choices) = openAiResponse["choices"].as_array() else {
                log::error!("Could not get choices array from OpenAI response");
                return Err(Error::new(ErrorKind::InvalidData, "error"));
            };

            let choice = &choices[0];
            log::debug!("{:?}", &choice);

            let message = &choice["message"];
            log::debug!("message: {:?}", message);

            let response_content = &message["content"].as_str().unwrap();
            log::debug!("-----response_content: {}", response_content);



            // remove code-block formatting
            let without_backticks = remove_codeblock_delimiters(response_content);
            log::debug!("without_backticks: {}", without_backticks);
            let without_label = remove_text_until_brace(without_backticks);
            log::debug!("without_label: {}", without_label);



            let prefix_suffix_relative: serde_json::Value = serde_json::from_str(&without_label).expect("Failed to parse json string");
            log::debug!("{:?}", prefix_suffix_relative);




            let prefix = &prefix_suffix_relative["prefix"].as_str().unwrap();
            log::debug!("prefix: {}", prefix);

            let suffix = &prefix_suffix_relative["suffix"].as_str().unwrap();
            log::debug!("suffix: {}", suffix);

            let relative = &prefix_suffix_relative["relative"].as_str().unwrap();
            log::debug!("relative: {}", relative);


            let conversation_parser_id = ConversationParserId {
                prefix: prefix.to_string(),
                suffix: suffix.to_string(),
                relative: relative.to_string(),
            };

            return Ok(conversation_parser_id)

        }
        Err(e) => {
            log::debug!("Did not receive response from open ai");
            return Err(Error::new(ErrorKind::InvalidData, "error"));
        }
    }

}

async fn get_conversation_parser_content(document: &str) -> Result<ConversationParserContent, io::Error> {
    log::trace!("In get_conversation_parser_content");

    let prompt = r##"
    Hi ChatGPT. Please examine the subsequent text and do your best to identify posts/comments like that people leave on websites such as discussion forums. If present, extract the common text that directly precedes the post content (prefix), and also the common text that immediately follows the post content (suffix). If you do not see any posts in the text, respond only with the digit '0'. Otherwise print your response based on the following json:
    {"prefix":"prefix string","suffix":"suffix string"}
    When populating the prefix or suffix string, ensure newline escape characters are double-escaped. Do not include triple-backticks or anything signifying a code block. Please do not include any introduction or final summary in your response. Thank you.
    "##;
    let content = format!("{} {}", prompt, document);

    let maybeOpenAiResponse = llm_parse(content).await;

    match maybeOpenAiResponse {
        Ok(openAiResponse) => {
            let Some(choices) = openAiResponse["choices"].as_array() else {
                log::error!("Could not get choices array from OpenAI response");
                return Err(Error::new(ErrorKind::InvalidData, "error"));
            };

            let choice = &choices[0];
            log::debug!("{:?}", &choice);

            let message = &choice["message"];
            log::debug!("message: {:?}", message);

            let response_content = &message["content"].as_str().unwrap();
            log::debug!("-----response_content: {}", response_content);



            // remove code-block formatting
            let without_backticks = remove_codeblock_delimiters(response_content);
            log::debug!("without_backticks: {}", without_backticks);
            let without_label = remove_text_until_brace(without_backticks);
            log::debug!("without_label: {}", without_label);



            let prefix_suffix: serde_json::Value = serde_json::from_str(&without_label).expect("Failed to parse json string");
            log::debug!("{:?}", prefix_suffix);




            let prefix = &prefix_suffix["prefix"].as_str().unwrap();
            log::debug!("prefix: {}", prefix);

            let suffix = &prefix_suffix["suffix"].as_str().unwrap();
            log::debug!("suffix: {}", suffix);



            let conversation_parser_content = ConversationParserContent {
                prefix: prefix.to_string(),
                suffix: suffix.to_string()
            };

            return Ok(conversation_parser_content)

        }
        Err(e) => {
            log::debug!("Did not receive response from open ai");
            return Err(Error::new(ErrorKind::InvalidData, "error"));
        }
    }
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

fn document_to_conversation(document: String) {
    log::trace!("In document_to_conversation");

    let chunks = chunk_string(&document, 20000);
    log::debug!("number of chunks: {}", chunks.len());

    let chunk = &chunks[0];

    let rt = Runtime::new().unwrap();

    rt.block_on(async {



        let conversation_parser = get_conversation_parser(chunk).await.unwrap();


        //let conversation_parser_content = ConversationParserContent {
        //    prefix: r##"<div class="comment">
        //          <span class="commtext c00">"##.to_string(),
        //    suffix: r##"</span>
        //      <div class='reply'>"##.to_string(),
        //};

        //let conversation_parser_id = ConversationParserId {
        //    prefix: r##"<tr class='athing comtr' id='"##.to_string(),
        //    suffix: r##"'><td><table border='0'>"##.to_string(),
        //    relative: String::from("before"),
        //};

        //let conversation_parser_parent_id = ConversationParserParentId {
        //    prefix: r##" | <a href="#"##.to_string(),
        //    suffix: r##"" class="clicky" aria-hidden="true">parent</a> |"##.to_string(),
        //    relative: String::from("before"),
        //};

        //let conversation_parser = ConversationParser {
        //    parent_id: conversation_parser_parent_id,
        //    id: conversation_parser_id,
        //    content: conversation_parser_content,
        //};

        log::debug!("{:?}", conversation_parser);









        let start = &conversation_parser.content.prefix;
        log::debug!("{}", start);
        let end = &conversation_parser.content.suffix;
        log::debug!("{}", end);







        let mut conversation_posts = Vec::new();
        let mut current = document.clone();
        let mut start_offset = 0;

        loop {


            let fixed_index = current.char_indices()
                .map(|(i, _)| i)
                .take_while(|&i| i <= start_offset)
                .last()
                .unwrap_or(0);

            let current_slice = &current[fixed_index..];

            if let Some(start_index) = current_slice.find(start) {

                if let Some(end_index) = current[start_offset + start_index..].find(end) {

                    let mut content = &current[start_offset + start_index..start_offset + start_index + end_index];
                    content = &content[start.len()..content.len()];






                    let id_start_index = start_offset + start_index;

                    if conversation_parser.id.relative == "before" {

                        if let Some(id) = search_and_extract(&document, id_start_index, false, &conversation_parser.id.suffix, &conversation_parser.id.prefix) {





                            if conversation_parser.parent_id.relative == "before" {

                                if let Some(parent_id) = search_and_extract(&document, id_start_index, false, &conversation_parser.parent_id.suffix, &conversation_parser.parent_id.prefix) {



                                    let conversation_post = ConversationPost {
                                        parent_id: parent_id.to_string(),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    conversation_posts.push(conversation_post);



                                } else {
                                    let conversation_post = ConversationPost {
                                        parent_id: String::from(""),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    conversation_posts.push(conversation_post);
                                }

                            } else {

                                if let Some(parent_id) = search_and_extract(&document, id_start_index, true, &conversation_parser.parent_id.prefix, &conversation_parser.parent_id.suffix) {

                                    let conversation_post = ConversationPost {
                                        parent_id: parent_id.to_string(),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    conversation_posts.push(conversation_post);



                                } else {
                                    let conversation_post = ConversationPost {
                                        parent_id: String::from(""),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    conversation_posts.push(conversation_post);
                                }

                            }








                        } else {
                            log::error!("Could not find content id");
                        }
                    } else {

                        if let Some(id) = search_and_extract(&document, id_start_index, true, &conversation_parser.id.prefix, &conversation_parser.id.suffix) {





                            if conversation_parser.parent_id.relative == "before" {

                                if let Some(parent_id) = search_and_extract(&document, id_start_index, false, &conversation_parser.parent_id.suffix, &conversation_parser.parent_id.prefix) {



                                    let conversation_post = ConversationPost {
                                        parent_id: parent_id.to_string(),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    conversation_posts.push(conversation_post);



                                } else {
                                    let conversation_post = ConversationPost {
                                        parent_id: String::from(""),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    conversation_posts.push(conversation_post);
                                }

                            } else {

                                if let Some(parent_id) = search_and_extract(&document, id_start_index, true, &conversation_parser.parent_id.prefix, &conversation_parser.parent_id.suffix) {

                                    let conversation_post = ConversationPost {
                                        parent_id: parent_id.to_string(),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    conversation_posts.push(conversation_post);



                                } else {
                                    let conversation_post = ConversationPost {
                                        parent_id: String::from(""),
                                        id: id.to_string(),
                                        content: content.to_string(),
                                    };

                                    conversation_posts.push(conversation_post);
                                }

                            }








                        } else {
                            log::error!("Could not find content id");
                        }
                    }














                    start_offset = start_offset + start_index + end_index + end.len();

                } else {
                    break;
                }
            } else {
                break;
            }
        }

        log::debug!("posts: {}", conversation_posts.len());


        //  * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * *
        //  * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * *
        //  * * * * * * * * * * * * * *

        let final_output = serde_json::to_string(&conversation_posts).expect("Failed to serialize to JSON");
        println!("{}", final_output);

        //  * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * *
        //  * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * * *
        //  * * * * * * * * * * * * * *
    });

}

fn main() {
    simple_logging::log_to_file("debug.log", LevelFilter::Trace);

    let mut document = String::new();

    match load_stdin() {
        Ok(stdin) => {
            document = stdin;
        }
        Err(e) => {
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
            "conversation" => document_to_conversation(document),
            _ => log::error!("Unexpected data type: {}", data_type),
        }

        return;
    } else {
        log::info!("Data type not provided, aborting...");
        return;
    }
}
