## Compilation
tv can be compiled to different targets using cargo.

```sh
# Default linux shared binary
cargo build --release

# Build static linux binary
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl

# Add Windows target for static binary
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

## Docs

Build gif demo using [vhs](https://github.com/charmbracelet/vhs).

```sh
vhs docs/tv-demo.tape -o docs/tv-demo.gif
```