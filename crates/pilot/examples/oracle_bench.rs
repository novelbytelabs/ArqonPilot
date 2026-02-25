use std::time::Instant;

/// Simple benchmarks for the Oracle module
/// Run with: cargo run -p ship --example oracle_bench
fn main() {
    println!("Oracle Benchmarks\n");
    println!("================\n");

    // 1. Tree-sitter Parsing Speed
    benchmark_rust_parsing();
    benchmark_python_parsing();

    // 2. Hash computation speed
    benchmark_hashing();
}

fn benchmark_rust_parsing() {
    let rust_code = include_str!("../src/oracle/mod.rs");

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();

    let iterations = 100;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = parser.parse(rust_code, None);
    }

    let elapsed = start.elapsed();
    let per_parse = elapsed.as_micros() / iterations;

    println!("Rust Parsing:");
    println!("  File size: {} bytes", rust_code.len());
    println!("  Iterations: {}", iterations);
    println!("  Total time: {:?}", elapsed);
    println!("  Per parse: {} µs", per_parse);
    println!(
        "  Throughput: {:.0} parses/sec\n",
        1_000_000.0 / per_parse as f64
    );
}

fn benchmark_python_parsing() {
    // Sample Python code
    let python_code = r#"
import os
import sys

def main():
    """Main entry point."""
    print("Hello, World!")

class Greeter:
    """A class for greeting."""
    
    def __init__(self, name):
        self.name = name
    
    def greet(self):
        return f"Hello, {self.name}!"

if __name__ == "__main__":
    main()
"#;

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .unwrap();

    let iterations = 1000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = parser.parse(python_code, None);
    }

    let elapsed = start.elapsed();
    let per_parse = elapsed.as_micros() / iterations;

    println!("Python Parsing:");
    println!("  File size: {} bytes", python_code.len());
    println!("  Iterations: {}", iterations);
    println!("  Total time: {:?}", elapsed);
    println!("  Per parse: {} µs", per_parse);
    println!(
        "  Throughput: {:.0} parses/sec\n",
        1_000_000.0 / per_parse as f64
    );
}

fn benchmark_hashing() {
    use sha2::{Digest, Sha256};

    let sample = "fn example_function(arg1: i32, arg2: String) -> Result<()>";
    let iterations = 10000;

    let start = Instant::now();

    for _ in 0..iterations {
        let mut hasher = Sha256::new();
        hasher.update(sample.as_bytes());
        let _ = format!("{:x}", hasher.finalize());
    }

    let elapsed = start.elapsed();
    let per_hash = elapsed.as_nanos() / iterations as u128;

    println!("SHA256 Hashing:");
    println!("  Input size: {} bytes", sample.len());
    println!("  Iterations: {}", iterations);
    println!("  Total time: {:?}", elapsed);
    println!("  Per hash: {} ns", per_hash);
    println!(
        "  Throughput: {:.0} hashes/sec\n",
        1_000_000_000.0 / per_hash as f64
    );
}
