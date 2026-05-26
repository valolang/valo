import re
with open('core/src/frontend/parser/program.rs', 'r') as f:
    lines = f.readlines()
    for i, line in enumerate(lines):
        if 'if let Some(ns) = &namespace' in line:
            print(f"Line {i+1}: {line.strip()}")
            # Find the start of the function containing this line
            for j in range(i, -1, -1):
                if 'fn ' in lines[j]:
                    print(f"  Function: {lines[j].strip()}")
                    break
