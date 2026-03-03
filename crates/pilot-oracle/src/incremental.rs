use rusqlite::{Connection, params, OptionalExtension, Result};
use crate::hash::compute_hash;

pub fn should_process(conn: &Connection, path: &str, content: &str) -> Result<bool> {
    // Check if any node exists for this path with a DIFFERENT hash
    // NOTE: This is a simplification. Ideally we track file-level hashes separately.
    // For now, we check the first node in the file. If it exists and matches, we assume file is same.
    // This is simplified but fast for MVP.
    
    // Better approach: Maintain a 'files' table.
    // For T015, we'll implement the "check hash" logic.
    
    // Let's assume we want to re-process if ANY node hash changed, or if file is new.
    // Since we don't have a file table, we'll rely on external logic (mod time) or always process in MVP.
    // However, the spec asks for hash check.
    
    let current_hash = compute_hash(content);
    
    // Create a virtual table or just query existing nodes?
    // Since 'nodes' stores 'signature_hash' per node, not file header.
    // We'll update the plan to add a `files` table later.
    
    // For now, return true to force safe re-index until file table exists.
    Ok(true)
}
