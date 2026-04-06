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
    pub predicate: Option<XPathPredicate>,
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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum XPathPredicate {
    Position(usize),
    Attribute { name: String, value: String },
}

impl XPath {
    pub fn from_str(s: &str) -> Result<Self, Errors> {
        log::trace!("In XPath::from_str");

        let segments = s
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
        let (node_part, predicate) = if let Some(bracket_pos) = s.find('[') {
            if !s.ends_with(']') {
                return Err(Errors::XPathParseError(format!(
                    "Unterminated predicate in segment: {}",
                    s
                )));
            }
            let pred_str = &s[bracket_pos + 1..s.len() - 1];
            (&s[..bracket_pos], Some(XPathPredicate::from_str(pred_str)?))
        } else {
            (s, None)
        };

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
            predicate,
        })
    }

    pub fn to_string(&self) -> String {
        let axis_prefix = if self.axis == XPathAxis::Child {
            String::new()
        } else {
            format!("{}::", self.axis.to_str())
        };
        let predicate_suffix = match &self.predicate {
            Some(pred) => format!("[{}]", pred.to_string()),
            None => String::new(),
        };
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
        }
    }
}
