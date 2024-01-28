pub mod chat {
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
}
