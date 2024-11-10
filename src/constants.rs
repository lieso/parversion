use serde::{Serialize, Deserialize};

// echo -n "text" | sha256sum
pub const TEXT_NODE_HASH: &str = "982d9e3eb996f559e633f4d194def3761d909f5a3b647d1a851fead67c32c9d1";
// echo -n "root" | sha256sum
pub const ROOT_NODE_HASH: &str = "4813494d137e1631bba301d5acab6e7bb7aa74ce1185d456565ef51d737677b2";

pub const UNSEEN_BLACKLISTED_ATTRIBUTES: &[&str] = &[
    "style", "bgcolor", "border", "cellpadding", "cellspacing",
    "width", "height", "rows", "cols", "wrap",
    "aria-hidden", "size", "op", "lang", "colspan", "rel"
];

pub const UNSEEN_BLACKLISTED_ELEMENTS: &[&str] = &[
    "script", "meta", "link", "iframe", "svg", "style", "noscript"
];

pub const SEEN_BLACKLISTED_ELEMENTS: &[&str] = &[
    "head", "body", "br", "form"
];

pub const MAX_CONCURRENCY: usize = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LlmProvider {
    openai,
    anthropic
}
