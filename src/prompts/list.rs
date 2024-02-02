pub mod patterns {
    pub static PROMPT: &str = r##"
Hi ChatGPT. Please examine the subsequent text and do your best to identify a pattern signifying lists of items of some kind. Similar blocks of text that differ slightly in detail but with an overall similar structure. If you do see lists of items, look for data fields that each item has in common. For example, an item may contain an id, url, title, timestamp, etc. For each of these fields, provide a regular expression that would match the corresponding field value. Print your response based on the following json and add as many keys as you can:
{"id": "id pattern", "url": "url pattern", "title": "title regex pattern", "other key": "regex"}
If you see multiple lists of items, print a json array for each distinct list where the various keys correspond to regular expression patterns. If the text does not contain any list items, print only the text 'false' and nothing else. Please do not include any introduction or final summary in your response. Thank you.
"##;
}
