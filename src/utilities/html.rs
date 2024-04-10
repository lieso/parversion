extern crate html5ever;
extern crate markup5ever_rcdom;

use html5ever::driver::ParseOpts;
use markup5ever_rcdom as rcdom;
use html5ever::{parse_document, serialize};
use markup5ever_rcdom::{Handle, RcDom, SerializableHandle};
use html5ever::tendril::TendrilSink;
use std::io;
use std::default::Default;
use std::string::String;

pub fn is_valid_html(html_string: &str) -> bool {
    let parser = parse_document(rcdom::RcDom::default(), ParseOpts::default());

    let dom = parser.one(html_string);
    log::debug!("dom.errors: {:?}", dom.errors);

    dom.errors.len() == 0
}

pub fn html_to_xhtml(html: &str) -> io::Result<String> {
    let parser = parse_document(rcdom::RcDom::default(), ParseOpts::default());

    let dom = parser.one(html);

    let mut result_bytes = Vec::new();
    serialize(
        &mut result_bytes,
        &SerializableHandle::from(dom.document.clone()),
        Default::default(),
    )?;

    let result_string =
        String::from_utf8(result_bytes).expect("Serialization produced a non-utf8 result");
    Ok(result_string)
}
