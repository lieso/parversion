pub mod patterns {
    pub static PROMPT: &str = r##"
Hi ChatGPT. Your job is to interpret textual documents and to glean from it patterns that represent the salient information contained within these documents. Please examine the subsequent text and do your best to identify a pattern signifying lists of items of some kind. Similar blocks of text that differ slightly in detail but with an overall similar structure. If you do see lists of items, look for data fields that each item has in common. For example, an item may contain an id, url, title, timestamp and an assortment of various other fields. For each of these fields, provide a regular expression that would match the corresponding field value. Additionally, add to these regular expressions some common text that precedes or follows each field you identify in these lists, including as much text as you find each field has in common. For example if an item in a list contains a title and your regular expression matches all titles, also include all text that comes before or after 'title' fields in all list items. Do not provide an optimized regular expression, include as much redundant text that comes before or after all list item fields. If a list item contains a url, do not just provide a regular expression for urls, also include fixed strings that come before or after all url fields. Please also select one block of text and return it in your response as the "example". Print your response based on the following json, but replace the keys with all data fields that you identify. Please include the maximum number of common data fields you can see:
{
    "patterns": {
        "id": "id pattern",
        "url": "url pattern",
        "title": "title regex pattern"
    },
    "example": "example list item original text"
}
If you see multiple lists of items, print a json array for each distinct list where the various keys correspond to regular expression patterns. If the text does not contain any list items, print only the text 'false' and nothing else. Please do not include any introduction or final summary in your response. Thank you.
"##;
    pub static CHAT_REF_PROMPT: &str = r##"
Hi ChatGPT. Your job is to process and interpret text. Please examine the subsequent text and do your best to see if it contains a link to a document that is expected to contain a discussion forum, comments, or any kind of chat content. If you do identify a link to such content, please provide a regular expression that would capture the link to this content. Print your response based on the following json:
{
    "chat": "url pattern goes here"
}
Please do not include any introduction of final summary in your response. Thank you.
"##;
    pub static LIST_GROUP_PROMPT: &str = r##"
Hi ChatGPT. Your job is to interpret textual documents and to glean from it patterns that represent the salient information contained within these documents. Please examine the subsequent text and do your best to identify a pattern signifying lists of items of some kind. Similar blocks of text that differ slightly in detail but with an overall similar structure. When delineating where a list item begins and ends, try to interpret the content itself and see if it makes sense to group text under one list item within a larger context. If you do see lists of items, provide a regular expression that would capture each list item in the text. Do not provide an optimized regular expression, include as much redundant text that precedes or follows each list item. Print your response based on the following json:
{
    "pattern": "regex pattern goes here"
}
If the text does not contain any lists, print only the text 'false' and nothing else. Please do not include any introduction or final summary in your response. Thank you.
"##;
    pub static LIST_ITEM_PROMPT: &str = r##"
Hi ChatGPT. Your job is to interpret textual documents and to glean from it patterns that represent the salient information contained within these documents. Please examine the subsequent list of texts and do your best to identify which parts of it pertain to information a person might want to know as opposed to code, formatting or out of context text. Examples of things to search for might include: titles, descriptions, timestamps, authors/users, links to websites or comments, etc. Try to infer the broader context the text implies and search for salient information relevant to this context. For example, if the text seems to be a weather forecast, try to find minimum and maximum temperatures in the text. I will provide multiple examples of the text to assist you in identifying when content is static or dynamic. Having established the context and having identified the salient content, please provide regular expressions that would capture each distinct field in the text. Each regular expression should contain at most one capturing group. If multiple elements need to be captured, provide separate regular expressions for each, ensuring only one capturing group is present in each expression. Ensure all urls (absolute or relative) have a corresponding regular expression. Set the key name to be the name of the field and its value should be the regular expression. Print your response based on the following json:
{
    "fieldName": "regex pattern goes here",
    "otherField": "regex pattern goes here",
    ...
},
Please do not include any introduction or final summary in your response. Thank you.
"##;
}
