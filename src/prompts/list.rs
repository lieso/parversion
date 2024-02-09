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
}
