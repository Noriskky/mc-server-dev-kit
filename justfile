release:
    cargo build --release

lint:
    cargo clippy

bin:
    cargo run --bin bin -- arg1

install:
    cargo install --path ./