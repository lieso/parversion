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
pub static CURATED_LISTING_ITEM_ADAPTER_PROMPT: &str = r##"
Hi ChatGPT. Your job is to map the keys of a JSON document I will provide you to a new set of keys. The keys of the JSON I will ultimately provide should represent an item from a curated lists of items like a news aggregator consisting of user generated submissions. Please try to match the keys of the JSON I will ultimately provide to one of the keys of this JSON guide, and set the blank value corresponding to each key to a key of the JSON guide. Match keys based on them probably referring to the same thing but with a different name.
JSON guide:
{
    "title": "The main title of the user submission",
    "author": "The user account that submitted the content",
    "id": "The identifier of the item",
    "points": "Number representing votes a submission has received",
    "timestamp": "The timestamp, perhaps relative or absolute",
    "chatLink": "A link to user discussion of content",
    "url": "The main url that the user submitted"
}
For example, if a key I provide is called "user", set its value to "author" as per the above guide, since these things are roughly equivalent. The values of this JSON guide provide  more information about how to match the keys of the JSON document. Ensure that the set of values you map keys to contains no duplicate values. For example if multiple keys seem to map to "url", only select one key to map to "url" and leave the others to map to their original values. If multiple keys seem to match above guide, use the most probably match and set the other keys to their original name. If the JSON document I provide contains keys that do not correspond to the above template, set the value to the original key name.
Please do not include any introduction or final summary in your response. Thank you.

JSON document:
"##;
