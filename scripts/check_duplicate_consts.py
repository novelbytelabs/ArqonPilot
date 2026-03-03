#!/usr/bin/env python3
import sys
import re
import os

def check_file(filepath):
    if not os.path.exists(filepath):
        print(f"Error: File not found: {filepath}")
        return False

    with open(filepath, 'r') as f:
        content = f.read()

    # Regex to find 'const name =' or 'let name =' at the start of a line (top-levelish)
    # This is a simple check targeting the "cluster" pattern in G-015.
    pattern = re.compile(r'^(?:const|let)\s+([a-zA-Z0-9_$]+)\s*=', re.MULTILINE)
    
    found = {}
    duplicates = []
    
    for line_num, line in enumerate(content.splitlines(), 1):
        match = re.search(r'^\s*(?:const|let)\s+([a-zA-Z0-9_$]+)\s*=', line)
        if match:
            var_name = match.group(1)
            # Only track top-level (no leading whitespace) or semi-top-level
            # In pilot_ui.js most are at 0 indentation
            if line.startswith('const ') or line.startswith('let '):
                if var_name in found:
                    duplicates.append((var_name, found[var_name], line_num))
                else:
                    found[var_name] = line_num

    if duplicates:
        print(f"❌ FAILED: Duplicate declarations found in {filepath}:")
        for name, first, second in duplicates:
            print(f"  - '{name}' first declared at line {first}, then again at line {second}")
        return False
    
    print(f"✅ PASSED: No top-level duplicate const/let found in {filepath} ({len(found)} declarations checked)")
    return True

def main():
    files_to_check = [
        "crates/pilot/src/pilot_ui.js",
        "crates/pilot/src/serve_ui.rs" # In case there's an inline block
    ]
    
    all_ok = True
    for f in files_to_check:
        if os.path.exists(f):
            if not check_file(f):
                all_ok = False
        else:
            # Silently skip if file doesn't exist in current repo structure
            pass
            
    if not all_ok:
        sys.exit(1)

if __name__ == "__main__":
    main()
