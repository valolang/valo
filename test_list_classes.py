import re
import os

# Scan the codebase to find where classes are registered
def find_classes():
    for root, _, files in os.walk('core/src'):
        for file in files:
            path = os.path.join(root, file)
            with open(path, 'r') as f:
                content = f.read()
                if 'self.classes.insert' in content:
                    print(f"Found insertion in: {path}")

find_classes()
