use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;
use sha2::{Sha256, Digest};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentValueMetadata {
    pub is_title: bool,
    pub is_primary_content: bool,
    pub is_url: bool,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentValue {
    pub meta: ContentValueMetadata,
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentMetadataRecursive {
    pub is_root: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentMetadataEnumerative {
    pub next_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentMetadataAssociative {
    pub subgraph: String,
    pub associated_subgraphs: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<ContentMetadataRecursive>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enumerative: Option<ContentMetadataEnumerative>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub associative: Option<ContentMetadataAssociative>,
}

fn is_metadata_empty(meta: &ContentMetadata) -> bool {
    meta.recursive.is_none() && meta.enumerative.is_none() && meta.associative.is_none()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Content {
    pub id: String,
    pub lineage: String,
    #[serde(skip_serializing_if = "is_metadata_empty")]
    pub meta: ContentMetadata,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<ContentValue>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inner_content: Vec<Content>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Content>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lists: Vec<Vec<Content>>,
}

impl Default for Content {
    fn default() -> Self {
        Content {
            id: Uuid::new_v4().to_string(),
            lineage: String::new(),
            meta: ContentMetadata {
                recursive: None,
                enumerative: None,
                associative: None,
            },
            values: Vec::new(),
            inner_content: Vec::new(),
            children: Vec::new(),
            lists: Vec::new(),
        }
    }
}

pub fn postprocess_content(content: &mut Content) {
    log::trace!("In postprocess_content");

    log::info!("Organising recursive content...");
    let content_copy = content.clone();
    organize_recursive_content(content, &content_copy);

    log::info!("Organising associative content...");
    organize_associative_content(content);

    //log::info!("Organising enumerative content...");
    //organize_enumerative_content(content);

    log::info!("Removing empty objects from content...");
    content.remove_empty();

    //log::info!("Merging content...");
    //content.merge_content();

    log::info!("Clearing data structure meta...");
    clear_data_structure_meta(content);
}

fn clear_data_structure_meta(content: &mut Content) {
    content.inner_content.iter_mut().for_each(|child| clear_data_structure_meta(child));
    content.children.iter_mut().for_each(|child| clear_data_structure_meta(child));
    content.lists.iter_mut().for_each(|list| {
        list.iter_mut().for_each(|child| clear_data_structure_meta(child));
    });

    content.meta = ContentMetadata {
        recursive: None,
        enumerative: None,
        associative: None,
    };
}

fn organize_associative_content(content: &mut Content) {
    content.inner_content.iter_mut().for_each(|child| organize_associative_content(child));

    let mut matched_indices = HashSet::new();
    let inner_content = content.inner_content.clone();
    let len = inner_content.len();

    for i in 0..len {
        if matched_indices.contains(&i) {
            continue;
        }

        for j in (i + 1)..len {
            if matched_indices.contains(&j) {
                continue;
            }

            if let Some(associative_a) = &inner_content[i].meta.associative {
                if let Some(associative_b) = &inner_content[j].meta.associative {
                    if associative_b.associated_subgraphs.contains(&associative_a.subgraph) {
                        matched_indices.insert(i);
                        matched_indices.insert(j);

                        content.inner_content[i].inner_content.extend(inner_content[j].inner_content.clone());
                        content.inner_content[j].inner_content.clear();

                        content.inner_content[i].meta.associative = None;
                        content.inner_content[j].meta.associative = None;

                        break;
                    }
                }
            }
        }
    }
}

fn organize_enumerative_content(content: &mut Content) {
    content.inner_content.iter_mut().for_each(|child| organize_enumerative_content(child));

    let content_map: HashMap<String, Content> = content
        .inner_content
        .iter()
        .map(|item| (item.id.clone(), item.clone()))
        .collect();
    let mut listed_item_ids = HashSet::new();

    for item in &content.inner_content {
        if listed_item_ids.contains(&item.id) {
            continue;
        }

        let mut current_list = Vec::new();
        let mut current_item = item;

        while let Some(meta) = &current_item.meta.enumerative {
            let item_id = current_item.id.clone();

            if current_item.meta.recursive.is_some() {
                break;
            }

            if !listed_item_ids.contains(&item_id) {
                current_list.push(current_item.clone());
                listed_item_ids.insert(item_id.clone());
            }

            match &meta.next_id {
                Some(next_id) => {
                    if let Some(next_item) = content_map.get(next_id) {
                        current_item = next_item;
                    } else {
                        break;
                    }
                },
                None => break,
            }
        }

        if !current_list.is_empty() {
            content.lists.push(current_list);
        }
    }

    content.inner_content.retain(|item| !listed_item_ids.contains(&item.id));
}

fn organize_recursive_content(root: &mut Content, content: &Content) {
    content.inner_content.iter().for_each(|child| organize_recursive_content(root, &child));

    if let Some(recursive) = &content.meta.recursive {
        if let Some(parent_id) = &recursive.parent_id {
            let mut found_parent = false;
            let mut found_content = false;
            let mut queue = VecDeque::new();
            queue.push_back(root);

            while let Some(current) = queue.pop_front() {
                if &current.id == parent_id {
                    found_parent = true;
                    current.children.push(content.clone());
                }

                if let Some(position) = current.inner_content.iter().position(|item| {
                    item.id == content.id
                }) {
                    found_content = true;
                    current.inner_content.remove(position);
                }

                if found_parent && found_content {
                    break;
                }

                for child in &mut current.inner_content {
                    queue.push_back(child);
                }

                for child in &mut current.children {
                    queue.push_back(child);
                }
            }
        }
    }
}

impl ContentMetadata {
    pub fn is_empty(&self) -> bool {
        self.recursive.is_none() && self.enumerative.is_none()
    }
}

impl Content {
    pub fn remove_empty(&mut self) {
        self.inner_content.iter_mut().for_each(|child| child.remove_empty());
        self.children.iter_mut().for_each(|child| child.remove_empty());

        for list in self.lists.iter_mut() {
            list.iter_mut().for_each(|child| child.remove_empty());
        }

        self.inner_content.retain(|child| !child.is_empty());

        if self.values.is_empty() && self.inner_content.len() == 1 && self.inner_content[0].values.is_empty() {
            self.inner_content = self.inner_content[0].inner_content.clone();
        }
    }

    fn is_empty(&self) -> bool {
        self.values.is_empty() && self.inner_content.is_empty()
    }

    pub fn merge_content(&mut self) {
        log::trace!("In merge_content");

        self.inner_content.iter_mut().for_each(|child| child.merge_content());
        self.children.iter_mut().for_each(|child| child.merge_content());

        let merged_values: Vec<ContentValue> = self
            .inner_content
            .iter_mut()
            .filter(|child| {
                child.inner_content.is_empty() && child.meta.is_empty()
            })
            .flat_map(|content| content.values.drain(..))
            .collect();

        self.inner_content.retain(|content| !content.inner_content.is_empty() || !content.meta.is_empty());

        if !merged_values.is_empty() {
            let merged_content = Content {
                id: Uuid::new_v4().to_string(),
                lineage: "?".to_string(),
                meta: ContentMetadata {
                    recursive: None,
                    enumerative: None,
                    associative: None,
                },
                values: merged_values,
                inner_content: Vec::new(),
                children: Vec::new(),
                lists: Vec::new(),
            };

            self.inner_content.insert(0, merged_content);
        }
    }

    pub fn to_json_schema(&mut self) -> Value {
        let mut json_schema = Value::Object(serde_json::Map::new());

        if let Value::Object(ref mut map) = json_schema {
            map.insert("title".to_string(), Value::String("placeholder".to_string()));
            map.insert("properties".to_string(), self.to_json_properties());
        }

        json_schema
    }

    pub fn to_json_properties(&mut self) -> Value {
        let mut values = Value::Object(serde_json::Map::new());

        for value in self.values.iter() {
            let key = &value.name;

            let mut object_value = HashMap::new();
            object_value.insert("type", "string");
            object_value.insert("description", &value.meta.description);

            if let Value::Object(ref mut map) = values {
                map.insert(key.clone(), json!(object_value));
           }
        }



        let mut type_lookup: HashMap<String, usize> = HashMap::new();
        self.inner_content.iter().for_each(|inner_content| {
            let inner_properties = inner_content.clone().to_json_properties();

            let inner_hash = if let Value::Object(ref map) = inner_properties {
                let mut inner_hasher = Sha256::new();

                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();

                for key in keys {
                    inner_hasher.update(key.as_bytes());
                }
                format!("{:x}", inner_hasher.finalize())
            } else {
                panic!("The provided value is not an object.");
            };




            let count = type_lookup.entry(inner_hash).or_insert(0);
            *count += 1;
        });





        let mut seen_hashes = HashSet::new();

        self.inner_content.iter().enumerate().for_each(|(index, inner_content)| {
            let inner_properties = inner_content.clone().to_json_properties();

            let inner_hash = if let Value::Object(ref map) = inner_properties {
                let mut inner_hasher = Sha256::new();

                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();

                for key in keys {
                    inner_hasher.update(key.as_bytes());
                }
                format!("{:x}", inner_hasher.finalize())
            } else {
                panic!("The provided value is not an object.");
            };

            if seen_hashes.insert(inner_hash.clone()) {

                if let Value::Object(ref mut map) = values {
                    if let Some(&count) = type_lookup.get(&inner_hash) {
                        let mut object_value: HashMap<String, Value> = HashMap::new();

                        if count > 1 { 
                            object_value.insert("type".to_string(), Value::String("array".to_string()));
                            object_value.insert("items".to_string(), json!(inner_properties));
                        } else {
                            object_value.insert("type".to_string(), Value::String("object".to_string()));
                            object_value.insert("properties".to_string(), json!(inner_properties));
                        }

                        let key = format!("inner_content_{}", index + 1);

                        map.insert(key, json!(object_value));
                    }
                }

            }
        });





        values
    }
}

pub fn find_content_value_by_path(content: &Content, path: &String, index: usize) -> Option<ContentValue> {
    log::trace!("In find_content_value_by_path");
    log::debug!("index: {}", index);

    fn recurse(
        current_content: &Content,
        current_path: String,
        target_path: &String,
        current_index: &mut usize,
        target_index: &usize
    ) -> Option<ContentValue> {

        for value in &current_content.values {
            let final_path = if current_path.is_empty() {
                format!("{}", value.name)
            } else {
                format!("{}.{}", current_path, value.name)
            };

            if final_path == *target_path {
                if current_index == target_index {
                    return Some(value.clone());
                } else {
                    *current_index += 1;
                }
            }
        }

        for inner_content in &current_content.inner_content {
            let new_path = if current_path.is_empty() {
                String::from("inner_content")
            } else {
                format!("{}.inner_content", current_path)
            };

            if let Some(content_value) = recurse(
                inner_content,
                new_path,
                target_path,
                current_index,
                target_index
            ) {
                return Some(content_value);
            }
        }

        None
    }

    let fixed_path = path
        .split('.')
        .map(|segment| {
            if segment.starts_with("inner_content") {
                String::from("inner_content")
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(".");

    recurse(
        content,
        String::new(),
        &fixed_path,
        &mut 0,
        &index
    )
}
