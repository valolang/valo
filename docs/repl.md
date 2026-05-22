# Valo REPL

The Valo Read-Eval-Print Loop (REPL) is an interactive environment for experimenting with Valo code.

## Starting the REPL

You can start the REPL using the CLI:

```sh
valo repl
```

## Features

- **Interactive Evaluation**: Test expressions and statements immediately.
- **State Persistence**: Variables declared in one line are available in the next.
- **Builtins**: Access `Console.WriteLine` and other builtins easily.

## Limitations

- The REPL is currently experimental.
- Complex declaration workflows (like multi-line classes or interfaces) are not fully supported interactively.
- We recommend using standard source files (`.valo`, `.bas`, `.cls`) for anything beyond simple experimentation.

## Examples

```txt
valo> Dim x As Integer
valo> x = 42
valo> Console.WriteLine(x)
42
valo> Console.WriteLine("Hello " & "World")
Hello World
valo> exit
```
