use anyhow::{Context, Result};
use tree_sitter::{Language, Parser, Tree};

pub struct RustParser {
    parser: Parser,
}

impl RustParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        // tree-sitter 0.22+ API: LanguageFn into_raw() -> Language
        let language: Language = tree_sitter_rust::LANGUAGE.into();
        parser
            .set_language(&language)
            .context("Error loading Rust grammar")?;
        Ok(Self { parser })
    }

    pub fn parse(&mut self, code: &str) -> Option<Tree> {
        self.parser.parse(code, None)
    }
}
