use std::sync::{Arc};
use serde::{Serialize, Deserialize};
use std::collections::{VecDeque};
use uuid::Uuid;

use crate::graph_node::{Graph, get_lineage, apply_lineage, GraphNodeData};
use crate::xml_node::{XmlNode};
use crate::basis_node::{BasisNode};
use crate::error::{Errors};
use crate::macros::*;
use crate::basis_graph::{BasisGraph};
use crate::node_data_structure::{apply_structure};

#[derive(Clone, Debug)]
pub struct Traversal {
    pub output_tree: Graph<XmlNode>,
    pub basis_graph: Option<BasisGraph>,
}

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
pub struct ContentMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<ContentMetadataRecursive>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Content {
    pub id: String,
    pub meta: ContentMetadata,
    pub values: Vec<ContentValue>,
    pub inner_content: Vec<Content>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Content>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Harvest {
    pub content: Content,
    pub related_content: Content,
}

impl Content {
    pub fn remove_empty(&mut self) {
        self.inner_content.iter_mut().for_each(|child| child.remove_empty());
        self.children.iter_mut().for_each(|child| child.remove_empty());

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
                child.inner_content.is_empty() && child.meta.recursive.is_none()
            })
            .flat_map(|content| content.values.drain(..))
            .collect();

        self.inner_content.retain(|content| !content.inner_content.is_empty() || content.meta.recursive.is_some());

        if !merged_values.is_empty() {
            let merged_content = Content {
                id: Uuid::new_v4().to_string(),
                meta: ContentMetadata {
                    recursive: None,
                },
                values: merged_values,
                inner_content: Vec::new(),
                children: Vec::new(),
            };

            self.inner_content.insert(0, merged_content);
        }
    }
}

fn organize_content(root: &mut Content, content: &Content) {
    content.inner_content.iter().for_each(|child| organize_content(root, &child));

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

fn postprocess_content(content: &mut Content) {
    log::trace!("In postprocess_content");

    log::info!("Organising content...");
    let content_copy = content.clone();
    organize_content(content, &content_copy);

    log::info!("Removing empty objects from content...");
    content.remove_empty();

    log::info!("Merging content...");
    content.merge_content();
}

fn process_node(
    output_node: Graph<XmlNode>,
    basis_graph: Graph<BasisNode>,
    content: &mut Content,
    related_content: &mut Content,
) {
    log::trace!("In process_node");

    {
        let block_separator = "=".repeat(60);
        log::info!("{}", format!(
        "\n{}
PROCESSING NODE:
{}
Node:   {}
{}",
            block_separator,
            block_separator,
            read_lock!(output_node).data.describe(),
            block_separator,
        ));
    }

    let output_node_xml: XmlNode = read_lock!(output_node).data.clone();
    let lineage = get_lineage(Arc::clone(&output_node));
    let basis_node: Graph<BasisNode> = apply_lineage(Arc::clone(&basis_graph), lineage);




    // If a node has a parent which is an element with an href that is interpreted to be an "action" link, we discard it
    // These nodes only describe the action link
    // e.g. <a href="reply?id=123">reply</a>
    let rl = read_lock!(output_node);
    if let Some(output_node_parent) = rl.parents.get(0) {
        let output_node_parent_lineage = get_lineage(Arc::clone(&output_node_parent));
        let output_node_parent_basis_node: Graph<BasisNode> = apply_lineage(Arc::clone(&basis_graph), output_node_parent_lineage);
        let output_node_parent_basis_node_data = read_lock!(output_node_parent_basis_node).data.data.clone();
        let is_parent_action = read_lock!(output_node_parent_basis_node_data).iter().any(|item| {
            if let Some(element_data) = &item.element {
                return element_data.attribute == "href" && !element_data.is_page_link;
            }

            false
        });

        if is_parent_action {
            log::info!("Discarding node data whose parent is an action href");
            return;
        }
    }
    




    let data = read_lock!(basis_node).data.data.clone();
    for node_data in read_lock!(data).iter() {
        if let Some(text_data) = &node_data.text {
            if text_data.is_presentational {
                log::info!("Discarding presentational text node data");
                continue;
            }
        }

        if let Some(element_data) = &node_data.element {
            if element_data.attribute == "href" {
                if !element_data.is_page_link {
                    log::info!("Discarding href action link...");
                    continue;
                }
            }
        }

        let is_advertisement = {
            node_data.clone().text.map_or(false, |text| text.is_advertisement) ||
            node_data.clone().element.map_or(false, |element| element.is_advertisement)
        };
        if is_advertisement {
            log::info!("Discarding advertisement");
            continue;
        }

        let content_value = ContentValue {
            name: node_data.name.clone(),
            value: node_data.value(&output_node_xml),
            meta: ContentValueMetadata {
                is_title: node_data.text.clone().map_or(false, |text| text.is_title),
                is_primary_content: node_data.text.clone().map_or(false, |text| text.is_primary_content),
                is_url: node_data.element.clone().map_or(false, |element| {
                    element.attribute == "href"
                })
            },
        };

        let is_peripheral = {
            node_data.clone().text.map_or(false, |text| text.is_peripheral_content) ||
            node_data.clone().element.map_or(false, |element| element.is_peripheral_content)
        };
        log::debug!("is_peripheral: {}", is_peripheral);

        if is_peripheral {
            related_content.values.push(content_value);
        } else {
            content.values.push(content_value);
        }
    }

    let structures = read_lock!(basis_node).data.structure.clone();
    for structure in read_lock!(structures).iter() {
        let meta = apply_structure(
            structure.clone(),
            Arc::clone(&output_node),
        );

        content.meta = meta.clone();
        related_content.meta = meta.clone();
    }
}

impl Traversal {
    pub fn from_tree(output_tree: Graph<XmlNode>) -> Self {
        Traversal {
            output_tree: output_tree,
            basis_graph: None,
        }
    }

    pub fn with_basis(mut self, graph: BasisGraph) -> Self {
        self.basis_graph = Some(graph);

        self
    }

    pub fn harvest(self) -> Result<Harvest, Errors> {
        let mut content = Content {
            id: read_lock!(self.output_tree).id.clone(),
            meta: ContentMetadata {
                recursive: None,
            },
            values: Vec::new(),
            inner_content: Vec::new(),
            children: Vec::new(),
        };
        let mut related_content = Content {
            id: read_lock!(self.output_tree).id.clone(),
            meta: ContentMetadata {
                recursive: None,
            },
            values: Vec::new(),
            inner_content: Vec::new(),
            children: Vec::new(),
        };

        fn recurse(
            mut output_node: Graph<XmlNode>,
            basis_graph: Graph<BasisNode>,
            output_content: &mut Content,
            output_related_content: &mut Content,
        ) {
            if read_lock!(output_node).is_linear_tail() {
                panic!("Did not expect to encounter node in linear tail");
            }

            if read_lock!(output_node).is_linear_head() {
                log::info!("Output node is head of linear sequence of nodes");

                while read_lock!(output_node).is_linear() {
                    process_node(Arc::clone(&output_node), Arc::clone(&basis_graph), output_content, output_related_content);

                    output_node = {
                        let next_node = read_lock!(output_node).children.first().expect("Linear output node has no children").clone();
                        next_node.clone()
                    };
                }

                process_node(Arc::clone(&output_node), Arc::clone(&basis_graph), output_content, output_related_content);
            } else {
                log::info!("Output node is non-linear");

                process_node(Arc::clone(&output_node), Arc::clone(&basis_graph), output_content, output_related_content);
            }

            for child in read_lock!(output_node).children.iter() {
                let mut child_content = Content {
                    id: read_lock!(child).id.clone(),
                    meta: ContentMetadata {
                        recursive: None,
                    },
                    values: Vec::new(),
                    inner_content: Vec::new(),
                    children: Vec::new(),
                };
                let mut child_related_content = Content {
                    id: read_lock!(child).id.clone(),
                    meta: ContentMetadata {
                        recursive: None,
                    },
                    values: Vec::new(),
                    inner_content: Vec::new(),
                    children: Vec::new(),
                };

                recurse(
                    Arc::clone(child),
                    Arc::clone(&basis_graph),
                    &mut child_content,
                    &mut child_related_content,
                );

                output_content.inner_content.push(child_content);
                output_related_content.inner_content.push(child_related_content);
            }
        }

        recurse(
            Arc::clone(&self.output_tree),
            Arc::clone(&self.basis_graph.clone().unwrap().root),
            &mut content,
            &mut related_content,
        );

        postprocess_content(&mut content);
        postprocess_content(&mut related_content);

        Ok(Harvest {
            content: content,
            related_content: related_content,
        })
    }
}
