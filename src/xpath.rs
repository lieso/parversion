use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{RwLock, Arc};
use std::cell::RefCell;

use crate::prelude::*;
use crate::graph_node::{Graph, GraphNode};

thread_local! {
    static XPATH_CACHE: RefCell<HashMap<(ID, Vec<XPathSegment>), Vec<Graph>>> = RefCell::new(HashMap::new());
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
pub struct XPath {
    pub segments: Vec<XPathSegment>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
pub struct XPathSegment {
    pub axis: XPathAxis,
    pub node_test: String,
    pub predicates: Vec<XPathPredicate>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
pub enum XPathAxis {
    Child,
    Parent,
    Self_,
    Descendant,
    Ancestor,
    FollowingSibling,
    PrecedingSibling,
    Following,
    Preceding,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
pub enum XPathPredicate {
    Position(usize),
    Attribute { name: String, value: String },
    AttributePresence(Vec<String>),
    Contains { name: String, value: String },
    Last,
    StartsWith { name: String, value: String },
    Path(XPath),
}

impl XPath {
    pub fn traverse(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        start: Graph,
    ) -> Result<Option<Graph>, Errors> {
        let start_id = read_lock!(start).id.clone();
        let mut current: Vec<Graph> = vec![Arc::clone(&start)];

        for (index, segment) in self.segments.iter().enumerate() {
            let cache_key = (start_id.clone(), self.segments[0..=index].to_vec());

            if let Some(cached) = XPATH_CACHE.with(|cache| cache.borrow().get(&cache_key).cloned()) {
                current = cached;
                if current.is_empty() {
                    return Ok(None);
                }
                continue;
            }

            current = current
                .iter()
                .map(|graph| {
                    GraphNode::traverse_using_xpath_segment(
                        Arc::clone(&normalization_context),
                        Arc::clone(graph),
                        segment
                    )
                })
            .collect::<Result<Vec<Vec<Graph>>, Errors>>()?
                .into_iter()
                .flatten()
                .collect();

            if current.is_empty() {
                return Ok(None);
            }

            XPATH_CACHE.with(|cache| cache.borrow_mut().insert(cache_key, current.clone()));
        }

        Ok(current.first().cloned())
    }

    pub fn from_str(s: &str) -> Result<Self, Errors> {
        let s = s.replace("//", "/descendant::");

        let mut parts: Vec<&str> = Vec::new();
        let mut depth = 0;
        let mut start = 0;
        for (i, c) in s.char_indices() {
            match c {
                '[' => depth += 1,
                ']' => depth -= 1,
                '/' if depth == 0 => {
                    parts.push(&s[start..i].trim());
                    start = i + 1;
                }
                _ => {}
            }
        }
        parts.push(&s[start..].trim());

        let segments = parts
            .into_iter()
            .filter(|part| !part.is_empty())
            .map(XPathSegment::from_str)
            .collect::<Result<Vec<_>, Errors>>()?;

        if segments.is_empty() {
            return Err(Errors::XPathParseError("XPath is empty".to_string()));
        }

        Ok(XPath { segments })
    }

    pub fn to_string(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join("/")
    }
}

impl XPathSegment {
    fn from_str(s: &str) -> Result<Self, Errors> {
        let mut rest = s;
        let mut predicate_strs: Vec<&str> = Vec::new();

        while rest.ends_with(']') {
            // find the matching '[' for this trailing ']', scanning from the end
            // so nested/independent bracket groups don't get confused
            let mut depth = 0;
            let mut open_pos = None;
            for (i, c) in rest.char_indices().rev() {
                match c {
                    ']' => depth += 1,
                    '[' => {
                        depth -= 1;
                        if depth == 0 {
                            open_pos = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let open_pos = open_pos.ok_or_else(|| {
                Errors::XPathParseError(format!("Unterminated predicate in segment: {}", s))
            })?;

            predicate_strs.push(&rest[open_pos + 1..rest.len() - 1]);
            rest = &rest[..open_pos];
        }
        predicate_strs.reverse(); // preserve left-to-right predicate order

        let node_part = rest;

        let predicates = predicate_strs
            .into_iter()
            .map(XPathPredicate::from_str)
            .collect::<Result<Vec<_>, Errors>>()?;

        if node_part == "." {
            return Ok(XPathSegment {
                axis: XPathAxis::Self_,
                node_test: String::new(),
                predicates,
            });
        }

        if node_part == ".." {
            return Ok(XPathSegment {
                axis: XPathAxis::Parent,
                node_test: String::new(),
                predicates,
            });
        }

        let (axis, node_test) = if let Some(axis_end) = node_part.find("::") {
            let axis = XPathAxis::from_str(&node_part[..axis_end])?;
            (axis, &node_part[axis_end + 2..])
        } else {
            (XPathAxis::Child, node_part)
        };

        if node_test.is_empty() {
            return Err(Errors::XPathParseError(format!(
                "Empty node test in segment: {}",
                s
            )));
        }

        Ok(XPathSegment {
            axis,
            node_test: node_test.to_string(),
            predicates,
        })
    }

    pub fn to_string(&self) -> String {
        let axis_prefix = if self.axis == XPathAxis::Child {
            String::new()
        } else {
            format!("{}::", self.axis.to_str())
        };
        let predicate_suffix: String = self.predicates
            .iter()
            .map(|pred| format!("[{}]", pred.to_string()))
            .collect();
        format!("{}{}{}", axis_prefix, self.node_test, predicate_suffix)
    }
}

impl XPathAxis {
    fn from_str(s: &str) -> Result<Self, Errors> {
        match s {
            "child" => Ok(XPathAxis::Child),
            "parent" => Ok(XPathAxis::Parent),
            "self" => Ok(XPathAxis::Self_),
            "descendant" => Ok(XPathAxis::Descendant),
            "ancestor" => Ok(XPathAxis::Ancestor),
            "following-sibling" => Ok(XPathAxis::FollowingSibling),
            "preceding-sibling" => Ok(XPathAxis::PrecedingSibling),
            "following" => Ok(XPathAxis::Following),
            "preceding" => Ok(XPathAxis::Preceding),
            _ => Err(Errors::XPathParseError(format!("Unknown axis: {}", s))),
        }
    }

    fn to_str(&self) -> &str {
        match self {
            XPathAxis::Child => "child",
            XPathAxis::Parent => "parent",
            XPathAxis::Self_ => "self",
            XPathAxis::Descendant => "descendant",
            XPathAxis::Ancestor => "ancestor",
            XPathAxis::FollowingSibling => "following-sibling",
            XPathAxis::PrecedingSibling => "preceding-sibling",
            XPathAxis::Following => "following",
            XPathAxis::Preceding => "preceding"
        }
    }
}

impl XPathPredicate {
    fn from_str(s: &str) -> Result<Self, Errors> {
        if s == "last()" {
            return Ok(XPathPredicate::Last);
        }

        if s.contains(" and ") && s.split(" and ").all(|part| {
            let part = part.trim();
            part.starts_with('@') && !part.contains('=')
        }) {
            let names = s.split(" and ")
                .map(|part| part.trim().trim_start_matches('@').to_string())
                .collect();
            return Ok(XPathPredicate::AttributePresence(names));
        }

        if let Some(inner) = s.strip_prefix('@') {
            if let Some(eq_pos) = inner.find('=') {
                let name = inner[..eq_pos].to_string();
                let value = inner[eq_pos + 1..]
                    .trim_matches('\'')
                    .trim_matches('"')
                    .to_string();
                Ok(XPathPredicate::Attribute { name, value })
            } else {
                Ok(XPathPredicate::AttributePresence(vec![inner.to_string()]))
            }
        } else if let Some(inner) = s.strip_prefix("contains(").and_then(|s| s.strip_suffix(')')) {
            let (attr_part, val_part) = inner.split_once(',')
                .ok_or_else(|| Errors::XPathParseError(format!("Invalid contains() predicate: {}", s)))?;
            let name = attr_part.trim().trim_start_matches('@').to_string();
            let value = val_part.trim().trim_matches('\'').trim_matches('"').to_string();
            Ok(XPathPredicate::Contains { name, value })
        } else if let Some(inner) = s.strip_prefix("starts-with(").and_then(|s| s.strip_suffix(')')) {
            let (attr_part, val_part) = inner.split_once(',')
                .ok_or_else(|| Errors::XPathParseError(format!("Invalid starts-with() predicate: {}", s)))?;
            let name = attr_part.trim().trim_start_matches('@').to_string();
            let value = val_part.trim().trim_matches('\'').trim_matches('"').to_string();
            Ok(XPathPredicate::StartsWith { name, value })
        } else if let Ok(pos) = s.parse::<usize>() {
            Ok(XPathPredicate::Position(pos))
        } else if let Ok(path) = XPath::from_str(s) {
            Ok(XPathPredicate::Path(path))
        } else {
            Err(Errors::XPathParseError(format!(
                "Unrecognized predicate: {}",
                s
            )))
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            XPathPredicate::Position(n) => n.to_string(),
            XPathPredicate::Attribute { name, value } => format!("@{}='{}'", name, value),
            XPathPredicate::Contains { name, value } => format!("contains(@{},'{}')", name, value),
            XPathPredicate::Last => "last()".to_string(),
            XPathPredicate::AttributePresence(attrs) => {
                attrs.iter()
                    .map(|attr| format!("@{}", attr))
                    .collect::<Vec<_>>()
                    .join(" and ")
            },
            XPathPredicate::StartsWith { name, value } => format!("starts-with(@{},'{}')", name, value),
            XPathPredicate::Path(path) => path.to_string(),
        }
    }
}
