use serde::{Serialize};

#[derive(Debug)]
#[derive(Serialize)]
#[derive(Clone)]
pub struct ChatParserParentId {
    pub prefix: String,
    pub suffix: String,
    pub relative: String,
}

#[derive(Debug)]
#[derive(Serialize)]
#[derive(Clone)]
pub struct ChatParserId {
    pub prefix: String,
    pub suffix: String,
    pub relative: String,
}

#[derive(Debug)]
#[derive(Serialize)]
#[derive(Clone)]
pub struct ChatParserContent {
    pub prefix: String,
    pub suffix: String,
}

#[derive(Debug)]
#[derive(Serialize)]
#[derive(Clone)]
pub struct ChatParser {
    pub parent_id: ChatParserParentId,
    pub id: ChatParserId,
    pub content: ChatParserContent,
}

#[derive(Debug)]
#[derive(Serialize)]
#[derive(Clone)]
pub struct ChatPost {
    pub parent_id: String,
    pub id: String,
    pub content: String,
}

#[derive(Debug)]
#[derive(Serialize)]
#[derive(Clone)]
pub struct Chat {
    pub posts: Vec<ChatPost>,
}
