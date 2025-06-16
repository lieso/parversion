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
struct PeripheralResponse {
    pub is_peripheral: bool,
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

impl OpenAI {
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

        log::info!("Determining if field is peripheral...");

        let peripheral = Self::get_peripheral_if_applicable(
            lineage,
            field,
            value,
            snippets.clone(),
        ).await.expect("Could not determine if field is peripheral");

        if peripheral.is_peripheral {
            log::info!("Field identified as secondary/peripheral");

            let transformation = FieldTransformation {
                id: ID::new(),
                description: String::from("Related content description"),
                field: field.to_string(),
                image: String::from("related_content"),
                meta: FieldMetadata {}
            };

            return Some(transformation);
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

    async fn get_peripheral_if_applicable(
        lineage: &Lineage,
        field: &str,
        value: &str,
        snippets: Vec<String>,
    ) -> Result<PeripheralResponse, Errors> {
        log::trace!("In get_peripheral_if_applicable");

        let field_value = if field == "text" { value } else { field };

        let system_prompt = format!(r##"
You interpret the contextual meaning of HTML attributes or text nodes and infer if it is content pertaining to the core purpose of the website, or if it is peripheral/secondary content. Peripheral content is not the primary focus of the website's message or purpose.

Primary content is defined as content that is essential to the website's core purpose and cannot be removed without altering the fundamental experience of interacting with the
content. This includes:
* Content that directly contributes to the main purpose of the site, such as articles, user profiles, or discussion threads on news and social platforms.
* Elements that are integral to user engagement and understanding of the site's main offerings.

Peripheral content includes:
* Website menu bars, footers, or sidebars that link to unrelated pages or external resources.
* Links to administrative pages such as login, signup, or settings that do not enhance the understanding or interaction with the main content.
* Advertisements or promotional banners that do not contribute to the main purpose of the site.

Include the following in your response:
1. (is_peripheral): If this is peripheral content.
2. (justification): Provide justification for your response.

One or more examples of the attribute or text node will be provided, contained within an HTML snippet, providing crucial context for you to use.

The target attribute or text node will be delimited with an HTML comment like so:
<!-- Target node: Start --><a href="https://example.com" other-attribute="val"><!-- Target node: End -->.

When providing your response, you must generalize across all possible values for the text node or attribute, which are not limited to just the set of values in the example snippet(s).
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
            "name": "meaningful_response",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "is_peripheral": {
                        "type": "boolean"
                    },
                    "justification": {
                        "type": "string"
                    }
                },
                "required": ["is_peripheral", "justification"],
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
                log::debug!("║          IS PERIPHERAL START           ║");
                log::debug!("╚════════════════════════════════════════╝");

                log::debug!("***lineage***\n{}", lineage.to_string());
                log::debug!("***system_prompt***\n{}", system_prompt);
                log::debug!("***user_prompt***\n{}", user_prompt);
                log::debug!("***response***\n{:?}", response);

                log::debug!("╔═══════════════════════════════════════╗");
                log::debug!("║          IS PERIPHERAL END            ║");
                log::debug!("╚═══════════════════════════════════════╝");

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
                "model": "gpt-4o-2024-08-06",
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
                .header(header::CONTENT_TYPE, "application/json")
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
