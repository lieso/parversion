use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use crate::prelude::*;
use crate::context::{Context, ContextID};
use crate::graph_node::Graph;
use crate::document::{Document, DocumentType};
use crate::document_format::DocumentFormat;

#[derive(Clone, Debug)]
pub struct MetaContext {
    pub contexts: HashMap<ContextID, Arc<Context>>,
    pub graph_root: Graph,
    pub contexts_lookup: HashMap<ID, Arc<Context>>,
    pub document_type: DocumentType,
    pub acyclic_subgraph_hash: Hash,
}

impl MetaContext {
    pub fn generate_context_string(&self) -> Result<String, Errors> {
        let spatial_context = self.generate_spatial_context()?;

        Ok(spatial_context)
    }

    fn generate_spatial_context(&self) -> Result<String, Errors> {
        let max_lineages: usize = 1;
        let render_ids = get_render_ids(
            self.graph_root.clone(),
            &max_lineages
        );

        let partial_document = Document::from_meta_context(
            self,
            &DocumentFormat {
                format_type: self.document_type.clone(),
                encoding: Some(String::from("UTF-8")),
                indent: None,
                line_ending: None,
                headers: None,
                wrap_text: None,
                exclude_nulls: None,
                custom_delimiter: None,
            },
            Some(&render_ids),
        )?;

        Ok(partial_document.to_string())
    }
}

fn get_render_ids(start_node: Graph, max_lineages: &usize) -> HashSet<GraphNodeID> {
    let mut render_ids: HashSet<GraphNodeID> = HashSet::new();
    let mut lineage_counts: HashMap<Lineage, usize> = HashMap::new();

    let mut queue: VecDeque<Graph> = VecDeque::new();
    queue.push_back(Arc::clone(&start_node));

    while let Some(node) = queue.pop_front() {
        let lock = read_lock!(node);
        let lineage = &lock.lineage;

        let count = lineage_counts.entry(lineage.clone()).or_insert(0);
        
        if *count < *max_lineages {
            *count += 1;
            render_ids.insert(lock.id.clone());
        }

        for child in &lock.children {
            queue.push_back(Arc::clone(&child));
        }
    }

    render_ids
}
