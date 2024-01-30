use std::io::{Error, ErrorKind};
use std::io;

use crate::models;

pub fn transform_document_to_chat(document: String, parser: models:: ChatParser) -> models::Chat {
    log::trace!("In transform_document_to_chat");
    
    let content_prefix = &parser.content.prefix;
    log::debug!("{}", content_prefix);
    let content_suffix = &parser.content.suffix;
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


        let start_index: usize;
        let end_index: usize;
        let content: String;

        match find_next_content(current_slice, content_prefix, content_suffix) {
            Some(result) => {
                log::info!("Found content in document");
                content = result.0;
                start_index = result.1;
                end_index = result.2;
            },
            None => {
                log::info!("No more content found in document");
                break;
            },
        } 


        let id: String;
        let id_search_begin_index = start_offset + start_index;


        match find_next_id(&document, id_search_begin_index, &parser.id.relative, &parser.id.prefix, &parser.id.suffix) {
            Some(result) => {
                log::info!("Found content id in document");
                id = result;
            },
            None => {
                log::warn!("Unable to find content id");
                break;
            },
        }





        if parser.parent_id.relative == "before" {

            if let Some(parent_id) = search_and_extract(&document, id_search_begin_index, false, &parser.parent_id.suffix, &parser.parent_id.prefix) {



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

            if let Some(parent_id) = search_and_extract(&document, id_search_begin_index, true, &parser.parent_id.prefix, &parser.parent_id.suffix) {

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












        start_offset = start_offset + start_index + end_index + content_suffix.len();

    }

    let chat = models::Chat {
        posts: chat_posts,
    };

    return chat;
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

fn find_next_content(document: &str, prefix: &String, suffix: &String) -> Option<(String, usize, usize)> {
    log::trace!("In find_next_content");

    if let Some(start_index) = document.find(prefix) {
        log::debug!("Found content start index");

        if let Some(end_index) = document[start_index..].find(suffix) {
            log::debug!("Found content end index");

            let mut content = &document[start_index..start_index + end_index];
            content = &content[prefix.len()..content.len()];

            return Some((content.to_string(), start_index, end_index));
        } else {
            return None;
        }
    } else {
        return None;
    }
}

fn find_next_id(document: &str, begin_index: usize, relative: &String, prefix: &String, suffix: &String) -> Option<String> {
    log::trace!("In find_next_id");

    if relative == "before" {
        if let Some(id) = search_and_extract(&document, begin_index, false, suffix, prefix) {
            return Some(id.to_string());
        } else {
            return None;
        }
    } else {
        if let Some(id) = search_and_extract(&document, begin_index, true, prefix, suffix) {
            return Some(id.to_string());
        } else {
            return None;
        }
    }
}
