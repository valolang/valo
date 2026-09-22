# Expressions

Valo preserves Basic-family operator syntax. Numeric coercion, overflow and radix-literal rules are still being migrated; see the [audit](../architecture/native-migration.md).

Unary `+` and `-` are valid for numeric values, including `Integer`, `Long`, `LongLong`, `Single`, `Double`, `Currency`, `Decimal`, `Date`, numeric literals with type suffixes, and hexadecimal or octal integer literals after normal literal coercion.

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

Exponentiation binds tighter than unary sign, as in VB.NET:

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

Used with whole numbers rather than `Boolean` values, `And`, `Or`, and `Xor` are
bitwise. Every integral width takes part, and the result keeps the wider
operand's type, so masking a `Long` does not narrow it to `Integer`.

```vb
Dim flags As Long = &H00FF
Dim masked As Long = flags And &H000F
```

## Shift Operators

`<<` and `>>` shift a whole number left or right.

```vb
Console.WriteLine(1 << 10)     ' 1024
Console.WriteLine(1024 >> 3)   ' 128
Console.WriteLine(-16 >> 2)    ' -4
```

The result keeps the left operand's type, and the shift count is masked to that
type's width: shifting an `Integer` by 17 shifts by 1. `>>` is arithmetic, so the
sign bit is preserved, which is why `-16 >> 2` is `-4` rather than a large
positive number.

## Compound Assignment

Every binary arithmetic, concatenation, and shift operator has a compound
assignment form.

| Operator | Equivalent to |
|---|---|
| `x += y` | `x = x + y` |
| `x -= y` | `x = x - y` |
| `x *= y` | `x = x * y` |
| `x /= y` | `x = x / y` |
| `x \= y` | `x = x \ y` |
| `x ^= y` | `x = x ^ y` |
| `x &= y` | `x = x & y` |
| `x <<= y` | `x = x << y` |
| `x >>= y` | `x = x >> y` |

```vb
Dim total As Long = 10
total += 5
total *= 2

Dim message As String = "Valo"
message &= " 1.0"
```

Compound assignment works on variables, array elements, and object fields:

```vb
counts(index) += 1
customer.Balance -= amount
```

A compound assignment expands to the equivalent binary expression, so the target
appears twice in the expansion. That is only observable when an index or receiver
expression has side effects, which is worth avoiding regardless.

## Conversion and Reflection Operators

See [Types and conversions](types.md) for `CType`, `DirectCast`, `TryCast`,
`GetType`, and `NameOf`.

## Operator Precedence

Valo follows standard Basic operator precedence, with the following additions:

1. `^` (Exponentiation)
2. Unary `-` (Negation)
3. `*`, `/`
4. `\` (Integer Division)
5. `Mod`
6. `+`, `-` (Addition/Subtraction)
7. `&` (Concatenation)
8. `<<`, `>>` (Shift)
9. `<`, `>`, `<=`, `>=`, `=`, `<>`, `Is`, `IsNot`, `Like`
10. `Not`
11. `And`, `AndAlso`
12. `Or`, `OrElse`
13. `Xor`
14. `Eqv`
15. `Imp`

Shifts bind tighter than comparison and looser than concatenation, so
`1 << 1 + 1` shifts by `2`, not by `1`.

Non-numeric unary operands are rejected with a type mismatch diagnostic.

## Queries

A query filters, sorts, and projects in one expression. It starts with `From`,
names a range variable, and walks anything `For Each` can walk: an array, a
`Collection`, or a class with an iterator.

```vb
Dim names = From p In people
            Where p.Age > 30
            Order By p.Age Descending
            Select p.Name
```

The result is a `Collection`, so it can be held in a variable or walked
straight away. A query with no `Select` yields what it walked.

| Clause | Does |
|---|---|
| `Where cond` | keeps the values the condition holds for |
| `Order By key [Ascending\|Descending]` | sorts by the key; ascending unless said otherwise |
| `Select expr` | replaces each value with the expression |
| `Distinct` | drops repeats |
| `Take n` / `Skip n` | keeps the first n, or drops them |

Clauses apply in the order they are written, each to what the one before it
produced: `Take 3` after an `Order By` means the first three of the sorted
values, not of the original ones.

The range variable belongs to the query and is not in scope outside it. After a
`Select` it no longer holds what the query started from, so `Where` and
`Order By` cannot follow one. `Distinct`, `Take`, and `Skip` can, since they
only reshape the sequence.

A projection can build a tuple, which keeps several pieces together without
declaring a type for them:

```vb
For Each entry In From p In people Order By p.Age Select (Who := p.Name, Age := p.Age)
    Console.WriteLine($"{entry.Who} ({entry.Age})")
Next entry
```

## Related

- [Example: queries](../../examples/queries.valo)
