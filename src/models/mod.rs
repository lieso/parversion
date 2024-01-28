use serde::{Serialize};

#[derive(Debug)]
pub struct ConversationParserParentId {
    pub prefix: String,
    pub suffix: String,
    pub relative: String,
}

#[derive(Debug)]
pub struct ConversationParserId {
    pub prefix: String,
    pub suffix: String,
    pub relative: String,
}

#[derive(Debug)]
pub struct ConversationParserContent {
    pub prefix: String,
    pub suffix: String,
}

#[derive(Debug)]
pub struct ConversationParser {
    pub parent_id: ConversationParserParentId,
    pub id: ConversationParserId,
    pub content: ConversationParserContent,
}

#[derive(Serialize)]
pub struct ConversationPost {
    pub parent_id: String,
    pub id: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct Conversation {
    pub posts: Vec<ConversationPost>,
}
