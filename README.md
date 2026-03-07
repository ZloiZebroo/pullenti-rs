# Pullentipy

Pullenti Rust version: 4.33 06.02.2026

\+ Python bindings

## Build

### Rust
`cargo build --release`

### Python

#### Current OS

`cd /bindings/python`

`maturin build --release`

`pip install ./target/wheels/*.whl`

#### x86-64 linux-gnu

`docker run --rm -v "$(pwd)":/io ghcr.io/pyo3/`

`pip install "maturin[zig]"`

`rustup target add x86_64-unknown-linux-gnu`

`maturin build --release --target x86_64-unknown-linux-gnu -m bindings/python/Cargo.toml -i python3.14`
> \-i \*your python version\*


#### x86-64 linux-musl

`docker run --rm -v "$(pwd)":/io ghcr.io/pyo3/`

`pip install "maturin[zig]"`

`rustup target add x86_64-unknown-linux-musl`

`maturin build --release --target x86_64-unknown-linux-musl -m bindings/python/Cargo.toml -i python3.14`
> \-i \*your python version\*

# Test
`cargo test`

# Examples

## Rust
`cargo run --example demo`

## Python
`python bindings/python/test.py`