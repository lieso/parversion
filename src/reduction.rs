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
use std::collections::{HashMap, HashSet};
use swc_atoms::Atom;
use std::fs;
use std::path::Path;
use std::io;

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









    let program =  parse(text);
    explore(&program);













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
    pub ignore_hash: HashSet<String>,
    pub hash_ignore_count: i64,
    pub cm: Lrc<SourceMap>,
}

impl AstExplorer {
    fn emit_stmt(&self, stmt: Stmt, span: swc_common::Span) -> String {
        let module = Module {
            span,
            body: vec![ModuleItem::Stmt(stmt)],
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

        String::from_utf8(buf).expect("non-utf8 output from emitter")
    }
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




        let hash = Hash::from_str(&output);
        
        let hash_string = hash.to_string().unwrap();

        if !self.ignore_hash.contains(&hash_string) {

            log::debug!("hash: {}", hash_string);
            log::debug!("PRETTY FUNCTION:\n{}", output);

            *self.hash_count
                .entry(hash_string)
                .or_insert(0) += 1;

        } else {
            self.hash_ignore_count += 1;
        }


        f.visit_children_with(self);
    }

    fn visit_fn_decl(&mut self, n: &FnDecl) {
        n.visit_children_with(self);
    }

    fn visit_var_decl(&mut self, n: &VarDecl) {
        let stmt = Stmt::Decl(Decl::Var(Box::new(n.clone())));
        let output = self.emit_stmt(stmt, n.span);
        log::debug!("VAR DECL:\n{}", output);

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






#[derive(Debug, Clone)]
enum JavaScriptValue {
    Indeterminate, // unknowable, null, undefined
    Bool(bool),
    Number(f64),
    String(String),
}

struct ValueCollector {
    pub values: HashMap<String, JavaScriptValue>,
    pub cm: Lrc<SourceMap>,
}

impl ValueCollector {
    fn resolve_expr(&mut self, expr: &Expr) -> JavaScriptValue {
        match expr {
            Expr::Lit(lit) => match lit {
                Lit::Str(s) => JavaScriptValue::String(s.value.as_str().unwrap().to_string()),
                Lit::Num(n) => JavaScriptValue::Number(n.value),
                Lit::Bool(b) => JavaScriptValue::Bool(b.value),
                _ => JavaScriptValue::Indeterminate,
            }
            Expr::Ident(id) => {
                let name = id.sym.to_string();
                self.values.get(&name).cloned().unwrap_or(JavaScriptValue::Indeterminate)
            }
            Expr::Bin(bin) => {
                let left = self.resolve_expr(&bin.left);
                let right = self.resolve_expr(&bin.right);

                match (bin.op, left, right) {
                    (BinaryOp::Add, JavaScriptValue::Number(a), JavaScriptValue::Number(b)) => {
                        JavaScriptValue::Number(a + b)
                    }
                    (BinaryOp::Add, JavaScriptValue::String(a), JavaScriptValue::String(b)) => {
                        JavaScriptValue::String(format!("{}{}", a, b))
                    }
                    _ => JavaScriptValue::Indeterminate
                }
            }
            Expr::Object(obj) => {
                for prop in &obj.props {
                    match prop {
                        PropOrSpread::Prop(p) => {
                            if let Prop::KeyValue(kv) = &**p {
                                if let Some(key_str) = self.resolve_prop_name(&kv.key) {
                                    let value = self.resolve_expr(&kv.value);
                                    self.values.insert(key_str, value);
                                }
                            }

                        }
                        _ => {}
                    }
                }

                JavaScriptValue::Indeterminate
            }
            _ => JavaScriptValue::Indeterminate
        }
    }

    fn resolve_prop_name(&self, name: &PropName) -> Option<String> {
        match name {
            PropName::Ident(i) => Some(i.sym.to_string()),
            PropName::Str(s) => Some(s.value.as_str().unwrap().to_string()),
            _ => None,
        }
    }

    fn bind_pattern(&mut self, pat: &Pat, value: JavaScriptValue) {
        match pat {
            Pat::Ident(bi) => {
                let name = bi.id.sym.to_string();
                self.values.insert(name, value);
            }
            Pat::Array(arr) => {
                for elem in &arr.elems {
                    if let Some(p) = elem {
                        self.bind_pattern(&p, JavaScriptValue::Indeterminate);
                    }
                }
            }
            Pat::Object(obj) => {
                for prop in &obj.props {
                    match prop {
                        ObjectPatProp::Assign(assign) => {
                            let name = assign.key.sym.to_string();
                            self.values.insert(name, JavaScriptValue::Indeterminate);
                        }
                        ObjectPatProp::KeyValue(kv) => {
                            self.bind_pattern(&kv.value, JavaScriptValue::Indeterminate);
                        }
                        ObjectPatProp::Rest(rest) => {
                            self.bind_pattern(&rest.arg, JavaScriptValue::Indeterminate);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl Visit for ValueCollector {
    fn visit_var_decl(&mut self, n: &VarDecl) {
        for decl in &n.decls {
            let value = if let Some(init) = &decl.init {
                self.resolve_expr(&*init)
            } else {
                JavaScriptValue::Indeterminate
            };

            self.bind_pattern(&decl.name, value);
        }

        n.visit_children_with(self);
    }

    fn visit_fn_decl(&mut self, n: &FnDecl) {
        let name = n.ident.sym.to_string();

        self.values.insert(name, JavaScriptValue::Indeterminate);
        n.visit_children_with(self);
    }
}









fn parse(text: String) -> Program {
    let cm: Lrc<SourceMap> = Default::default();

    let source_file = cm.new_source_file(Rc::new(FileName::Custom("inline.js".into())), text.to_string());

    let lexer = Lexer::new(
        Syntax::Es(Default::default()),
        Default::default(),
        StringInput::from(&*source_file),
        None,
    );

    let mut parser = Parser::new_from(lexer);

    parser.parse_program().expect("Could not parse program")
}



fn explore(target_program: &Program) {
     let cm1: Lrc<SourceMap> = Default::default();


     //let path = Path::new("./js/min.js");
     //let contents = fs::read_to_string(path).expect("Could not load lodash");

     //let program = parse(contents);
     //let mut explorer = AstExplorer {
     //    fn_count: 0,
     //    hash_count: HashMap::new(),
     //    ignore_hash: HashSet::new(),
     //    hash_ignore_count: 0,
     //    cm: cm1,
     //};

     //program.visit_with(&mut explorer);
     //println!("fn count: {}", explorer.fn_count);
     //println!("hash count: {}", explorer.hash_count.len());






     let cm3: Lrc<SourceMap> = Default::default();

     let mut collector = ValueCollector {
         values: HashMap::new(),
         cm: cm3,
     };

     target_program.visit_with(&mut collector);

     log::debug!("collector: {:?}", collector.values);














     //let cm2: Lrc<SourceMap> = Default::default();

     //let mut ignore_hash = HashSet::new();
     //ignore_hash.extend(explorer.hash_count.keys().cloned());

     //let mut explorer = AstExplorer {
     //    fn_count: 0,
     //    hash_count: HashMap::new(),
     //    ignore_hash,
     //    hash_ignore_count: 0,
     //    cm: cm2
     //};

     //target_program.visit_with(&mut explorer);
     //println!("hash count: {}", explorer.hash_count.len());
     //println!("hash ignore count: {}", explorer.hash_ignore_count);






 }

