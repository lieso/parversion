extern crate html5ever;
extern crate markup5ever_rcdom;

use html5ever::driver::ParseOpts;
use markup5ever_rcdom as rcdom;
use html5ever::serialize::{SerializeOpts, Serializer, TraversalScope};
use html5ever::{parse_document, serialize};
use markup5ever_rcdom::{SerializableHandle};
use html5ever::tendril::TendrilSink;
use std::io;
use std::default::Default;
use std::string::String;
use ammonia::Builder;
use maplit::hashset;

pub fn is_valid_html(html_string: &str) -> bool {
    let parser = parse_document(rcdom::RcDom::default(), ParseOpts::default());

    let dom = parser.one(html_string);
    log::debug!("dom.errors: {:?}", dom.errors);

    if !dom.errors.is_empty() {
        for error in &dom.errors {
            log::debug!("Error: {}", error);
         }
    }

    dom.errors.is_empty()
}

pub fn html_to_xhtml(html: &str) -> io::Result<String> {
    log::debug!("html: {}", html);
    let xhtml = remove_doctype(&html);
    //let xhtml = Builder::new()
    //    .clean(&xhtml)
    //    .to_string();



    log::warn!("NOT IMPLEMENTED. TAGS ARE NOT CLOSED.");




    Ok(xhtml)
}

pub fn remove_doctype(html: &str) -> String {
    let doctype_pattern = regex::Regex::new(r"(?i)<!DOCTYPE\s+[^>]*>").unwrap();
    doctype_pattern.replace(html, "").to_string()
}
