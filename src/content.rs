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

    log::info!("Merging content...");
    content.merge_content();

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

    pub fn to_json_schema(&mut self) -> HashMap<String, Value> {
        let mut object: HashMap<String, Value> = HashMap::new();
        let mut hasher = Sha256::new();
        let mut hasher_items = Vec::new();

        for value in self.values.iter() {
            let key = &value.name;
            let object_value = "string";

            hasher_items.push(key.clone());
            hasher_items.push(object_value.to_string());

            object.insert(key.clone(), json!(object_value.to_string()));
        }

        hasher_items.sort();
        hasher.update(hasher_items.join(""));
        
        let hash = format!("{:x}", hasher.finalize());
        let mut final_object = vec![(hash, json!(object))];

        self.inner_content.iter().for_each(|inner_content| {
            final_object.extend(inner_content.clone().to_json_schema());
        });

        final_object
            .into_iter()
            .filter(|(_, v)| !v.is_object() || !v.as_object().unwrap().is_empty())
            .collect()
    }
}
