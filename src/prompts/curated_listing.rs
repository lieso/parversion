pub static CURATED_LISTING_GROUP_PROMPT: &str = r##"
Hi ChatGPT. Your job is to interpret textual documents and to glean from it patterns that represent the salient information contained within these documents.

Please examine the subsequent text and do your best to identify a pattern signifying curated lists of items like a news aggregator consisting of user generated submissions. Look for blocks of text that have a similar structure, even if the details vary. 

Should you identify any such lists in the text, I want you to provide regular expressions that match the entirety of each list item in the text without using capturing groups. Use as much of the redundant text that precedes or follows each list item as needed to ensure consistency across the matches. Feel free to provide multiple regular expressions if there are list items that do seem to be members of the overall list, but with significantly different text. Please return the regular expressions in JSON array format, with each array item associated with the regex value. The regex should match the full extent of each list item without capturing parts of it.

Here is how the response should be formatted:
[
    "regex pattern goes here",
    "perhaps another pattern here"
]

Make sure to exclude any introductory text or conclusion in your response. Thank you.
"##;

pub static CURATED_LISTING_ITEM_PROMPT: &str = r##"
Hi ChatGPT. Your job is to interpret textual documents and to glean from it patterns that represent the salient information contained within these documents. Please examine the subsequent list of texts and do your best to identify which parts of it pertain to information a person might want to know as opposed to code, formatting or out of context text. Examples of things to search for might include: titles, descriptions, timestamps, authors/users, points or ranking, link to a comments/discussion page, etc. I will provide multiple examples of the text to assist you in identifying when content is static or dynamic. Please provide regular expressions that would capture each distinct field in the text. Include as many different fields as you possibly can. Each regular expression should contain at most one capturing group. If a regular expression happens to overlap across fields, copy the regular expression and change which field gets captured as a matching group. For example, if a regular expression contains both a title and a url, create two fields 'title' and 'url' with the first capturing group corresponding to each one. Ensure all urls (absolute or relative) have a corresponding regular expression, with the word url included as part of the key name. Set the key name to be the name of the field and its value should be the regular expression. Print your response based on the following json:
{
    "title": "title regular expression",
    "url": "main url regular expression",
    ...
},
Ensure there are at least two fields present for the main title and the url. Please do not include any introduction or final summary in your response. Thank you.
"##;
pub static CURATED_LISTING_ITEM_ADAPTER_PROMPT: &str = r##"
Hi ChatGPT. Your task is to map the keys of a JSON document I provide to the corresponding keys of a JSON schema based on their semantic meaning. For context, the JSON document should be representing a curated list item as seen on a news aggregator website. You should follow these guidelines:

 1 Each key from the input JSON document should be mapped to a unique key in the schema below. No duplicates are allowed in the final mapping.
 2 If more than one key from the input document matches a schema key, choose the best fit based on its meaning and usage context and retain the original name for the other keys.
 3 If an input document key doesn't correspond to any schema key, map it to its original name.

Here is the schema for mapping:

 • "title" for the main title of the user submission.
 • "author" for the user account that submitted the content.
 • "id" for the identifier of the item.
 • "points" for the number representing votes a submission has received.
 • "timestamp" for the timestamp, perhaps relative or absolute.
 • "chat_url" for the link to user discussion of content.
 • "url" for the main url that the user submitted.

Please map the provided JSON document keys to the given schema without including any additional commentary or summary. Thank you.

The provided JSON document is:
"##;

pub fn self_improve_list_group_pattern(bad_pattern: &str, document: &str) -> String {
    format!(r##"
Hi ChatGPT. Please examine the following regular expression:

{}

It doesn't seem to result in the correct number of matches against the document I will subsequently provide you below. Please do your best to improve the regular expression to ensure it matches the document. Print only the improved regular expression in your response without including any additional commentary or summary. Thank you.

The document is:

{}

"##, bad_pattern, document)
}
