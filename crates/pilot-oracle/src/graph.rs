use crate::parser::RustParser;
use crate::parser_py::PythonParser;
use anyhow::Result;
use sha2::{Digest, Sha256};
use tree_sitter::{Node, TreeCursor};

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub path: String,
    pub node_type: String,
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub signature_hash: String,
    pub docstring: Option<String>,
}

pub struct GraphBuilder {
    rust_parser: RustParser,
    _python_parser: PythonParser,
}

impl GraphBuilder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            rust_parser: RustParser::new()?,
            _python_parser: PythonParser::new()?,
        })
    }

    pub fn extract_nodes(&mut self, path: &str, content: &str) -> Vec<GraphNode> {
        if path.ends_with(".rs") {
            self.extract_rust(path, content)
        } else if path.ends_with(".py") {
            self.extract_python(path, content)
        } else {
            vec![]
        }
    }

    fn extract_rust(&mut self, path: &str, content: &str) -> Vec<GraphNode> {
        let mut nodes = Vec::new();
        if let Some(tree) = self.rust_parser.parse(content) {
            let mut cursor = tree.walk();
            self.visit_rust_node(&mut cursor, path, content, &mut nodes);
        }
        nodes
    }

    fn visit_rust_node(
        &self,
        cursor: &mut TreeCursor,
        path: &str,
        content: &str,
        nodes: &mut Vec<GraphNode>,
    ) {
        let node = cursor.node();
        let kind = node.kind();

        if kind == "function_item" || kind == "struct_item" || kind == "impl_item" {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = extract_text(name_node, content);
                let signature = extract_text(node, content); // Simplified: full text as signature hash source
                let hash = compute_hash(&signature);

                // Extract doc comments (/// or //!)
                let docstring = self.extract_rust_docstring(&node, content);

                nodes.push(GraphNode {
                    path: path.to_string(),
                    node_type: kind.replace("_item", ""), // "function", "struct", "impl"
                    name,
                    start_line: node.start_position().row,
                    end_line: node.end_position().row,
                    signature_hash: hash,
                    docstring,
                });
            }
        }

        if cursor.goto_first_child() {
            loop {
                self.visit_rust_node(cursor, path, content, nodes);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    fn extract_rust_docstring(&self, node: &Node, content: &str) -> Option<String> {
        // Look for preceding sibling line_comment nodes that start with /// or //!
        let lines: Vec<&str> = content.lines().collect();
        let start_line = node.start_position().row;

        if start_line == 0 {
            return None;
        }

        let mut doc_lines = Vec::new();
        let mut line_idx = start_line.saturating_sub(1);

        // Collect doc comments going backwards
        loop {
            if line_idx >= lines.len() {
                break;
            }
            let line = lines[line_idx].trim();
            if line.starts_with("///") {
                doc_lines.push(line.trim_start_matches("///").trim());
            } else if line.starts_with("//!") {
                doc_lines.push(line.trim_start_matches("//!").trim());
            } else if line.is_empty() {
                // Skip empty lines
            } else {
                break; // Non-doc-comment line
            }
            if line_idx == 0 {
                break;
            }
            line_idx -= 1;
        }

        if doc_lines.is_empty() {
            None
        } else {
            doc_lines.reverse();
            Some(doc_lines.join("\n"))
        }
    }

    fn extract_python(&mut self, path: &str, content: &str) -> Vec<GraphNode> {
        let mut nodes = Vec::new();
        if let Some(tree) = self._python_parser.parse(content) {
            let mut cursor = tree.walk();
            self.visit_python_node(&mut cursor, path, content, &mut nodes);
        }
        nodes
    }

    fn visit_python_node(
        &self,
        cursor: &mut TreeCursor,
        path: &str,
        content: &str,
        nodes: &mut Vec<GraphNode>,
    ) {
        let node = cursor.node();
        let kind = node.kind();

        // Extract functions and classes
        if kind == "function_definition" || kind == "class_definition" {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = extract_text(name_node, content);
                let signature = extract_text(node, content);
                let hash = compute_hash(&signature);

                // Try to extract docstring (first statement if it's a string)
                let docstring = self.extract_python_docstring(&node, content);

                nodes.push(GraphNode {
                    path: path.to_string(),
                    node_type: if kind == "function_definition" {
                        "function"
                    } else {
                        "class"
                    }
                    .to_string(),
                    name,
                    start_line: node.start_position().row,
                    end_line: node.end_position().row,
                    signature_hash: hash,
                    docstring,
                });
            }
        }

        if cursor.goto_first_child() {
            loop {
                self.visit_python_node(cursor, path, content, nodes);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    fn extract_python_docstring(&self, node: &Node, content: &str) -> Option<String> {
        // Look for body -> first child that is expression_statement -> string
        if let Some(body) = node.child_by_field_name("body") {
            if let Some(first_stmt) = body.child(0) {
                if first_stmt.kind() == "expression_statement" {
                    if let Some(string_node) = first_stmt.child(0) {
                        if string_node.kind() == "string" {
                            let text = extract_text(string_node, content);
                            // Clean up triple quotes
                            let clean = text.trim_matches('"').trim_matches('\'').trim();
                            return Some(clean.to_string());
                        }
                    }
                }
            }
        }
        None
    }
}

fn extract_text(node: Node, content: &str) -> String {
    content[node.byte_range()].to_string()
}

fn compute_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text);
    format!("{:x}", hasher.finalize())
}
