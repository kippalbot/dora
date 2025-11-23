import re

with open('test_sender.py', 'r') as f:
    content = f.read()

# Find all node.send_output blocks and fix them
new_content = re.sub(
    r'node\.send_output\(\s*\n\s*(["\'][^"\']+["\']),\s*\n\s*(.+?),\s*\n\s*(\{[^}]+\})\s*\n\s*\)',
    lambda m: f'node.send_output(\n        {m.group(1)},\n        bytes({m.group(2).rstrip(",")}, "utf-8"),\n        {m.group(3)}\n    )',
    content
)

with open('test_sender.py', 'w') as f:
    f.write(new_content)

print('Fixed all send_output calls')
