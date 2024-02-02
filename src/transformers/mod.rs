use crate::models;

pub fn transform_document_to_chat(document: String, parser: models::chat::ChatParser) -> models::chat::Chat {
    log::trace!("In transform_document_to_chat");
    
    let content_prefix = &parser.content.prefix;
    log::debug!("{}", content_prefix);
    let content_suffix = &parser.content.suffix;
    log::debug!("{}", content_suffix);

    let mut chat_posts = Vec::new();
    let current = document.clone();
    let mut start_offset = 0;

    let mut previous_content_index: usize = 0;

    loop {
        // Fix Rust utf boundary issue when slicing with offset
        let fixed_index = current.char_indices()
            .map(|(i, _)| i)
            .take_while(|&i| i <= start_offset)
            .last()
            .unwrap_or(0);

        let current_slice = &current[fixed_index..];

        // =====================================================================================
        // Content extraction
        // =====================================================================================

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

        // =====================================================================================
        // ID extraction
        // =====================================================================================

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

        // =====================================================================================
        // Parent ID extraction
        // =====================================================================================

        let parent_id: String;
        let id_search_max_index: usize;

        if &parser.parent_id.relative == "before" {
            id_search_max_index = previous_content_index;
        } else {


            let next_fixed_index = current.char_indices()
                .map(|(i, _)| i)
                .take_while(|&i| i <= start_offset + start_index + end_index + content_suffix.len())
                .last()
                .unwrap_or(0);
            let next_slice = &current[next_fixed_index..];

            match find_next_content(next_slice, content_prefix, content_suffix) {
                Some(result) => {
                    // TODO: not sure about this
                    id_search_max_index = result.1 + start_index;
                },
                None => {
                    id_search_max_index = current_slice.len();
                },
            }


        }

        match find_next_parent_id(&document, id_search_begin_index, id_search_max_index, &parser.parent_id.relative, &parser.parent_id.prefix, &parser.parent_id.suffix) {
            Some(result) => {
                log::info!("Content has a parent");
                parent_id = result;
            },
            None => {
                log::info!("Content does not have parent");
                parent_id = "".to_string();
            },
        }



        let chat_post = models::chat::ChatPost {
            parent_id: parent_id,
            id: id,
            content: content,
        };

        chat_posts.push(chat_post);

        start_offset = start_offset + start_index + end_index + content_suffix.len();



        previous_content_index = start_index;
    }

    let chat = models::chat::Chat {
        posts: chat_posts,
    };

    return chat;
}

fn search_and_extract<'a>(
    document: &'a str,
    index: usize,
    search_forward: bool,
    target_substring: &str,
    delimiter_substring: &str,
    max_index_option: Option<usize>
) -> Option<&'a str> {
    if search_forward {
        if let Some(start_pos) = document[index..].find(target_substring) {

            match max_index_option {
                Some(max_index) => {
                    if start_pos > max_index {
                        log::debug!("start_pos is greater than max index");
                        return None;
                    }
                },
                None => {
                    // no-op
                }
            }

            let start_pos = start_pos + index + target_substring.len();
            if let Some(end_pos) = document[start_pos..].find(delimiter_substring) {
                let end_pos = start_pos + end_pos;
                return Some(&document[start_pos..end_pos]);
            }
        }
    } else {
        if let Some(end_pos) = document[..index].rfind(target_substring) {

            match max_index_option {
                Some(max_index) => {
                    if end_pos < max_index {
                        log::debug!("end_pos is less than max index");
                        return None;
                    }
                },
                None => {
                    // no-op
                }
            }


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
        if let Some(id) = search_and_extract(&document, begin_index, false, suffix, prefix, None) {
            return Some(id.to_string());
        } else {
            return None;
        }
    } else {
        if let Some(id) = search_and_extract(&document, begin_index, true, prefix, suffix, None) {
            return Some(id.to_string());
        } else {
            return None;
        }
    }
}

fn find_next_parent_id(document: &str, begin_index: usize, max_index: usize, relative: &String, prefix: &String, suffix: &String) -> Option<String> {
    log::trace!("In find_next_parent_id");

    if relative == "before" {
        if let Some(id) = search_and_extract(&document, begin_index, false, suffix, prefix, Some(max_index)) {
            return Some(id.to_string());
        } else {
            return None;
        }
    } else {
        if let Some(id) = search_and_extract(&document, begin_index, true, prefix, suffix, Some(max_index)) {
            return Some(id.to_string());
        } else {
            return None;
        }
    }
}
