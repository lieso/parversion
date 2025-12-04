use std::sync::{Arc, RwLock};
use swc_common::sync::Lrc;
use swc_common::{SourceMap, FileName, BytePos, SyntaxContext};
use swc_ecma_parser::{Syntax, Parser, StringInput};
use swc_ecma_parser::lexer::Lexer;
use swc_ecma_ast::Program;
use swc_ecma_ast::*;
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith, VisitWith};
use swc_ecma_codegen::{Emitter, text_writer::JsWriter};
use std::rc::Rc;
use std::collections::HashMap;
use swc_atoms::Atom;

use crate::prelude::*;
use crate::document::{Document, DocumentType};
use crate::provider::Provider;
use crate::meta_context::MetaContext;
use crate::mutations::Mutations;

pub async fn reduce<P: Provider>(
    provider: Arc<P>,
    mut document: Document,
    _options: &Option<Options>,
) -> Result<Arc<RwLock<MetaContext>>, Errors> {
    log::trace!("In reduce");

    unimplemented!()
}

pub async fn reduce_text_to_mutations<P: Provider>(
    provider: Arc<P>,
    text: String,
    _options: &Option<Options>,
    document_type: DocumentType,
) -> Result<Mutations, Errors> {
    log::trace!("In reduce_text_to_mutations");








    let cm: Lrc<SourceMap> = Default::default();

    let source_file = cm.new_source_file(Rc::new(FileName::Custom("inline.js".into())), text.to_string());

    let lexer = Lexer::new(
        Syntax::Es(Default::default()),
        Default::default(),
        StringInput::from(&*source_file),
        None,
    );

    let mut parser = Parser::new_from(lexer);

    match parser.parse_program() {
        Ok(program) => {
            explore_with_visitor(&program);
        },
        Err(e) => {
            log::info!("Document is not javascript");
        }
    }













    unimplemented!()
}

pub async fn reduce_url_to_mutations<P: Provider>(
    provider: Arc<P>,
    url: &str,
    _options: &Option<Options>,
    document_type: DocumentType,
) -> Result<Mutations, Errors> {
    log::trace!("In reduce_url_to_mutations");

    unimplemented!()
}


pub async fn reduce_file_to_mutations<P: Provider>(
    provider: Arc<P>,
    path: &str,
    _options: &Option<Options>,
    document_type: DocumentType,
) -> Result<Mutations, Errors> {
    log::trace!("In reduce_file_to_mutations");

    unimplemented!()
}














struct AstExplorer {
    pub fn_count: i64,
    pub hash_count: HashMap<String, usize>,
    pub cm: Lrc<SourceMap>,
}

impl Visit for AstExplorer {
    fn visit_function(&mut self, f: &Function) {
        self.fn_count += 1;



        let mut cloned_fn = f.clone();



        let mut normalizer = Normalizer::default();
        cloned_fn.visit_mut_with(&mut normalizer);





        let func_decl = FnDecl {
            ident: Ident::new("fn".into(), f.span, SyntaxContext::empty()),
            declare: false,
            function: Box::new(cloned_fn),
        };

        let module = Module {
            span: f.span,
            body: vec![ModuleItem::Stmt(Stmt::Decl(Decl::Fn(func_decl)))],
            shebang: None,
        };

        let mut buf = Vec::new();
        {
            let writer = JsWriter::new(self.cm.clone(), "\n", &mut buf, None);
            let mut emitter = Emitter {
                cfg: Default::default(),
                comments: None,
                cm: self.cm.clone(),
                wr: Box::new(writer),
            };

            emitter.emit_module(&module).expect("emit failed");
        }

        let output = String::from_utf8(buf).expect("non-utf8 output from emitter");


        log::debug!("PRETTY FUNCTION:\n{}", output);


        let hash = Hash::from_str(&output);
        
        log::debug!("hash: {}", hash.to_string().unwrap());


        *self.hash_count
            .entry(hash.to_string().unwrap())
            .or_insert(0) += 1;


        f.visit_children_with(self);
    }

    fn visit_fn_decl(&mut self, n: &FnDecl) {
        n.visit_children_with(self);
    }

    fn visit_var_decl(&mut self, n: &VarDecl) {
        n.visit_children_with(self);
    }

    fn visit_class_decl(&mut self, n: &ClassDecl) {
        n.visit_children_with(self);
    }

    fn visit_expr(&mut self, n: &Expr) {
        if let Expr::Call(call) = n {
            call.visit_children_with(self);
            return;
        }
        n.visit_children_with(self);
    }
}

#[derive(Default)]
struct Normalizer {
    rename_map: HashMap<Atom, Atom>,
}

impl VisitMut for Normalizer {
    fn visit_mut_function(&mut self, f: &mut Function) {

        for (idx, param) in f.params.iter_mut().enumerate() {
            if let Pat::Ident(bi) = &mut param.pat {
                let old = bi.id.sym.clone();
                let new: Atom = format!("p{}", idx).into();
                bi.id.sym = new.clone();
                self.rename_map.insert(old, new);
            }
        }

        if let Some(body) = &mut f.body {
            for stmt in &mut body.stmts {
                if let Stmt::Decl(Decl::Var(var_decl)) = stmt {
                    for (idx, decl) in var_decl.decls.iter_mut().enumerate() {
                        if let Pat::Ident(bi) = &mut decl.name {
                            let old = bi.id.sym.clone();
                            let new: Atom = format!("v{}", idx).into();
                            bi.id.sym = new.clone();
                            self.rename_map.insert(old, new);
                        }
                    }
                }
            }

            body.visit_mut_with(self);
        }
    }

    fn visit_mut_ident(&mut self, i: &mut Ident) {
        if let Some(new) = self.rename_map.get(&i.sym) {
            i.sym = new.clone();
        }
    }
}





fn explore_with_visitor(program: &Program) {
     let cm: Lrc<SourceMap> = Default::default();

     let mut explorer = AstExplorer {
         fn_count: 0,
         hash_count: HashMap::new(),
         cm,
     };

     program.visit_with(&mut explorer);
     println!("fn count: {}", explorer.fn_count);
     println!("hash count: {:?}", explorer.hash_count);
     println!("hash count: {}", explorer.hash_count.len());
 }

