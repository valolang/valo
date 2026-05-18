import re

with open('core/src/runtime/diagnostic.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# remove for_message block
content = re.sub(r'    fn for_message\(message: &str\) -> Self \{.*?\n    \}\n', '', content, flags=re.DOTALL)

with open('core/src/runtime/diagnostic.rs', 'w', encoding='utf-8') as f:
    f.write(content)
