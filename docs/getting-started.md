# Getting Started with Valo

Valo is an experimental native systems language with VB.NET-inspired syntax. The current Rust toolchain interprets `.valo` sources.

## Install or Build

If a release is available for your platform, install it from the project releases or with the install script documented in the main README.

To build from source, install Rust with [rustup](https://rustup.rs/), then run:

```sh
git clone https://github.com/valolang/valo
cd valo
cargo build --release
```

The CLI binary is built at `target/release/valo`.

## First Program

Create `hello.valo`:

```vb
Sub Main()
    Console.WriteLine("Hello, Valo")
End Sub
```

Run it:

```sh
./target/release/valo run hello.valo
```

Or, while developing from the repository:

```sh
cargo run -p valo_cli -- run examples/hello.valo
```

## Check Without Running

```sh
./target/release/valo check hello.valo
```

`check` parses, loads imports, and validates the project without executing `Sub Main`.

## Interactive REPL

Use the REPL for quick experimentation:

```sh
./target/release/valo repl
```

Example session:

```txt
> Dim x As Integer
> x = 10
> Console.WriteLine(x + 5)
15
> exit
```

## Run the Examples

```sh
./target/release/valo run examples/hello.valo
./target/release/valo run examples/modules/main.valo
```

Native FFI examples require the corresponding platform library; core COM examples have been removed.

## Development Checks

Before submitting changes, run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test
cargo test -p valo_core --test examples -- --nocapture
```
