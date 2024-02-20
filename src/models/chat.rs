use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatParserParentId {
    pub prefix: String,
    pub suffix: String,
    pub relative: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatParserId {
    pub prefix: String,
    pub suffix: String,
    pub relative: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatParserContent {
    pub prefix: String,
    pub suffix: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatParser {
    pub parent_id: ChatParserParentId,
    pub id: ChatParserId,
    pub content: ChatParserContent,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatPost {
    pub parent_id: String,
    pub id: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Chat {
    pub posts: Vec<ChatPost>,
}
