with open('core/src/frontend/parser/program.rs', 'r') as f:
    lines = f.readlines()
    for i, line in enumerate(lines[460:480]):
        print(f"{i+460}: {line.strip()}")
