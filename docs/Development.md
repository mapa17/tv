## Installation

The project is configured to define the build chains used for the project and setups multiple
auxiliary tools like hasky-rs to execute pre-commit hooks.

Install the debian supported dev environment with


```sh
git clone git@github.com:mapa17/tv.git
cd tv

# Install rust if you have not yet done so
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Setup project and install dependencies
sh docs/install-dev-tools.sh

# Build binary
cargo build --release
```


## Multi platform compilation
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