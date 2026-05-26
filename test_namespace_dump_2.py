import re
# If class_decl.name is 'A.One', then qualified_symbol_key returns "0.a.one"
# But the code uses qualified_symbol_key("0", "A.One") -> "0.a.one"
# If I try to resolve "A.One", resolve_user_type_name splits "A.One" into ["A", "One"]
# 'A' is treated as a module, 'One' as the member.
# Is "A" a module? Namespace "A" is not a module!
