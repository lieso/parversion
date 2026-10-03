use xmltree::{Element, XMLNode};
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::graph_node::Graph;
use crate::normalization_context::NormalizationContext;
use crate::xpath::{XPath};

const XSL_NAMESPACE: &str = "http://www.w3.org/1999/XSL/Transform";

pub struct Xslt {
    template: Template,
}

struct Template {
    pattern: XPath,
    body: Vec<Instruction>
}

enum Instruction {
    Variable { name: String, select: XPath },
    CopyOf { select: XPath },
}

impl Xslt {
    pub fn new(stylesheet: &str) -> Result<Self, Errors> {
        let root = Element::parse(stylesheet.as_bytes()).map_err(|e| {
            Errors::XsltParseError(format!("Stylesheet is not well-formed XML: {}", e))
        })?;

        if root.namespace.as_deref() != Some(XSL_NAMESPACE) || root.name != "stylesheet" {
            return Err(Errors::XsltParseError(
                "Root element must be xsl:stylesheet with valid namespace".to_string()
            ));
        };

        let mut templates: Vec<&Element> = Vec::new();

        for child in &root.children {
            let XMLNode::Element(element) = child else {
                continue; // whitespace, comments
            };

            if element.namespace.as_deref() != Some(XSL_NAMESPACE) {
                return Err(Errors::XsltParseError(format!(
                    "Unexpected top-level element <{}>: expected XSLT elements only",
                    element.name
                )));
            }

            match element.name.as_str() {
                "template" => templates.push(element),
                "output" => {} // accepted, ignored
                other => {
                    return Err(Errors::XsltParseError(format!(
                        "Unsupported top-level element xsl:{}",
                        other
                    )));
                }
            }
        }

        // only expect only a single template per stylesheet

        let template_element = match templates.as_slice() {
            [only] => *only,
            [] => return Err(Errors::XsltParseError("Stylesheet has no xsl:template".to_string())),
            _ => {
                return Err(Errors::XsltParseError(
                        "Only a single xsl:template is supported".to_string(),
                ))
            }
        };

        let match_attribute = template_element.attributes.get("match").ok_or_else(|| {
            Errors::XsltParseError("xsl:template requires a match attribute".to_string())
        })?;

        let pattern = XPath::from_str(match_attribute)?;

        let mut body: Vec<Instruction> = Vec::new();

        for child in &template_element.children {
            let XMLNode::Element(element) = child else {
                continue; // whitespace between instructions
            };

            if element.namespace.as_deref() != Some(XSL_NAMESPACE) {
                return Err(Errors::XsltParseError(format!(
                    "Literal result element <{}> is not supported: this engine only selects nodes",
                    element.name
                )));
            }

            let instruction = match element.name.as_str() {
                "variable" => Instruction::Variable {
                    name: required_attribute(element, "name")?,
                    select: XPath::from_str(&required_attribute(element, "select")?)?,
                },
                "copy-of" => Instruction::CopyOf {
                    select: XPath::from_str(&required_attribute(element, "select")?)?,
                },
                other => {
                    return Err(Errors::XsltParseError(format!(
                        "Unsupported instruction xsl:{}",
                        other
                    )));
                }
            };

            body.push(instruction);
        }

        Ok(Xslt {
            template: Template {
                pattern,
                body,
            }
        })
    }

    pub fn traverse(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        start: Graph,
    ) -> Result<Vec<Graph>, Errors> {
        unimplemented!()
    }
}

fn required_attribute(element: &Element, name: &str) -> Result<String, Errors> {
    element.attributes.get(name).cloned().ok_or_else(|| {
        Errors::XsltParseError(format!("xsl:{} requires a {} attribute", element.name, name))
    })
}
