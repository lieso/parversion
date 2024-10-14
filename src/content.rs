use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Content {
    pub id: String,
    pub meta: ContentMetadata,
    pub values: Vec<ContentValue>,
    pub inner_content: Vec<Content>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Content>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lists: Vec<Vec<Content>>,
}

pub fn postprocess_content(content: &mut Content) {
    log::trace!("In postprocess_content");

    log::info!("Organising recursive content...");
    let content_copy = content.clone();
    organize_recursive_content(content, &content_copy);

    log::info!("Organising enumerative content...");
    organize_enumerative_content(content);

    log::info!("Removing empty objects from content...");
    content.remove_empty();

    log::info!("Merging content...");
    content.merge_content();
}

fn organize_enumerative_content(content: &mut Content) {
    content.inner_content.iter_mut().for_each(|child| organize_enumerative_content(child));

    let content_map: HashMap<String, &Content> = content
        .inner_content
        .iter()
        .map(|item| (item.id.clone(), item))
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
}
