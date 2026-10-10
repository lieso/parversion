use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::graph_node::Graph;
use crate::prelude::*;

mod traverse;

pub use traverse::{
    traverse_using_xpath_node_test, traverse_using_xpath_predicate, traverse_using_xpath_segment,
};

thread_local! {
    static XPATH_CACHE: RefCell<HashMap<(ID, Vec<XPathSegment>), Vec<Value>>> = RefCell::new(HashMap::new());
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
pub struct XPath {
    #[serde(default)]
    pub start_variable: Option<String>,
    pub segments: Vec<XPathSegment>,
    #[serde(default)]
    pub union: Vec<XPath>,
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
    Attribute,
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
    ContainsNormalized { value: String },
    Last,
    StartsWith { name: String, value: String },
    Path(XPath),
    And(Vec<XPathPredicate>),
    Not(Box<XPathPredicate>),
    ContainsToken { name: String, value: String },
    Equals { lhs: Expr, rhs: Operand },
}

#[derive(Clone, Debug)]
pub struct Value {
    pub graph: Graph,
    pub selection: Option<Selection>,
}

pub type Variables = HashMap<String, Vec<Value>>;

impl Value {
    pub fn from_graph(graph: Graph) -> Self {
        Value {
            graph,
            selection: None,
        }
    }
    pub fn is_node(&self) -> bool {
        self.selection.is_none()
    }
    pub fn to_number(&self) -> Option<f64> {
        match &self.selection {
            Some(Selection::Number(n)) => Some(*n),
            Some(Selection::Attribute { value, .. }) | Some(Selection::String(value)) => {
                value.trim().parse().ok()
            }
            Some(Selection::Boolean(b)) => Some(if *b { 1.0 } else { 0.0 }),
            None => None, // node string-value: not needed yet
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Selection {
    Attribute { name: String, value: String },
    String(String),
    Number(f64),
    Boolean(bool),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Expr {
    Path(XPath),
    Function { name: String, arg: Box<Expr> }, // number(...)
    Filter { base: Box<Expr>, position: usize }, // (...)[1]
}

impl Expr {
    fn substitute(&self, variables: &Variables) -> Result<Expr, Errors> {
        Ok(match self {
            Expr::Path(path) => Expr::Path(path.substitute(variables)?),
            Expr::Function { name, arg } => Expr::Function {
                name: name.clone(),
                arg: Box::new(arg.substitute(variables)?),
            },
            Expr::Filter { base, position } => Expr::Filter {
                base: Box::new(base.substitute(variables)?),
                position: *position,
            },
        })
    }

    pub fn from_str(s: &str) -> Result<Expr, Errors> {
        let s = s.trim();

        // name( ... )
        if let Some(open) = s.find('(') {
            let name = &s[..open];
            if !name.is_empty()
                && name.chars().all(|c| c.is_alphanumeric() || c == '-')
                && matching_paren(s, open) == Some(s.len() - 1)
            {
                return Ok(Expr::Function {
                    name: name.to_string(),
                    arg: Box::new(Expr::from_str(&s[open + 1..s.len() - 1])?),
                });
            }
        }

        // ( ... ) or ( ... )[n]
        if s.starts_with('(') {
            let close = matching_paren(s, 0)
                .ok_or_else(|| Errors::XPathParseError(format!("Unbalanced parentheses: {}", s)))?;
            let inner = Expr::from_str(&s[1..close])?;
            let rest = s[close + 1..].trim();
            if rest.is_empty() {
                return Ok(inner);
            }
            let position = rest
                .strip_prefix('[')
                .and_then(|r| r.strip_suffix(']'))
                .and_then(|r| r.trim().parse::<usize>().ok())
                .ok_or_else(|| Errors::XPathParseError(format!("Unsupported expression: {}", s)))?;
            return Ok(Expr::Filter {
                base: Box::new(inner),
                position,
            });
        }

        Ok(Expr::Path(XPath::from_str(s)?))
    }
}

/// Index of the ')' matching the '(' at `open`, ignoring parens inside quotes.
fn matching_paren(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    let mut quote: Option<char> = None;
    for (i, c) in s.char_indices().skip_while(|(i, _)| *i < open) {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '\'') | (None, '"') => quote = Some(c),
            (None, '(') => depth += 1,
            (None, ')') => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Operand {
    /// Source text, so Operand can stay Hash + Eq (f64 can't).
    Number(String),
    Variable(String),
    Arith {
        op: ArithOp,
        lhs: Box<Operand>,
        rhs: Box<Operand>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
pub enum ArithOp {
    Add,
    Sub,
}

impl Operand {
    fn substitute(&self, variables: &Variables) -> Result<Operand, Errors> {
        Ok(match self {
            Operand::Number(_) => self.clone(),
            Operand::Variable(name) => {
                let n = variables
                    .get(name)
                    .and_then(|values| values.first())
                    .and_then(|v| v.to_number())
                    .ok_or_else(|| {
                        Errors::XPathTraverseError(format!(
                            "Unbound or non-numeric variable ${}",
                            name
                        ))
                    })?;
                Operand::Number(n.to_string())
            }
            Operand::Arith { op, lhs, rhs } => Operand::Arith {
                op: op.clone(),
                lhs: Box::new(lhs.substitute(variables)?),
                rhs: Box::new(rhs.substitute(variables)?),
            },
        })
    }

    fn from_str(s: &str) -> Result<Operand, Errors> {
        let s = s.trim();

        // rightmost top-level operator => left-associative: `a - b + c` = `(a - b) + c`
        if let Some((pos, op)) = rightmost_top_level_op(s) {
            return Ok(Operand::Arith {
                op,
                lhs: Box::new(Operand::from_str(&s[..pos])?),
                rhs: Box::new(Operand::from_str(&s[pos + 3..])?), // " - " / " + " are 3 bytes
            });
        }

        // ( ... ) grouping, and number( ... ): operands are already numeric, so it's the identity
        if let Some(open) = s.find('(') {
            let name = &s[..open];
            if (name.is_empty() || name == "number") && matching_paren(s, open) == Some(s.len() - 1)
            {
                return Operand::from_str(&s[open + 1..s.len() - 1]);
            }
        }

        if let Some(name) = s.strip_prefix('$') {
            return Ok(Operand::Variable(name.to_string()));
        }
        if s.parse::<f64>().is_ok() {
            return Ok(Operand::Number(s.to_string()));
        }
        Err(Errors::XPathParseError(format!(
            "Unsupported operand: {}",
            s
        )))
    }
}

impl Expr {
    pub fn to_string(&self) -> String {
        match self {
            Expr::Path(path) => path.to_string(),
            Expr::Function { name, arg } => format!("{}({})", name, arg.to_string()),
            Expr::Filter { base, position } => format!("({})[{}]", base.to_string(), position),
        }
    }

    pub fn evaluate(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        variables: &Variables,
        start: Graph,
    ) -> Result<Vec<Value>, Errors> {
        match self {
            Expr::Path(path) => path.evaluate(normalization_context, variables, start),
            Expr::Filter { base, position } => {
                let values = base.evaluate(normalization_context, variables, start)?;
                Ok(position
                    .checked_sub(1)
                    .and_then(|i| values.into_iter().nth(i))
                    .into_iter()
                    .collect())
            }
            Expr::Function { name, arg } => match name.as_str() {
                "number" => {
                    let values = arg.evaluate(
                        Arc::clone(&normalization_context),
                        variables,
                        Arc::clone(&start),
                    )?;
                    let n = values
                        .first()
                        .and_then(|v| v.to_number())
                        .unwrap_or(f64::NAN);
                    Ok(vec![Value {
                        graph: start,
                        selection: Some(Selection::Number(n)),
                    }])
                }
                other => Err(Errors::XPathTraverseError(format!(
                    "Unsupported function {}()",
                    other
                ))),
            },
        }
    }
}

impl Operand {
    pub fn to_string(&self) -> String {
        match self {
            Operand::Number(n) => n.clone(),
            Operand::Variable(name) => format!("${}", name),
            Operand::Arith { op, lhs, rhs } => {
                let sym = match op {
                    ArithOp::Add => "+",
                    ArithOp::Sub => "-",
                };
                format!("{} {} {}", lhs.to_string(), sym, rhs.to_string())
            }
        }
    }

    pub fn evaluate(&self) -> Result<f64, Errors> {
        match self {
            Operand::Number(n) => n
                .parse::<f64>()
                .map_err(|_| Errors::XPathTraverseError(format!("Invalid number literal: {}", n))),
            Operand::Variable(name) => Err(Errors::XPathTraverseError(format!(
                "Unsubstituted variable ${}",
                name
            ))),
            Operand::Arith { op, lhs, rhs } => {
                let l = lhs.evaluate()?;
                let r = rhs.evaluate()?;
                Ok(match op {
                    ArithOp::Add => l + r,
                    ArithOp::Sub => l - r,
                })
            }
        }
    }
}

impl XPath {
    pub fn substitute(&self, variables: &Variables) -> Result<XPath, Errors> {
        Ok(XPath {
            segments: self
                .segments
                .iter()
                .map(|s| s.substitute(variables))
                .collect::<Result<Vec<_>, Errors>>()?,
            start_variable: self.start_variable.clone(),
            union: self
                .union
                .iter()
                .map(|b| b.substitute(variables))
                .collect::<Result<Vec<_>, Errors>>()?,
        })
    }

    fn extend_dedup(out: &mut Vec<Value>, incoming: Vec<Value>) {
        for value in incoming {
            if !out
                .iter()
                .any(|v| Arc::ptr_eq(&v.graph, &value.graph) && v.selection == value.selection)
            {
                out.push(value);
            }
        }
    }

    pub fn evaluate(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        variables: &Variables,
        start: Graph,
    ) -> Result<Vec<Value>, Errors> {
        let bound = self.substitute(variables)?;

        let starts: Vec<Graph> = match &bound.start_variable {
            Some(name) => variables
                .get(name)
                .ok_or_else(|| Errors::XPathTraverseError(format!("Unbound variable ${}", name)))?
                .iter()
                .map(|v| Arc::clone(&v.graph))
                .collect(),
            None => vec![Arc::clone(&start)],
        };
        let path = XPath {
            start_variable: None,
            ..bound
        };

        let mut out = Vec::new();
        for s in starts {
            out.extend(path.traverse(Arc::clone(&normalization_context), s)?);
        }
        Ok(out)
    }

    fn evaluate_branch(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        variables: &Variables,
        start: Graph,
    ) -> Result<Vec<Value>, Errors> {
        let starts: Vec<Graph> = match &self.start_variable {
            Some(name) => variables
                .get(name)
                .ok_or_else(|| Errors::XPathTraverseError(format!("Unbound variable ${}", name)))?
                .iter()
                .map(|v| Arc::clone(&v.graph))
                .collect(),
            None => vec![Arc::clone(&start)],
        };
        let path = XPath {
            start_variable: None,
            union: Vec::new(),
            segments: self.segments.clone(),
        };

        let mut out = Vec::new();
        for s in starts {
            out.extend(path.traverse_branch(Arc::clone(&normalization_context), s)?);
        }
        Ok(out)
    }

    pub fn traverse(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        start: Graph,
    ) -> Result<Vec<Value>, Errors> {
        let mut out =
            self.traverse_branch(Arc::clone(&normalization_context), Arc::clone(&start))?;
        for branch in &self.union {
            let values =
                branch.traverse_branch(Arc::clone(&normalization_context), Arc::clone(&start))?;
            Self::extend_dedup(&mut out, values);
        }
        Ok(out)
    }

    pub fn traverse_branch(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        start: Graph,
    ) -> Result<Vec<Value>, Errors> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let traversal_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() % 100000)
            .unwrap_or(0);

        let start_id = read_lock!(start).id.clone();

        log::error!("");
        log::error!(
            "╔════════════════════════════════════════════════════════════════════════════════╗"
        );
        log::error!(
            "║                 ▶ XPATH TRAVERSE (XPath.rs) START [ID: {}]                   ║",
            traversal_id
        );
        log::error!(
            "╚════════════════════════════════════════════════════════════════════════════════╝"
        );
        log::error!(
            "[{}] Starting node ID: {}",
            traversal_id,
            start_id.to_string()
        );
        log::error!("[{}] XPath: {}", traversal_id, self.to_string());
        log::error!("[{}] Total segments: {}", traversal_id, self.segments.len());
        log::error!(
            "[{}] ─────────────────────────────────────────────────────────────────────────────",
            traversal_id
        );

        let mut current: Vec<Value> = vec![Value::from_graph(Arc::clone(&start))];

        for (index, segment) in self.segments.iter().enumerate() {
            log::error!("[{}]", traversal_id);
            log::error!(
                "[{}] ┌─ SEGMENT {}/{}: {}",
                traversal_id,
                index + 1,
                self.segments.len(),
                segment.to_string()
            );
            log::error!("[{}] │  Current graphs: {}", traversal_id, current.len());

            let cache_key = (start_id.clone(), self.segments[0..=index].to_vec());

            if let Some(cached) = XPATH_CACHE.with(|cache| cache.borrow().get(&cache_key).cloned())
            {
                log::info!("[{}] │  Cache HIT for segments 0..{}", traversal_id, index);
                current = cached;
                if current.is_empty() {
                    log::error!(
                        "[{}] └─ After segment: 0 graphs (cached, empty)",
                        traversal_id
                    );
                    log::error!("[{}] ─────────────────────────────────────────────────────────────────────────────", traversal_id);
                    log::error!("╔════════════════════════════════════════════════════════════════════════════════╗");
                    log::error!("║                 ✗ XPATH TRAVERSE FAILED [ID: {}]                              ║", traversal_id);
                    log::error!("╚════════════════════════════════════════════════════════════════════════════════╝");
                    log::error!("");
                    return Ok(Vec::new());
                }
                log::error!(
                    "[{}] │  Restored {} graphs from cache",
                    traversal_id,
                    current.len()
                );
                continue;
            }

            current = current
                .iter()
                .map(|value| {
                    traverse::traverse_using_xpath_segment(
                        Arc::clone(&normalization_context),
                        &value,
                        segment,
                    )
                })
                .collect::<Result<Vec<Vec<Value>>, Errors>>()?
                .into_iter()
                .flatten()
                .collect();

            log::error!(
                "[{}] └─ After segment: {} values(s) remaining",
                traversal_id,
                current.len()
            );

            if current.is_empty() {
                log::error!(
                    "[{}] ✗ NO MATCHES after segment {}",
                    traversal_id,
                    index + 1
                );
                log::error!("[{}] ─────────────────────────────────────────────────────────────────────────────", traversal_id);
                log::error!("╔════════════════════════════════════════════════════════════════════════════════╗");
                log::error!("║                 ✗ XPATH TRAVERSE FAILED [ID: {}]                              ║", traversal_id);
                log::error!("╚════════════════════════════════════════════════════════════════════════════════╝");
                log::error!("");
                return Ok(Vec::new());
            }

            XPATH_CACHE.with(|cache| cache.borrow_mut().insert(cache_key, current.clone()));
        }

        log::error!(
            "[{}] ✓ SUCCESS - {} graph(s) matched",
            traversal_id,
            current.len()
        );
        log::error!(
            "[{}] ─────────────────────────────────────────────────────────────────────────────",
            traversal_id
        );
        log::error!(
            "╔════════════════════════════════════════════════════════════════════════════════╗"
        );
        log::error!(
            "║                 ✓ XPATH TRAVERSE SUCCESS [ID: {}]                             ║",
            traversal_id
        );
        log::error!(
            "╚════════════════════════════════════════════════════════════════════════════════╝"
        );
        log::error!("");

        Ok(current.clone())
    }

    pub fn from_str(s: &str) -> Result<Self, Errors> {
        log::debug!("xpath: {}", s);

        let branches = split_top_level_union(s.trim());
        if branches.len() > 1 {
            let mut parsed = branches
                .into_iter()
                .map(XPath::from_str)
                .collect::<Result<Vec<_>, Errors>>()?
                .into_iter();
            let mut first = parsed.next().expect("split yields at least one branch");
            first.union = parsed.collect();
            return Ok(first);
        }

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

        let start_variable = match parts.first() {
            Some(p) if p.starts_with('$') => {
                let name = &p[1..];
                if name.is_empty()
                    || !name
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
                {
                    return Err(Errors::XPathParseError(format!(
                        "Unsupported variable reference: {}",
                        p
                    )));
                }
                let name = name.to_string();
                parts.remove(0);
                Some(name)
            }
            _ => None,
        };

        let segments = parts
            .into_iter()
            .filter(|part| !part.is_empty())
            .map(XPathSegment::from_str)
            .collect::<Result<Vec<_>, Errors>>()?;

        if segments.is_empty() && start_variable.is_none() {
            return Err(Errors::XPathParseError("XPath is empty".to_string()));
        }

        Ok(XPath {
            segments,
            start_variable,
            union: Vec::new(),
        })
    }

    pub fn to_string(&self) -> String {
        let path = self
            .segments
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join("/");

        let head = match &self.start_variable {
            Some(name) if path.is_empty() => format!("${}", name),
            Some(name) => format!("${}/{}", name, path),
            None => path,
        };

        std::iter::once(head)
            .chain(self.union.iter().map(|b| b.to_string()))
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

impl XPathSegment {
    fn substitute(&self, variables: &Variables) -> Result<XPathSegment, Errors> {
        Ok(XPathSegment {
            axis: self.axis.clone(),
            node_test: self.node_test.clone(),
            predicates: self
                .predicates
                .iter()
                .map(|p| p.substitute(variables))
                .collect::<Result<Vec<_>, Errors>>()?,
        })
    }

    fn from_str(s: &str) -> Result<Self, Errors> {
        let mut rest = s.trim_end();
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
            rest = &rest[..open_pos].trim_end();
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
        } else if let Some(attr_name) = node_part.strip_prefix('@') {
            (XPathAxis::Attribute, attr_name)
        } else {
            (XPathAxis::Child, node_part)
        };

        if node_test.is_empty() {
            return Err(Errors::XPathParseError(format!(
                "Empty node test in segment: {}",
                s
            )));
        }

        let valid_name = node_test
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':' | '#'));
        let valid_kind = matches!(node_test, "*" | "text()" | "node()" | "comment()");
        if !valid_name && !valid_kind {
            return Err(Errors::XPathParseError(format!(
                "Unsupported node test '{}' in segment: {}",
                node_test, s
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
        let predicate_suffix: String = self
            .predicates
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
            "attribute" => Ok(XPathAxis::Attribute),
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
            XPathAxis::Attribute => "attribute",
            XPathAxis::Descendant => "descendant",
            XPathAxis::Ancestor => "ancestor",
            XPathAxis::FollowingSibling => "following-sibling",
            XPathAxis::PrecedingSibling => "preceding-sibling",
            XPathAxis::Following => "following",
            XPathAxis::Preceding => "preceding",
        }
    }
}

impl XPathPredicate {
    fn substitute(&self, variables: &Variables) -> Result<XPathPredicate, Errors> {
        Ok(match self {
            XPathPredicate::And(ps) => XPathPredicate::And(
                ps.iter()
                    .map(|p| p.substitute(variables))
                    .collect::<Result<Vec<_>, Errors>>()?,
            ),
            XPathPredicate::Not(p) => XPathPredicate::Not(Box::new(p.substitute(variables)?)),
            XPathPredicate::Path(path) => XPathPredicate::Path(path.substitute(variables)?),
            XPathPredicate::Equals { lhs, rhs } => XPathPredicate::Equals {
                lhs: lhs.substitute(variables)?,
                rhs: rhs.substitute(variables)?,
            },
            other => other.clone(),
        })
    }

    // Splits `s` on top-level occurrences of `sep`, i.e. ones that are not
    // nested inside parentheses (so `and` inside `contains(...)` /
    // `starts-with(...)` arguments is left alone).
    fn split_top_level<'a>(s: &'a str, sep: &str) -> Vec<&'a str> {
        let mut parts = Vec::new();
        let mut depth = 0;
        let mut start = 0;

        for (i, c) in s.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }

            if depth == 0 && s[i..].starts_with(sep) {
                parts.push(s[start..i].trim());
                start = i + sep.len();
            }
        }

        parts.push(s[start..].trim());
        parts
    }

    fn from_str(s: &str) -> Result<Self, Errors> {
        if s == "last()" {
            return Ok(XPathPredicate::Last);
        }

        let clauses = Self::split_top_level(s, " and ");
        if clauses.len() > 1 {
            let predicates = clauses
                .into_iter()
                .map(XPathPredicate::from_str)
                .collect::<Result<Vec<_>, Errors>>()?;
            return Ok(XPathPredicate::And(predicates));
        }

        let equality = Self::split_top_level(s, " = ");
        if equality.len() == 2 {
            return Ok(XPathPredicate::Equals {
                lhs: Expr::from_str(equality[0])?,
                rhs: Operand::from_str(equality[1])?,
            });
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
        } else if let Some(inner) = s
            .strip_prefix("contains(normalize-space(.)")
            .and_then(|s| s.strip_suffix(')'))
        {
            let value = inner
                .trim_start_matches(',')
                .trim()
                .trim_matches('\'')
                .trim_matches('"')
                .to_string();
            Ok(XPathPredicate::ContainsNormalized { value })
        } else if let Some((name, value)) = Self::parse_contains_token(s) {
            Ok(XPathPredicate::ContainsToken { name, value })
        } else if let Some(inner) = s
            .strip_prefix("contains(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let (attr_part, val_part) = inner.split_once(',').ok_or_else(|| {
                Errors::XPathParseError(format!("Invalid contains() predicate: {}", s))
            })?;
            let attr_part = attr_part.trim();
            if attr_part.contains('(') || attr_part == "." {
                return Err(Errors::XPathParseError(format!(
                    "Unsupported contains() argument: {}",
                    attr_part
                )));
            }
            let name = attr_part.trim().trim_start_matches('@').to_string();
            let value = val_part
                .trim()
                .trim_matches('\'')
                .trim_matches('"')
                .to_string();
            Ok(XPathPredicate::Contains { name, value })
        } else if let Some(inner) = s.strip_prefix("not(").and_then(|s| s.strip_suffix(')')) {
            Ok(XPathPredicate::Not(Box::new(XPathPredicate::from_str(
                inner,
            )?)))
        } else if let Some(inner) = s
            .strip_prefix("starts-with(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let (attr_part, val_part) = inner.split_once(',').ok_or_else(|| {
                Errors::XPathParseError(format!("Invalid starts-with() predicate: {}", s))
            })?;

            let attr_part = attr_part.trim();
            if attr_part.contains('(') || attr_part == "." {
                return Err(Errors::XPathParseError(format!(
                    "Unsupported contains() argument: {}",
                    attr_part
                )));
            }

            let name = attr_part.trim().trim_start_matches('@').to_string();
            let value = val_part
                .trim()
                .trim_matches('\'')
                .trim_matches('"')
                .to_string();
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

    fn parse_contains_token(s: &str) -> Option<(String, String)> {
        let rest = s.strip_prefix("contains(concat(' ', normalize-space(@")?;
        let (name, rest) = rest.split_once("), ' '), '")?;
        let value = rest.strip_suffix("')")?;
        let token = value.strip_prefix(' ')?.strip_suffix(' ')?;
        Some((name.to_string(), token.to_string()))
    }

    pub fn to_string(&self) -> String {
        match self {
            XPathPredicate::Position(n) => n.to_string(),
            XPathPredicate::Attribute { name, value } => format!("@{}='{}'", name, value),
            XPathPredicate::Contains { name, value } => format!("contains(@{},'{}')", name, value),
            XPathPredicate::Last => "last()".to_string(),
            XPathPredicate::AttributePresence(attrs) => attrs
                .iter()
                .map(|attr| format!("@{}", attr))
                .collect::<Vec<_>>()
                .join(" and "),
            XPathPredicate::StartsWith { name, value } => {
                format!("starts-with(@{},'{}')", name, value)
            }
            XPathPredicate::Path(path) => path.to_string(),
            XPathPredicate::And(predicates) => predicates
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(" and "),
            XPathPredicate::ContainsNormalized { value } => {
                format!("contains(normalize-space(.),'{}'')", value)
            }
            XPathPredicate::Not(pred) => format!("not({})", pred.to_string()),
            XPathPredicate::ContainsToken { name, value } => {
                format!(
                    "contains(concat(' ', normalize-space(@{}), ' '), ' {} ')",
                    name, value
                )
            }
            XPathPredicate::Equals { lhs, rhs } => {
                format!("{} = {}", lhs.to_string(), rhs.to_string())
            }
        }
    }
}

/// Splits `s` on `|` characters that are not inside brackets, parentheses or quotes.
fn split_top_level_union(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut quote: Option<char> = None;
    let mut start = 0;

    for (i, c) in s.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '\'') | (None, '"') => quote = Some(c),
            (None, '[') | (None, '(') => depth += 1,
            (None, ']') | (None, ')') => depth -= 1,
            (None, '|') if depth == 0 => {
                parts.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }

    parts.push(s[start..].trim());
    parts
}

/// Byte position and op of the rightmost ` - ` / ` + ` outside any parentheses.
fn rightmost_top_level_op(s: &str) -> Option<(usize, ArithOp)> {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut found = None;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b'-' | b'+' if depth == 0 && i > 0 && i + 1 < bytes.len() => {
                if bytes[i - 1] == b' ' && bytes[i + 1] == b' ' {
                    // i is the operator itself, so the separator starts at i - 1
                    let op = if b == b'-' {
                        ArithOp::Sub
                    } else {
                        ArithOp::Add
                    };
                    found = Some((i - 1, op));
                }
            }
            _ => {}
        }
    }
    found
}
