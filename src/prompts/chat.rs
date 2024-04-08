pub static CHAT_GROUP_PROMPT: &str = r##"
Hi ChatGPT. Your job is to interpret textual documents and to glean from it patterns that represent the salient information contained within these documents. Please examine the subsequent text and do your best to identify a pattern signifying posts/comments that people leave on websites such as discussion forums. These items are typically user generated, contain information like author, a body of text content, timestamp (relative or absolute), might contain points, parent and child comments as part of a larger thread and potentially much more. If you do see a set of distinct discussion posts, provide a regular expression that would capture each user generated post in the text. Do not provide an optimized regular expression, include as much redundant text that precedes or follows each post. Ensure that the regular expression matches across newline characters. Since dot-all mode or inline modifiers like (?s:.) are not supported, use a character class such as [.\n] or [\s\S] for this purpose. Additionally, avoid using lookahead and lookbehind constructs, as they may not be compatible with the regex engine in use. Print your response based on the following json:
{
    "pattern": "regex pattern goes here"
}
Please do not include any introduction or final summary in your response. Thank you.
"##;
pub static CHAT_ITEM_PROMPT: &str = r##"
Hi ChatGPT. Your job is to interpret textual documents and to glean from it patterns that represent the salient information contained within these documents. Please examine the subsequent set of texts that we can expect to represent comments left on dicussion forums. These items often come from instant message tools, collaboration software, team chat, messaging apps, discussion forums, etc, where users can start new threads on a particular topic and engage in conversations with each other by posting messages. They may contain information representing whether they have a parent comment identifier. Very often there will be user/author information, a timestamp (relative or absolute), and potentially other fields. A comment will always have a body of text, so make sure to at least provide a regular expression for this key piece of information. Please identify common fields across all chat items and create regular expressions that would capture each field. I will provide multiple examples of the text to assist you in identifying when fields is static or dynamic. Each regular expression should contain at most one capturing group. Ensure regular expressions are not optimized and are as long as possible to ensure there is at most one match. If multiple elements need to be captured, provide separate regular expressions for each, ensuring only one capturing group is present in each expression. Ensure all urls (absolute or relative) have a corresponding regular expression, except for those in the text body of user-generated content. Set the key name to be the name of the field and its value should be the regular expression. Ensure that the regular expression matches across newline characters. Since dot-all mode or inline modifiers like (?s:.) are not supported, use a character class such as [.\n] or [\s\S] for this purpose. Additionally, avoid using lookahead and lookbehind constructs, as they may not be compatible with the regex engine in use. Print your response based on the following json:
{
    "fieldName": "regex pattern goes here",
    "otherField": "regex pattern goes here",
    ...
},
Please do not include any introduction or final summary in your response. Thank you.
"##;
pub static CHAT_ITEM_ADAPTER_PROMPT: &str = r##"
Hi ChatGPT. Your task is to map the keys of a JSON document I provide to the corresponding keys of a JSON schema based on their semantic meaning. You should follow these guidelines:

 1 Each key from the input JSON document should be mapped to a unique key in the schema below. No duplicates are allowed in the final mapping.
 2 If more than one key from the input document matches a schema key, choose the best fit based on its meaning and usage context and retain the original name for the other keys.
 3 If an input document key doesn't correspond to any schema key, map it to its original name.
 4 The JSON keys in your response should be the keys of the JSON document I will provide you, and the values should be the mapped key based on the schema.

Here is the schema for mapping:

 • "text" for the body of comments or messages.
 • "author" for who created the comment or message.
 • "id" for unique identifiers of the comments or messages.
 • "parent_id" for identifiers linking to the immediate parent of the comment in a thread.
 • "timestamp" for when the comment or message was made.

Please map the provided JSON document keys to the given schema without including any additional commentary or summary. Thank you.

The provided JSON document is:
"##;
