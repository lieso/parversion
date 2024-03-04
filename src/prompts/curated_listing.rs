pub static CURATED_LISTING_GROUP_PROMPT: &str = r##"
Hi ChatGPT. Your job is to interpret textual documents and to glean from it patterns that represent the salient information contained within these documents. Please examine the subsequent text and do your best to identify a pattern signifying curated lists of items like a news aggregator consisting of user generated submissions. Similar blocks of text that differ slightly in detail but with an overall similar structure. If you do see lists of items, provide a regular expression that would capture each list item in the text. Do not provide an optimized regular expression, include as much redundant text that precedes or follows each list item. Print your response based on the following json:
{
    "pattern": "regex pattern goes here"
}
If the text does not contain any lists, print only the text 'false' and nothing else. Please do not include any introduction or final summary in your response. Thank you.
"##;
pub static CURATED_LISTING_ITEM_PROMPT: &str = r##"
Hi ChatGPT. Your job is to interpret textual documents and to glean from it patterns that represent the salient information contained within these documents. Please examine the subsequent list of texts and do your best to identify which parts of it pertain to information a person might want to know as opposed to code, formatting or out of context text. Examples of things to search for might include: titles, descriptions, timestamps, authors/users, points or ranking, links to websites or comments, etc. I will provide multiple examples of the text to assist you in identifying when content is static or dynamic. Please provide regular expressions that would capture each distinct field in the text. Each regular expression should contain at most one capturing group. If multiple elements need to be captured, provide separate regular expressions for each, ensuring only one capturing group is present in each expression. Ensure all urls (absolute or relative) have a corresponding regular expression. Set the key name to be the name of the field and its value should be the regular expression. Print your response based on the following json:
{
    "fieldName": "regex pattern goes here",
    "otherField": "regex pattern goes here",
    ...
},
Please do not include any introduction or final summary in your response. Thank you.
"##;
