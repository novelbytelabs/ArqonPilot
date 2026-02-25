use crate::parser::RustParser;
use crate::parser_py::PythonParser;
use anyhow::Result;
use tree_sitter::{Node, TreeCursor};

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub source_node_name: String, // Simplified linking by name
    pub target_node_name: String,
    pub edge_type: String, // "calls", "imports"
}

pub struct EdgeBuilder {
    rust_parser: RustParser,
    _python_parser: PythonParser,
}

impl EdgeBuilder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            rust_parser: RustParser::new()?,
            _python_parser: PythonParser::new()?,
        })
    }

    pub fn extract_edges(&mut self, path: &str, content: &str) -> Vec<GraphEdge> {
        if path.ends_with(".rs") {
            self.extract_rust(path, content)
        } else {
            vec![]
        }
    }

    fn extract_rust(&mut self, _path: &str, content: &str) -> Vec<GraphEdge> {
        let mut edges = Vec::new();
        if let Some(tree) = self.rust_parser.parse(content) {
            let mut cursor = tree.walk();
            Self::visit_rust_node(&mut cursor, content, &mut edges, None);
        }
        edges
    }

    fn visit_rust_node(
        cursor: &mut TreeCursor,
        content: &str,
        edges: &mut Vec<GraphEdge>,
        current_scope: Option<String>,
    ) {
        let node = cursor.node();
        let kind = node.kind();

        // Track current function scope
        let mut new_scope = current_scope.clone();
        if kind == "function_item" {
            if let Some(name_node) = node.child_by_field_name("name") {
                new_scope = Some(extract_text(name_node, content));
            }
        }

        // Find calls
        if kind == "call_expression" {
            if let Some(scope) = &current_scope {
                // Try to find the function being called
                // In Rust AST: call_expression -> function: identifier
                if let Some(func_node) = node.child_by_field_name("function") {
                    let called_name = extract_text(func_node, content);
                    edges.push(GraphEdge {
                        source_node_name: scope.clone(),
                        target_node_name: called_name,
                        edge_type: "calls".to_string(),
                    });
                }
            }
        }

        if cursor.goto_first_child() {
            loop {
                Self::visit_rust_node(cursor, content, edges, new_scope.clone());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
}

fn extract_text(node: Node, content: &str) -> String {
    content[node.byte_range()].to_string()
}
