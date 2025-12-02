// tests/test_harness.rs
//! Test harness infrastructure for AngelScript parser integration tests
//!
//! This module provides utilities for loading and testing AngelScript files,
//! validating parse results, and checking error conditions.

use angelscript::*;
use bumpalo::Bump;
use std::fs;
use std::path::PathBuf;

/// Test result that includes parsed AST and any errors
pub struct TestResult<'src, 'ast> {
    pub arena: Bump,
    pub script: Script<'src, 'ast>,
    pub errors: Vec<ParseError>,
    pub source: String,
}

/// Test harness for loading and parsing AngelScript files
pub struct TestHarness {
    test_scripts_dir: PathBuf,
}

impl TestHarness {
    /// Create a new test harness
    pub fn new() -> Self {
        let test_scripts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_scripts");
        Self { test_scripts_dir }
    }

    /// Load and parse an AngelScript file
    pub fn load_and_parse<'src, 'ast>(&self, filename: &str) -> TestResult<'src, 'ast>
    where
        'src: 'ast,
    {
        let arena = Bump::new();
        let path = self.test_scripts_dir.join(filename);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));

        // SAFETY: We're transmuting the lifetimes to match TestResult's requirements.
        // The arena is owned by TestResult, so the script remains valid.
        let (script, errors) = unsafe {
            let src_ptr = source.as_str() as *const str;
            let (s, e) = parse_lenient(&*src_ptr, &arena);
            (std::mem::transmute(s), e)
        };

        TestResult {
            arena,
            script,
            errors,
            source,
        }
    }
}

impl<'src, 'ast> TestResult<'src, 'ast> {
    /// Assert that parsing succeeded with no errors
    pub fn assert_success(&self) {
        if !self.errors.is_empty() {
            eprintln!("Source:\n{}", self.source);
            eprintln!("\nErrors:");
            for err in &self.errors {
                eprintln!("{}", err.display_with_source(&self.source));
            }
            panic!(
                "Expected successful parse, but got {} errors",
                self.errors.len()
            );
        }
    }

    /// Get items of a specific type
    pub fn get_functions(&self) -> Vec<&FunctionDecl<'_, '_>> {
        self.script
            .items()
            .iter()
            .filter_map(|item| {
                if let Item::Function(f) = item {
                    Some(f)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_classes(&self) -> Vec<&ClassDecl<'_, '_>> {
        self.script
            .items()
            .iter()
            .filter_map(|item| {
                if let Item::Class(c) = item {
                    Some(c)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_interfaces(&self) -> Vec<&InterfaceDecl<'_, '_>> {
        self.script
            .items()
            .iter()
            .filter_map(|item| {
                if let Item::Interface(i) = item {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_enums(&self) -> Vec<&EnumDecl<'_, '_>> {
        self.script
            .items()
            .iter()
            .filter_map(|item| {
                if let Item::Enum(e) = item {
                    Some(e)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_global_vars(&self) -> Vec<&GlobalVarDecl<'_, '_>> {
        self.script
            .items()
            .iter()
            .filter_map(|item| {
                if let Item::GlobalVar(v) = item {
                    Some(v)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if source contains a specific substring
    pub fn source_contains(&self, substring: &str) -> bool {
        self.source.contains(substring)
    }
}

/// Helper for counting AST nodes
pub struct AstCounter {
    pub function_count: usize,
    pub class_count: usize,
    pub if_count: usize,
    pub while_count: usize,
    pub for_count: usize,
    pub binary_expr_count: usize,
    pub call_count: usize,
    pub lambda_expr_count: usize,
}

impl AstCounter {
    pub fn new() -> Self {
        Self {
            function_count: 0,
            class_count: 0,
            if_count: 0,
            while_count: 0,
            for_count: 0,
            binary_expr_count: 0,
            call_count: 0,
            lambda_expr_count: 0,
        }
    }

    /// Count nodes in a script
    pub fn count_script(mut self, script: &Script) -> Self {
        for item in script.items() {
            self.count_item(item);
        }
        self
    }

    fn count_item(&mut self, item: &Item) {
        match item {
            Item::Function(f) => {
                self.function_count += 1;
                if let Some(body) = &f.body {
                    self.count_block(body);
                }
            }
            Item::Class(c) => {
                self.class_count += 1;
                for member in c.members {
                    if let ClassMember::Method(m) = member {
                        self.function_count += 1;
                        if let Some(body) = &m.body {
                            self.count_block(body);
                        }
                    }
                }
            }
            Item::Namespace(ns) => {
                // ✅ NEW: Recursively count items inside the namespace
                for item in ns.items {
                    self.count_item(item);
                }
            }
            _ => {}
        }
    }

    fn count_block(&mut self, block: &Block) {
        for stmt in block.stmts {
            self.count_stmt(stmt);
        }
    }

    fn count_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::If(s) => {
                self.if_count += 1;
                self.count_expr(&s.condition);
                self.count_stmt(&s.then_stmt);
                if let Some(else_stmt) = &s.else_stmt {
                    self.count_stmt(else_stmt);
                }
            }
            Stmt::While(s) => {
                self.while_count += 1;
                self.count_expr(&s.condition);
                self.count_stmt(&s.body);
            }
            Stmt::For(s) => {
                self.for_count += 1;
                if let Some(init) = &s.init {
                    match init {
                        ForInit::VarDecl(v) => {
                            for var in v.vars {
                                if let Some(init_expr) = &var.init {
                                    self.count_expr(init_expr);
                                }
                            }
                        }
                        ForInit::Expr(e) => self.count_expr(e),
                    }
                }
                if let Some(cond) = &s.condition {
                    self.count_expr(cond);
                }
                for update in s.update {
                    self.count_expr(update);
                }
                self.count_stmt(&s.body);
            }
            Stmt::Expr(e) => {
                if let Some(expr) = &e.expr {
                    self.count_expr(expr);
                }
            }
            Stmt::Block(b) => self.count_block(b),
            Stmt::VarDecl(v) => {
                for var in v.vars {
                    if let Some(init) = &var.init {
                        self.count_expr(init);
                    }
                }
            }
            _ => {}
        }
    }

    fn count_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Binary(b) => {
                self.binary_expr_count += 1;
                self.count_expr(&b.left);
                self.count_expr(&b.right);
            }
            Expr::Call(c) => {
                self.call_count += 1;
                self.count_expr(&c.callee);
                for arg in c.args {
                    self.count_expr(&arg.value);
                }
            }
            Expr::Unary(u) => self.count_expr(&u.operand),
            Expr::Index(i) => {
                self.count_expr(&i.object);
                for idx in i.indices {
                    self.count_expr(&idx.index);
                }
            }
            Expr::Member(m) => self.count_expr(&m.object),
            Expr::Ternary(t) => {
                self.count_expr(&t.condition);
                self.count_expr(&t.then_expr);
                self.count_expr(&t.else_expr);
            }
            Expr::Assign(a) => {
                self.count_expr(&a.target);
                self.count_expr(&a.value);
            }
            Expr::Postfix(p) => self.count_expr(&p.operand),
            Expr::Cast(c) => self.count_expr(&c.expr),
            Expr::Paren(p) => self.count_expr(&p.expr),
            Expr::Lambda(l) => {
                self.lambda_expr_count += 1;
                // Count expressions in lambda body
                for stmt in l.body.stmts {
                    self.count_stmt(stmt);
                }
            }
            _ => {}
        }
    }
}
