pub mod id {
    pub static PROMPT: &str = r##"
Hi ChatGPT. Please examine the subsequent text and do your best to identify posts/comments like that people leave on websites such as discussion forums. If present, extract
the common text that directly precedes and follows the identifier (id) associated with the post content, as the prefix and suffix, respectively. Additionally, determine whet
her the identifier comes 'before' or 'after' the post content and label this value 'relative'. If you do not see any posts in the text, respond only with the digit '0'. Othe
rwise print your response based on the following json:
{"prefix":"prefix string","suffix":"suffix string","relative": "before or after"}
When populating the prefix or suffix string, ensure newline escape characters are double-escaped. Do not include triple-backticks or anything signifying a code block. Please
do not include any introduction or final summary in your response. Thank you.
"##;
}
pub mod parent_id {
    pub static PROMPT: &str = r##"
Hi ChatGPT. Please examine the subsequent text and do your best to identify posts/comments like that people leave on websites such as discussion forums. If present, try to then see if these posts have a parent id like when a person replies to another post. If these parent references are present, extract the common text that directly precedes and follows a post's parent identifier (id) associated with the post content, as the prefix and suffix, respectively. Additionally, determine whether the parent id comes 'before' or 'after' the post content and label this value 'relative'. If you do not see any posts in the text, respond only with the digit '0'. Otherwise print your response based on the following json:
{"prefix":"prefix string","suffix":"suffix string","relative": "before or after"}
When populating the prefix or suffix string, ensure newline escape characters are double-escaped. Do not include triple-backticks or anything signifying a code block.
Please do not include any introduction or final summary in your response. Thank you.
"##;
}
pub mod content {
    pub static PROMPT: &str = r##"
Hi ChatGPT. Please examine the subsequent text and do your best to identify posts/comments like that people leave on websites such as discussion forums. If present, extract the common text that directly precedes the post content (prefix), and also the common text that immediately follows the post content (suffix). If you do not see any posts in the text, respond only with the digit '0'. Otherwise print your response based on the following json:
{"prefix":"prefix string","suffix":"suffix string"}
When populating the prefix or suffix string, ensure newline escape characters are double-escaped. Do not include triple-backticks or anything signifying a code block. Please do not include any introduction or final summary in your response. Thank you.
"##;
}

pub static CHAT_GROUP_PROMPT: &str = r##"
Hi ChatGPT. Your job is to interpret textual documents and to glean from it patterns that represent the salient information contained within these documents. Please examine the subsequent text and do your best to identify a pattern signifying posts/comments that people leave on websites such as discussion forums. These items are typically user generated, contain information like author, a body of text content, timestamp (relative or absolute), might contain points, parent and child comments as part of a larger thread and potentially much more. If you do see a set of distinct discussion posts, provide a regular expression that would capture each user generated post in the text. Do not provide an optimized regular expression, include as much redundant text that precedes or follows each post. Print your response based on the following json:
{
    "pattern": "regex pattern goes here"
}
If the text does not contain any user discussion, print only the text 'false' and nothing else. Please do not include any introduction or final summary in your response. Thank you.
"##;
