use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct XPath {
    pub segments: Vec<XPathSegment>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct XPathSegment {
    pub axis: XPathAxis,
    pub node_test: String,
    pub predicates: Vec<XPathPredicate>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum XPathAxis {
    Child,
    Parent,
    Self_,
    Descendant,
    Ancestor,
    FollowingSibling,
    PrecedingSibling,
    Following,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum XPathPredicate {
    Position(usize),
    Attribute { name: String, value: String },
    AttributePresence(Vec<String>),
    Contains { name: String, value: String }
}

impl XPath {
    pub fn from_str(s: &str) -> Result<Self, Errors> {
        log::trace!("In XPath::from_str");

        let segments = s
            .replace("//", "/descendant::")
            .split('/')
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
        }
    }
}

impl XPathPredicate {
    fn from_str(s: &str) -> Result<Self, Errors> {
        if let Some(inner) = s.strip_prefix('@') {
            let eq_pos = inner.find('=').ok_or_else(|| {
                Errors::XPathParseError(format!("Invalid attribute predicate: {}", s))
            })?;
            let name = inner[..eq_pos].to_string();
            let value = inner[eq_pos + 1..]
                .trim_matches('\'')
                .trim_matches('"')
                .to_string();
            Ok(XPathPredicate::Attribute { name, value })
        } else if let Some(inner) = s.strip_prefix("contains(").and_then(|s| s.strip_suffix(')')) {
            let (attr_part, val_part) = inner.split_once(',')
                .ok_or_else(|| Errors::XPathParseError(format!("Invalid contains() predicate: {}", s)))?;
            let name = attr_part.trim().trim_start_matches('@').to_string();
            let value = val_part.trim().trim_matches('\'').trim_matches('"').to_string();
            Ok(XPathPredicate::Contains { name, value })
        } else if let Ok(pos) = s.parse::<usize>() {
            Ok(XPathPredicate::Position(pos))
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
            XPathPredicate::AttributePresence(attrs) => {
                attrs.iter()
                    .map(|attr| format!("@{}", attr))
                    .collect::<Vec<_>>()
                    .join(" and ")
            }
        }
    }
}
