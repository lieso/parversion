use xmltree::{Element, XMLNode};
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::graph_node::{GraphNode, Graph};
use crate::normalization_context::NormalizationContext;
use crate::xpath::{XPath, traverse_using_xpath_predicate, traverse_using_xpath_node_test, Value, Selection, Variables, Expr};

const XSL_NAMESPACE: &str = "http://www.w3.org/1999/XSL/Transform";

pub struct Xslt {
    template: Template,
}

struct Template {
    pattern: XPath,
    body: Vec<Instruction>
}

enum Instruction {
    Variable { name: String, select: Expr },
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
                    select: Expr::from_str(&required_attribute(element, "select")?)?,
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
    ) -> Result<Vec<Value>, Errors> {
        if !self.matches(Arc::clone(&normalization_context), Arc::clone(&start))? {
            log::warn!("XSLT template pattern did not match start node; returning no nodes");
            return Ok(Vec::new());
        }

        log::info!("XSLT template pattern matched the starting node.");

        let mut variables: Variables = Variables::new();
        let mut result: Vec<Value> = Vec::new();

        for instruction in &self.template.body {
            match instruction {
                Instruction::CopyOf { select } => {
                    let selected = select.evaluate(
                        Arc::clone(&normalization_context),
                        &variables,
                        Arc::clone(&start),
                    )?;
                    result.extend(selected);
                }
                Instruction::Variable { name, select } => {
                    let selected = select.evaluate(
                        Arc::clone(&normalization_context),
                        &variables,
                        Arc::clone(&start),
                    )?;
                    variables.insert(name.clone(), selected);
                }
            }
        }

        Ok(result)
    }

    fn matches(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        node: Graph
    ) -> Result<bool, Errors> {
        let segment = &self.template.pattern.segments[0];

        let mut candidates = traverse_using_xpath_node_test(
            Arc::clone(&normalization_context),
            &Value::from_graph(Arc::clone(&node)),
            &segment.node_test,
        )?;

        for predicate in &segment.predicates {
            if candidates.is_empty() {
                break;
            }
            candidates = traverse_using_xpath_predicate(
                Arc::clone(&normalization_context),
                candidates,
                predicate,
            )?;
        }

        Ok(!candidates.is_empty())
    }
}

fn required_attribute(element: &Element, name: &str) -> Result<String, Errors> {
    element.attributes.get(name).cloned().ok_or_else(|| {
        Errors::XsltParseError(format!("xsl:{} requires a {} attribute", element.name, name))
    })
}
