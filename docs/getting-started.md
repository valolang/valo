# Getting Started with Valo

Welcome to Valo. This guide will help you get up and running with the language.

## 1. Prerequisites
Ensure you have Rust installed (via [rustup](https://rustup.rs/)).

## 2. Building Valo
```bash
git clone https://github.com/valolang/valo
cd valo
cargo build --release
```

## 3. Your First Program
Create a file named `hello.valo`:
```vb
Sub Main()
    Console.WriteLine("Hello, Valo!")
End Sub
```

Run it:
```bash
./target/release/valo run hello.valo
```

## 4. Interactive REPL
Use the REPL for quick experimentation:
```bash
./target/release/valo repl
> Console.WriteLine(10 + 5)
15
> exit
```
