use insta::assert_debug_snapshot;
use pilot::oracle::graph::GraphBuilder;

#[test]
fn test_rust_extraction() {
    let code = r#"
        struct MyStruct {
            field: i32,
        }

        impl MyStruct {
            fn new() -> Self {
                Self { field: 0 }
            }
        }
    "#;

    let mut builder = GraphBuilder::new().expect("Failed to init builder");
    let nodes = builder.extract_nodes("test.rs", code);

    // Sort nodes to ensure consistent order for snapshot
    // But extraction order is usually tree-order traversal which is deterministic.

    assert_debug_snapshot!(nodes);
}
