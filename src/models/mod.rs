use serde::{Serialize};

#[derive(Debug)]
pub struct ChatParserParentId {
    pub prefix: String,
    pub suffix: String,
    pub relative: String,
}

#[derive(Debug)]
pub struct ChatParserId {
    pub prefix: String,
    pub suffix: String,
    pub relative: String,
}

#[derive(Debug)]
pub struct ChatParserContent {
    pub prefix: String,
    pub suffix: String,
}

#[derive(Debug)]
pub struct ChatParser {
    pub parent_id: ChatParserParentId,
    pub id: ChatParserId,
    pub content: ChatParserContent,
}

#[derive(Debug)]
#[derive(Serialize)]
pub struct ChatPost {
    pub parent_id: String,
    pub id: String,
    pub content: String,
}

#[derive(Debug)]
#[derive(Serialize)]
pub struct Chat {
    pub posts: Vec<ChatPost>,
}
