with open('core/src/frontend/parser/program.rs', 'r') as f:
    lines = f.readlines()
    for i, line in enumerate(lines[1, 500]):
        if 'match self.peek_kind()' in line:
            print(f"Line {i+1}: {line.strip()}")
