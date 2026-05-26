from valo_core import Parser, FileId
source = "Namespace Game.Graphics\nPublic Class Sprite\nEnd Class\nEnd Namespace"
program = Parser.parse_source(source, FileId())
print(f"Namespace: {program.namespace}")
