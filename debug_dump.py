import re
with open('core/src/backend/interpreter/interpreter.rs', 'r') as f:
    content = f.read()
    # Find the dump of classes in self.classes
    # Actually I can't easily dump from python, but I can add a debug print in interpreter.rs
    pass
