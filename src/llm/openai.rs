use std::sync::{Arc};
use serde::{Serialize, Deserialize};
use reqwest::header;
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::prelude::*;
use crate::transformation::{FieldTransformation, FieldMetadata};
#[cfg(feature = "caching")]
use crate::cache::Cache;
use crate::environment::{get_env_variable};

pub struct OpenAI;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct EliminationResponse {
    pub is_unmeaningful: bool,
    pub justification: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct PrimaryResponse {
    pub name: String,
    pub description: String,
    pub justification: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AssociationsResponse {
    pub name: String,
    pub description: String,
    pub matching_fragments: Vec<String>,
    pub justification: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct SummaryResponse {
    pub category: String,
    pub description: String,
    pub structure: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct NormalResponse {
    pub new_property_name: String,
    pub alternative_property_names: Vec<String>,
    pub description: String,
    pub justification: String,
    pub json_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct MatchSchemaNodeResponse {
    pub json_path: Option<String>,
    pub source_path: String,
    pub target_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ShouldEliminateCodeResponse {
    pub is_query_or_mutation: bool,
    pub justification: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CodeToHttpResponseHeaders {
    pub content_type: String,
    pub accept: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CodeToHttpResponse {
    method: String,
    url: String,
    headers: CodeToHttpResponseHeaders,
    query_params: Vec<String>,
    body: Vec<String>,
}

impl OpenAI {
    pub async fn function_to_operation(code: &str) -> Result<Option<()>, Errors> {
        log::trace!("In function_to_operation");

        if code.len() > 20000 {
            log::debug!("{}", code);
            return Err(Errors::ContextTooLarge);
        }

        let eliminate_response = Self::should_eliminate_code(&code).await?;

        log::debug!("eliminate_response: {:?}", eliminate_response);

        if !eliminate_response.is_query_or_mutation {
            log::info!("Function is not a query or mutation");
            return Ok(None);
        }

        let operation_response = Self::code_to_http(&code).await?;

        log::debug!("operation_response: {:?}", operation_response);

        if operation_response.url.len() == 0 {
            log::warn!("Unexpected blank operation");
            
            // TODO
            /*
            function fn() {
                const v0 = document.createElement("link").relList;
                if (v0 && v0.supports && v0.supports("modulepreload")) return;
                for (const j of document.querySelectorAll('link[rel="modulepreload"]'))o(j);
                new MutationObserver((j)=>{
                    for (const w of j)if (w.type === "childList") for (const sl of w.addedNodes)sl.tagName === "LINK" && sl.rel === "modulepreload" && o(sl);
                }).observe(document, {
                    childList: !0,
                    subtree: !0
                });
                function $(p0) {
                    const v0 = {};
                    return p0.integrity && (v0.integrity = p0.integrity), p0.referrerPolicy && (v0.referrerPolicy = p0.referrerPolicy), p0.crossOrigin === "use-credentials" ? v0.credentials = "include" : p0.crossOrigin === "anonymous" ? v0.credentials = "omit" : v0.credentials = "same-origin", v0;
                }
                function o(p0) {
                    if (p0.ep) return;
                    p0.ep = !0;
                    const v0 = $(p0);
                    fetch(p0.href, v0);
                }
            }

            The function contains a call to fetch(p0.href, v0), which is a standard JavaScript method for making HTTP requests to a remote server. This is a direct query or mutation operation, as it sends a request to the URL specified by p0.href, potentially retrieving or modifying data on a remote server.
            */

            return Ok(None);
        }






        unimplemented!()
    }

    pub async fn match_schema_nodes(
        marked_schema_node: &String,
        target_schema: Arc<String>
    ) -> Result<(Option<String>, Option<String>, Option<String>), Errors> {
        log::trace!("In match_schema_nodes");

        if marked_schema_node.len() > 10000 || target_schema.len() > 10000 {
            return Err(Errors::ContextTooLarge);
        }

        let system_prompt = format!(r##"
Your task to compare two JSON schemas and attempt to match a target schema field from the first with the second, if there is an appropriate equivalent.

The first JSON schema will be an incomplete snippet, and the schema field to match against will be found inside delimiter strings:
START TARGET SCHEMA KEY >>>
<<< END TARGET SCHEMA KEY

Provide three pieces of information:

1. (json_path): A JSON path against the JSON schema of the second schema indicating which field is equivalent to the target schema field, or null if there is no equivalent. The JSON path should be relative to the JSON schema itself, not the resulting JSON document.

2. (source_path): The path in the source document structure where data values are found. Leave as empty string if you cannot determine an appropriate path.

3. (target_path): The path in the target document structure where data values should be placed. Leave as empty string if you cannot determine an appropriate path.

   Path mapping rules:
   - Use dot notation for object properties (e.g., invoice.date)
   - Use bracket notation with concrete indices for specific array positions (e.g., items[0], items[1])
   - Use bracket notation with variable names for array indices that should be preserved/mapped between source and target (e.g., [x], [y], [z])
   - You can use any single-letter variable names you need (a, b, c, ..., x, y, z, etc.)
   - Each unique array index position that needs to be mapped gets its own variable
   - The same variable in source_path and target_path means "preserve this array index correspondence"
   - If you cannot determine an appropriate mapping for either side, leave that field as an empty string

For example, if the first JSON schema is this, representing an invoice:
{{
  "title": "Invoice",
  "type": "object",
  "properties": {{
    "invoiceNumber": {{
      "type": "string",
      "description": "Unique identifier for the invoice"
    }},
    "START TARGET SCHEMA KEY>>>date<<< END TARGET SCHEMA KEY": {{
      "type": "string",
      "format": "date",
      "description": "Date when the invoice was issued"
    }},
    "dueDate": {{
      "type": "string",
      "format": "date",
      "description": "Date by which the invoice should be paid"
    }}
  }}
}}

And the second JSON schema is this:
{{
   "title": "Invoice",
   "type": "object",
   "properties": {{
     "id": {{
       "type": "string",
       "description": "Unique identifier for the invoice"
     }},
     "issueDate": {{
       "type": "string",
       "format": "date",
       "description": "Date when the invoice was issued"
     }},
     "paymentDue": {{
       "type": "string",
       "format": "date",
       "description": "Date by which the payment should be completed"
     }}
   }}
 }}

Your response should include:
- json_path: '$.properties.issueDate' (since the 'date' field matches the 'issueDate' field)
- source_path: 'date' (path to the data value in source document)
- target_path: 'issueDate' (path to the data value in target document)

Please also provide a justification for your response.
        "##);
        let user_prompt = format!(r##"
[FIRST JSON SCHEMA]:
{}

[SECOND JSON SCHEMA]:
{}
        "##, marked_schema_node, target_schema);

        let response_format = json!({
            "type": "json_schema",
            "name": "match_schema",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "json_path": {
                        "type": "string"
                    },
                    "source_path": {
                        "type": "string"
                    },
                    "target_path": {
                        "type": "string"
                    },
                    "justification": {
                        "type": "string"
                    }
                },
                "required": ["json_path", "source_path", "target_path", "justification"],
                "additionalProperties": false
            }
        });

        match Self::send_openai_request::<MatchSchemaNodeResponse>(&system_prompt, &user_prompt, response_format).await {
            Ok(response) => {
                log::debug!("╔═════════════════════════════════╗");
                log::debug!("║       TRANSLATE SCHEMA START    ║");
                log::debug!("╚═════════════════════════════════╝");

                log::debug!("***system_prompt***\n{}", system_prompt);
                log::debug!("***user_prompt***\n{}", user_prompt);
                log::debug!("***response***\n{:?}", response);

                log::debug!("╔══════════════════════════════╗");
                log::debug!("║       TRANSLATE SCHEMA END   ║");
                log::debug!("╚══════════════════════════════╝");

                let source_path = if response.source_path.is_empty() {
                    None
                } else {
                    Some(response.source_path)
                };

                let target_path = if response.target_path.is_empty() {
                    None
                } else {
                    Some(response.target_path)
                };

                Ok((response.json_path, source_path, target_path))
            }
            Err(e) => {
                log::error!("Failed to get response from OpenAI: {}", e);
                Err(Errors::UnexpectedError)
            }
        }
    }

    pub async fn get_normal_schema(
        marked_schema: &String
    ) -> Result<(String, String, Vec<String>, String), Errors> {
        log::trace!("In get_normal_schema");

        if marked_schema.len() > 10000 {
            log::error!("Schema is over 10000 characters...");
            return Err(Errors::ContextTooLarge);
        }

        let system_prompt = format!(r##"
Your task is to analyze a particular property in a JSON schema, with respect to the overall schema, and to offer an alternative, streamlined, more generalizable property name that may be used instead. The property name should be appropriate considering the overall context of the JSON schema and the resources it represents.

Additionally, you must provide a JSON path (excluding the new property name), for an imagined alternative schema representing that same resource in the current schema, but more streamlined and generalizable.

The property to analyze will be found inside delimiter strings:
START TARGET SCHEMA KEY >>>
<<< END TARGET SCHEMA KEY

You may return the current property name and JSON path if you think it's already very appropriate.

In addition to the new property name:
1. (alternative_property_names): please also suggest a few alternatives to the new property name you suggest, if you think there are any.
2. (description): suggest a more appropriate json schema description for the target schema key/property.
3. (json_path): the JSON path, excluding the new property name, for an alternative, more generic schema this property may be found in.
4. (justification): provide a justification for your response
        "##);
        let user_prompt = format!(r##"Schema: {}"##, marked_schema);

        let response_format = json!({
            "type": "json_schema",
            "name": "schema_normalize",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "new_property_name": {
                        "type": "string"
                    },
                    "alternative_property_names": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        }
                    },
                    "description": {
                        "type": "string"
                    },
                    "json_path": {
                        "type": "string"
                    },
                    "justification": {
                        "type": "string"
                    }
                },
                "required": ["new_property_name", "alternative_property_names", "description", "json_path", "justification"],
                "additionalProperties": false
            }
        });

        match Self::send_openai_request::<NormalResponse>(&system_prompt, &user_prompt, response_format).await {
            Ok(response) => {
                log::debug!("╔══════════════════════════════╗");
                log::debug!("║       NORMAL SCHEMA START    ║");
                log::debug!("╚══════════════════════════════╝");

                log::debug!("***system_prompt***\n{}", system_prompt);
                log::debug!("***user_prompt***\n{}", user_prompt);
                log::debug!("***response***\n{:?}", response);

                log::debug!("╔═══════════════════════════╗");
                log::debug!("║       NORMAL SCHEMA END   ║");
                log::debug!("╚═══════════════════════════╝");

                Ok((response.new_property_name, response.description, response.alternative_property_names, response.json_path))
            }
            Err(e) => {
                log::error!("Failed to get response from OpenAI: {}", e);
                Err(Errors::UnexpectedError)
            }
        }
    }

    pub async fn get_field_transformation(
        lineage: &Lineage,
        field: &str,
        value: &str,
        snippets: Vec<String>,
    ) -> Option<FieldTransformation> {
        log::trace!("In get_field_transformation");

        log::info!("Determining if field is meaningful...");

        let elimination = match field {
            "text" => {
                Self::should_eliminate_text(lineage, snippets.clone())
                    .await
                    .expect("Could not determine if text should be eliminated")
            },
            _ => {
                Self::should_eliminate_attribute(lineage, field, snippets.clone())
                    .await
                    .expect("Could not determine if attribute should be eliminated")
            }
        };

        if elimination.is_unmeaningful {
            log::info!("Eliminating unmeaningful field");
            return None;
        }

        log::info!("Determining primary field name and metadata...");

        let primary_content = Self::get_primary_content(
            lineage,
            field,
            value,
            snippets.clone(),
        ).await.expect("Could not obtain primary content");

        let transformation = FieldTransformation {
            id: ID::new(),
            description: primary_content.description.clone(),
            field: field.to_string(),
            image: primary_content.name.clone(),
            meta: FieldMetadata {},
        };

        Some(transformation)
    }

    pub async fn categorize_summarize(document: &String) -> Result<(String, String, String), Errors> {
        log::trace!("In categorize_summarize");

        let document = if document.len() > 3000 {
            log::warn!("truncating document");
            &format!("{}...", &document[..3000])
        } else {
            document
        };

        let system_prompt = format!(r##"
 You analyze a condensed website, extrapolate from this minimized version, and provide the following information about the original website the condensed document was derived from:
 1. category: Use one or two words to categorize this type of website. Provide response in snake case.
 2. description: A short paragraph describing what content this website shows.
 3. structure: A detailed description on how the HTML of the page is structured and the way content is organized from a technical perspective.
     "##);
        let user_prompt = format!(r##"
 [Document]
 {}
     "##, document);

        let response_format = json!({
            "type": "json_schema",
            "name": "document_summary",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string"
                    },
                    "description": {
                        "type": "string"
                    },
                    "structure": {
                        "type": "string"
                    }
                },
                "required": ["category", "description", "structure"],
                "additionalProperties": false
            }
        });

        match Self::send_openai_request::<SummaryResponse>(
            &system_prompt,
            &user_prompt,
            response_format
        ).await {
            Ok(response) => {
                log::debug!("╔════════════════════════════╗");
                log::debug!("║       SUMMARY START        ║");
                log::debug!("╚════════════════════════════╝");

                log::debug!("***system_prompt***\n{}", system_prompt);
                log::debug!("***user_prompt***\n{}", user_prompt);
                log::debug!("***response***\n{:?}", response);

                log::debug!("╔═══════════════════════════╗");
                log::debug!("║      SUMMARY END          ║");
                log::debug!("╚═══════════════════════════╝");

                Ok((response.category, response.description, response.structure))
            }
            Err(e) => {
                log::error!("Failed to get response from OpenAI: {}", e);
                Err(Errors::UnexpectedError)
            }
        }
    }

    pub async fn get_relationships(
        overall_context: String,
        target_subgraph_hash: String,
        subgraphs: Vec<(String, String)>,
    ) -> Result<(String, Vec<String>, String), Errors> {
        log::trace!("In get_relationships");

        if subgraphs.is_empty() {
            panic!("Expected at least one subgraph");
        }

        let system_prompt = format!(r##"
The data model for a website has been fragmented into distinct objects. You must interpret JSON fragments and attempt to reconstitute the original objects by matching fragment IDs to other fragment IDs.

A target fragment ID will be provided, and a list of fragments with corresponding fragment ID. Attempt to determine what other fragment IDs may match the target fragment ID by considering the contextual meaning of JSON values and their potential relationship to other fragments of particular type IDs.

If objects with the target fragment type ID would be merged with other objects of another type ID, the resulting JSON should be a coherent object representing a particular type in the data model for a website.

Zero or multiple fragments may match the target fragment. Please provide an array of fragment ID matches and a justification too.

Do not consider the keys or structure of the object, only the values.

Only provide a unique list of fragment IDs that does not include the target fragment ID.

Please also suggest a name in snake case that could be used for programmatically representing objects that result after merging matching fragments (name). Leave blank if zero fragments match.

Provide a description decribing in details the nature and purpose of the object.

The following is an example of how to perform this task:


Target fragment ID: 1

Fragment ID: 1
{{
  "id": 1,
  "username": "alice_smith",
  "email": "alice.smith@example.com",
  "firstName": "Alice"
}}

Fragment ID: 2
{{
  "lastName": "Smith",
  "createdAt": "2023-01-10T09:00:00Z",
  "roles": ["user"],
  "isActive": true
}}

Fragment ID: 1
{{
  "id": 2,
  "username": "bob_jones",
  "email": "bob.jones@example.com",
  "firstName": "Bob"
}}

Fragment ID: 2
{{
  "lastName": "Jones",
  "createdAt": "2023-01-12T11:15:00Z",
  "roles": ["user", "moderator"],
  "isActive": true
}}

Fragment ID: 1
{{
  "id": 3,
  "username": "carol_white",
  "email": "carol.white@example.com",
  "firstName": "Carol"
}}

Fragment ID: 2
{{
  "lastName": "White",
  "createdAt": "2023-01-14T13:30:00Z",
  "roles": ["user"],
  "isActive": false
}}

The response should indicate that fragment ID 2 matches the target fragment ID 1, as we can merge pairs of fragments with IDs 1 and 2 to get coherent typed objects representing user accounts.

"##);

        let fragments = subgraphs.iter().enumerate().fold(
            String::new(),
            |mut acc, (index, (subgraph_hash, json))| {
                acc.push_str(&format!(r##"
Fragment ID: {}:
{}
    "##, subgraph_hash, json));
                acc
            },
        );

        let user_prompt = format!(r##"
===================================================

Consider this website context when deciding how to match fragment type IDs:


{}


===================================================

[Target fragment ID]
{}

[Fragments]
{}
"##, overall_context, target_subgraph_hash, fragments);

        let response_format = json!({
            "type": "json_schema",
            "name": "matching_fragments",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string"
                    },
                    "description": {
                        "type": "string"
                    },
                    "matching_fragments": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        }
                    },
                    "justification": {
                        "type": "string"
                    }
                },
                "required": ["name", "description", "matching_fragments", "justification"],
                "additionalProperties": false
            }
        });

        match Self::send_openai_request::<AssociationsResponse>(&system_prompt, &user_prompt, response_format).await {
            Ok(response) => {
                log::debug!("╔══════════════════════════════╗");
                log::debug!("║       ASSOCIATIONS START     ║");
                log::debug!("╚══════════════════════════════╝");

                log::debug!("***system_prompt***\n{}", system_prompt);
                log::debug!("***user_prompt***\n{}", user_prompt);
                log::debug!("***response***\n{:?}", response);

                log::debug!("╔═══════════════════════════╗");
                log::debug!("║       ASSOCIATIONS END    ║");
                log::debug!("╚═══════════════════════════╝");

                Ok((response.name, response.matching_fragments, response.description))
            }
            Err(e) => {
                log::error!("Failed to get response from OpenAI: {}", e);
                Err(Errors::UnexpectedError)
            }
        }
    }

    async fn get_primary_content(
        lineage: &Lineage,
        field: &str,
        value: &str,
        snippets: Vec<String>,
    ) -> Result<PrimaryResponse, Errors> {
        log::trace!("In get_primary_content");

        let field_value = if field == "text" { value } else { field };

        let system_prompt = format!(r##"
You interpret the contextual meaning of HTML attributes or text nodes and reverse engineer the data model that was possibly used when building the website.

Please provide the following information:
* (name): A variable name in snake case that could be used to represent this text node or attribute programmatically
* (description): A description of the variable name as it might be found in a JSON schema.
* (justification): A justification for your response

One or more examples of the attribute or text node will be provided, contained within an HTML snippet, providing crucial context for you to use. 

The target attribute or text node will be delimited with an HTML comment like so:
<!-- Target node: Start --><a href="https://example.com" other-attribute="val"><!-- Target node: End -->.

When providing your response, you must generalize across all possible values for the text node or attribute, which are not limited to just the set of values in the example snippets. 
        "##);
        let examples = snippets.iter().enumerate().fold(
            String::new(),
            |mut acc, (index, snippet)| {
                acc.push_str(&format!(r##"
Example {}:
{}
"##, index + 1, snippet));
                acc
            }
        );
        let user_prompt = format!(r##"
[attribute/text]
{}

[Examples]
{}
        "##, field_value, examples);

        let response_format = json!({
            "type": "json_schema",
            "name": "primary",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string"
                    },
                    "description": {
                        "type": "string"
                    },
                    "justification": {
                        "type": "string"
                    }
                },
                "required": ["name", "description", "justification"],
                "additionalProperties": false
            }
        });

        match Self::send_openai_request(
            &system_prompt,
            &user_prompt,
            response_format
        ).await {
            Ok(response) => {
                log::debug!("╔═════════════════════════════════╗");
                log::debug!("║          PRIMARY START          ║");
                log::debug!("╚═════════════════════════════════╝");

                log::debug!("***lineage***\n{}", lineage.to_string());
                log::debug!("***system_prompt***\n{}", system_prompt);
                log::debug!("***user_prompt***\n{}", user_prompt);
                log::debug!("***response***\n{:?}", response);

                log::debug!("╔══════════════════════════════╗");
                log::debug!("║          PRIMARY END         ║");
                log::debug!("╚══════════════════════════════╝");

                Ok(response)
            }
            Err(e) => {
                log::error!("Failed to get response from OpenAI: {}", e);
                Err(Errors::UnexpectedError)
            }
        }
    }

    async fn should_eliminate_attribute(
        lineage: &Lineage,
        field: &str,
        snippets: Vec<String>
    ) -> Result<EliminationResponse, Errors> {
        log::trace!("In should_eliminate_attribute");

        let system_prompt = format!(r##"
You interpret the contextual meaning of a specific HTML attribute, and infer if the attribute represents meaningful natural language meant to be consumed by humans as part of their core purpose in visiting a website, as opposed to ancillary content. If a user would intentionally read the attribute's value as part of their usage, it is likely meaningful content.

Carefully examine the HTML attribute along with its surrounding content providing crucial context, and determine if any of the following applies to it:

1. If the attribute represents an advertisement of some kind.
2. If the attribute value contains code of some kind

Include the following in your response:
1. (is_unmeaningful): if any of the above criteria apply to the text node, respond true
2. (justification): provide justification for your response

One or more examples of the attribute will be provided, contained within an HTML snippet, providing crucial context for you to use. 

The attribute will be contained/delimited with an HTML comment like so:
<!-- Target node: Start --><a href="https://example.com" other-attribute="val"><!-- Target node: End -->

When providing your response, you must generalize across all possible values for the attribute, which is not limited to just the set of values in the example snippets. 
        "##);
        let examples = snippets.iter().enumerate().fold(
            String::new(),
            |mut acc, (index, snippet)| {
                acc.push_str(&format!(r##"
Example {}:
{}
"##, index + 1, snippet));
                acc
            }
        );
        let user_prompt = format!(r##"
[Attribute]
{}

[Examples]
{}
        "##, field.trim(), examples);


        Self::should_eliminate(lineage, &system_prompt, &user_prompt).await
    }

    async fn should_eliminate_text(
        lineage: &Lineage,
        snippets: Vec<String>
    ) -> Result<EliminationResponse, Errors> {
        log::trace!("In should_eliminate_text");

        let system_prompt = format!(r##"
You interpret the contextual meaning of a type of HTML text node, and infer if the text node represents meaningful natural language meant to be consumed by humans as part of their core purpose in visiting a website, as opposed to ancillary or presentational text.

Carefully examine the provided HTML text node along with supplementary information providing crucial context, and determine if any of the following applies to it:

1. If the text node represents an advertisement of some kind.
2. If the text node serves a presentational purpose. For example, a pipe symbol may be used to delineate menu items, other text nodes might represent an icon. Presentational text is not meaningful, semantic content humans consume as part of their core purpose for visiting a website.
3. If the text node is a label for a UI element meant to assist the user in understanding how to operate the website, as opposed to content that is meant to be consumed

Include the following in your response:
1. (is_unmeaningful): if any of the above criteria apply to the text node, respond true
2. (justification): provide justification for your response

One or more examples of the text node will be provided, contained within an HTML snippet, providing crucial context for you to use. 

The text nodes will be contained/delimited with an HTML comment like so:
<!-- Target node: Start -->Text node content here<!-- Target node: End -->

When providing your response, you must generalize across all possible values for the text node, which is not limited to just the set of values in the example snippets. 
        "##);
        let examples = snippets.iter().enumerate().fold(
            String::new(),
            |mut acc, (index, snippet)| {
                acc.push_str(&format!(r##"
Example {}:
{}
"##, index + 1, snippet));
                acc
            }
        );
        let user_prompt = format!(r##"
[Examples]
{}
        "##, examples);

        Self::should_eliminate(lineage, &system_prompt, &user_prompt).await
    }

    async fn should_eliminate(
        lineage: &Lineage,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<EliminationResponse, Errors> {
        log::trace!("In should_eliminate");

        let response_format = json!({
            "type": "json_schema",
            "name": "meaningful",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "is_unmeaningful": {
                        "type": "boolean"
                    },
                    "justification": {
                        "type": "string"
                    }
                },
                "required": ["is_unmeaningful", "justification"],
                "additionalProperties": false
            }
        });

        match Self::send_openai_request(
            &system_prompt,
            &user_prompt,
            response_format
        ).await {
            Ok(response) => {
                log::debug!("╔════════════════════════════════════════╗");
                log::debug!("║    SHOULD ELIMINATE FIELD START        ║");
                log::debug!("╚════════════════════════════════════════╝");

                log::debug!("***lineage***\n{}", lineage.to_string());
                log::debug!("***system_prompt***\n{}", system_prompt);
                log::debug!("***user_prompt***\n{}", user_prompt);
                log::debug!("***response***\n{:?}", response);

                log::debug!("╔═══════════════════════════════════════╗");
                log::debug!("║    SHOULD ELIMINATE FIELD END         ║");
                log::debug!("╚═══════════════════════════════════════╝");

                Ok(response)
            }
            Err(e) => {
                log::error!("Failed to get response from OpenAI: {}", e);
                Err(Errors::UnexpectedError)
            }
        }
    }

    async fn should_eliminate_code(code: &str) -> Result<ShouldEliminateCodeResponse, Errors> {
        let system_prompt = format!(r##"
Your task is to determine whether a pseudo-javascript function directly performs any kind of query or mutation on a remote server. 

Look for the presence of URLs, http methods, JSON payloads and other things normally required for javascript to perform a query or mutation.

Please include a justification for your response
        "##);
        let user_prompt = format!("{}", code);

        let response_format = json!({
            "type": "json_schema",
            "name": "query_or_mutation",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "is_query_or_mutation": {
                        "type": "boolean"
                    },
                    "justification": {
                        "type": "string"
                    }
                },
                "required": ["is_query_or_mutation", "justification"],
                "additionalProperties": false
            }
        });

        match Self::send_openai_request(
            &system_prompt,
            &user_prompt,
            response_format
        ).await {
            Ok(response) => {
                log::debug!("╔════════════════════════════════════════╗");
                log::debug!("║    SHOULD ELIMINATE CODE START         ║");
                log::debug!("╚════════════════════════════════════════╝");

                log::debug!("***system_prompt***\n{}", system_prompt);
                log::debug!("***user_prompt***\n{}", user_prompt);
                log::debug!("***response***\n{:?}", response);

                log::debug!("╔═══════════════════════════════════════╗");
                log::debug!("║    SHOULD ELIMINATE CODE END          ║");
                log::debug!("╚═══════════════════════════════════════╝");

                Ok(response)
            }
            Err(e) => {
                log::error!("Failed to get response from OpenAI: {}", e);
                Err(Errors::UnexpectedError)
            }
        }
    }

    async fn code_to_http(code: &str) -> Result<CodeToHttpResponse, Errors> {
        let system_prompt = format!(r##"
Your task is to convert a pseudo-javascript function into a json object that contains all the information needed to reconstruct the network request or API call.

If you do not see an appropriate value for any of the keys in the response format, of if you do not think the code represents a network request, please leave blank.

Please include a justification for your response.
        "##);
        let user_prompt = format!("{}", code);

        let response_format = json!({
            "type": "json_schema",
            "name": "js_http",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "method": {
                        "type": "string"
                    },
                    "url": {
                        "type": "string"
                    },
                    "headers": {
                        "type": "object",
                        "properties": {
                            "content_type": {
                                "type": "string"
                            },
                            "accept": {
                                "type": "string"
                            }
                        },
                        "required": ["content_type", "accept"],
                        "additionalProperties": false
                    },
                    "query_params": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        }
                    },
                    "body": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        }
                    },
                    "justification": {
                        "type": "string"
                    }
                },
                "required": [
                    "method",
                    "url",
                    "headers",
                    "query_params",
                    "body",
                    "justification",
                ],
                "additionalProperties": false
            }
        });

        match Self::send_openai_request(
            &system_prompt,
            &user_prompt,
            response_format
        ).await {
            Ok(response) => {
                log::debug!("╔════════════════════════════════════════╗");
                log::debug!("║    CODE TO HTTP START                  ║");
                log::debug!("╚════════════════════════════════════════╝");

                log::debug!("***system_prompt***\n{}", system_prompt);
                log::debug!("***user_prompt***\n{}", user_prompt);
                log::debug!("***response***\n{:?}", response);

                log::debug!("╔═══════════════════════════════════════╗");
                log::debug!("║    CODE TO HTTP END                   ║");
                log::debug!("╚═══════════════════════════════════════╝");

                Ok(response)
            }
            Err(e) => {
                log::error!("Failed to get response from OpenAI: {}", e);
                Err(Errors::UnexpectedError)
            }
        }
    }

    async fn send_openai_request<T>(
        system_prompt: &str,
        user_prompt: &str,
        response_format: serde_json::Value,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned,
    {
        log::trace!("In send_openai_request");

        let mut hash = Hash::from_items(vec![
            system_prompt,
            user_prompt,
            &response_format.to_string()
        ]);
        let hash = hash.finalize();

        let response = Self::get_or_set_cache(hash.clone(), || async {
            let openai_api_key = get_env_variable("OPENAI_API_KEY");

            let request_json = json!({
                "model": "gpt-4.1-2025-04-14",
                "temperature": 0,
                "input": [
                    {
                        "role": "system",
                        "content": system_prompt
                    },
                    {
                        "role": "user",
                        "content": user_prompt
                    }
                ],
                "text": {
                    "format": response_format,
                }
            });

            let url = "https://api.openai.com/v1/responses";
            let authorization = format!("Bearer {}", openai_api_key);
            let client = reqwest::Client::new();

            match client
                .post(url)
                .json(&request_json)
                .header(header::AUTHORIZATION, authorization)
                .send()
                .await
            {
                Ok(res) => {
                    log::trace!("okay response from openai");
                    log::debug!("res: {:?}", res);

                    match res.json::<serde_json::Value>().await {
                        Ok(json_response) => {
                            log::trace!("okay json from openai");
                            log::debug!("json_response: {:?}", json_response);

                            json_response["output"][0]["content"][0]["text"]
                                .as_str()
                                .map(String::from)
                        }
                        Err(e) => {
                            log::error!("Failed to parse JSON response: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to send request to OpenAI: {}", e);
                    None
                }
            }
        }).await;

        let json_response = response.ok_or("Failed to get response from OpenAI")?;
        let parsed_response: T = serde_json::from_str(&json_response)?;
        Ok(parsed_response)
    }

    async fn get_or_set_cache<F, Fut>(hash: Hash, fetch_data: F) -> Option<String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Option<String>>,
    {
        #[cfg(feature = "caching")]
        {
            Cache::get_or_set_cache(hash, fetch_data).await
        }

        #[cfg(not(feature = "caching"))]
        {
            log::debug!("caching is disabled");
            fetch_data().await
        }
    }
}
