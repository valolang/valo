# Expressions

Valo follows VBA-style numeric expression rules for unary operators and exponentiation.

Unary `+` and `-` are valid for numeric values, including `Integer`, `Long`, `LongLong`, `Single`, `Double`, `Currency`, `Decimal`, `Date`, numeric literals with VBA suffixes, and hexadecimal or octal integer literals after normal literal coercion.

```vb
Debug.Print -10#
Debug.Print -10!
Debug.Print -10@
Debug.Print -.5
Debug.Print -1E+3
Debug.Print -&H1
Debug.Print +.5
```

Unary operators can be applied to literals, variables, function calls, parenthesized expressions, array elements, member access, default property access, and native FFI return values. The unary operator preserves the operand's runtime numeric representation where the value can be represented by that type; assigning into a typed variable then uses the normal assignment coercion rules.

```vb
point.X = -point.X
Debug.Print -(x + y)
Debug.Print -Cos(0#)
```

Exponentiation binds tighter than unary sign, matching VBA's practical behavior:

```vb
Debug.Print -2 ^ 2    ' -4
Debug.Print (-2) ^ 2  ' 4
Debug.Print 2 ^ -2    ' 0.25
```

Non-numeric unary operands are rejected with a type mismatch diagnostic.
