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

## Identity Operators

Valo supports `Is` and `IsNot` for reference identity checks.

```vb
If obj Is Nothing Then ...
If obj IsNot Nothing Then ...
```

`IsNot` is a modern convenience equivalent to `Not (obj Is other)`.

## Logical Operators

Valo supports standard logical operators (`And`, `Or`, `Xor`, `Not`) as well as short-circuiting operators:

- `AndAlso`: The second operand is evaluated only if the first is `True`.
- `OrElse`: The second operand is evaluated only if the first is `False`.

Short-circuiting is particularly useful when checking for null objects before accessing their members:

```vb
If customer IsNot Nothing AndAlso customer.Age > 18 Then
    ' Accessing customer.Age is safe here
End If
```

## Operator Precedence

Valo follows standard Basic operator precedence, with the following additions:

1. `^` (Exponentiation)
2. Unary `-` (Negation)
3. `*`, `/`
4. `\` (Integer Division)
5. `Mod`
6. `+`, `-` (Addition/Subtraction)
7. `&` (Concatenation)
8. `<`, `>`, `<=`, `>=`, `=`, `<>`, `Is`, `IsNot`, `Like`
9. `Not`
10. `And`, `AndAlso`
11. `Or`, `OrElse`
12. `Xor`
13. `Eqv`
14. `Imp`

Non-numeric unary operands are rejected with a type mismatch diagnostic.
