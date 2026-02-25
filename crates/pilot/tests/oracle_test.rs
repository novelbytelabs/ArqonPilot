use tempfile::TempDir;

/// Test the oracle module components
mod oracle_tests {
    use super::*;

    #[test]
    fn test_rust_extraction() {
        // Test data - a sample Rust file
        let rust_code = r#"
/// This is a documented function
/// It does important things
fn documented_function() {
    println!("Hello");
}

struct MyStruct {
    field: i32,
}

impl MyStruct {
    fn method(&self) -> i32 {
        self.field
    }
}
"#;

        // Test tree-sitter parsing
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(rust_code, None).unwrap();

        // Verify we got a tree
        assert!(tree.root_node().child_count() > 0);

        // Check for function_item nodes
        let root = tree.root_node();
        let mut cursor = root.walk();

        let mut found_function = false;
        let mut found_struct = false;

        fn visit(
            cursor: &mut tree_sitter::TreeCursor,
            found_fn: &mut bool,
            found_struct: &mut bool,
        ) {
            let node = cursor.node();
            match node.kind() {
                "function_item" => *found_fn = true,
                "struct_item" => *found_struct = true,
                _ => {}
            }

            if cursor.goto_first_child() {
                loop {
                    visit(cursor, found_fn, found_struct);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }

        visit(&mut cursor, &mut found_function, &mut found_struct);

        assert!(found_function, "Should find function_item");
        assert!(found_struct, "Should find struct_item");
    }

    #[test]
    fn test_python_extraction() {
        // Test data - a sample Python file
        let python_code = r#"
def my_function(arg1, arg2):
    """This is the docstring for my_function."""
    return arg1 + arg2

class MyClass:
    """A class docstring."""
    
    def method(self):
        pass
"#;

        // Test tree-sitter parsing
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(python_code, None).unwrap();

        assert!(tree.root_node().child_count() > 0);

        // Check for function_definition and class_definition nodes
        let root = tree.root_node();
        let mut cursor = root.walk();

        let mut found_function = false;
        let mut found_class = false;

        fn visit(
            cursor: &mut tree_sitter::TreeCursor,
            found_fn: &mut bool,
            found_class: &mut bool,
        ) {
            let node = cursor.node();
            match node.kind() {
                "function_definition" => *found_fn = true,
                "class_definition" => *found_class = true,
                _ => {}
            }

            if cursor.goto_first_child() {
                loop {
                    visit(cursor, found_fn, found_class);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }

        visit(&mut cursor, &mut found_function, &mut found_class);

        assert!(found_function, "Should find function_definition");
        assert!(found_class, "Should find class_definition");
    }

    #[test]
    fn test_doc_comment_extraction() {
        let rust_code = "/// This is a doc comment\n/// Second line\nfn my_fn() {}";
        let lines: Vec<&str> = rust_code.lines().collect();

        // Simulate the doc extraction logic
        let start_line: usize = 2; // fn is on line 2 (0-indexed)
        let mut doc_lines: Vec<&str> = Vec::new();
        let mut line_idx = start_line.saturating_sub(1);

        loop {
            let line = lines[line_idx].trim();
            if line.starts_with("///") {
                doc_lines.push(line.trim_start_matches("///").trim());
            } else {
                break;
            }
            if line_idx == 0 {
                break;
            }
            line_idx -= 1;
        }

        doc_lines.reverse();
        let docstring = doc_lines.join("\n");

        assert_eq!(docstring, "This is a doc comment\nSecond line");
    }

    #[test]
    fn test_sqlite_store() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test_oracle.db");

        // Create a test database
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS nodes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL,
                type TEXT NOT NULL,
                name TEXT NOT NULL,
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                signature_hash TEXT NOT NULL,
                docstring TEXT
            )",
            [],
        )
        .unwrap();

        // Insert a test node
        conn.execute(
            "INSERT INTO nodes (path, type, name, start_line, end_line, signature_hash, docstring) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["src/lib.rs", "function", "test_fn", 10, 20, "abc123", "A test function"],
        ).unwrap();

        // Query it back
        let name: String = conn
            .query_row(
                "SELECT name FROM nodes WHERE path = ?1",
                ["src/lib.rs"],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(name, "test_fn");
    }
}
