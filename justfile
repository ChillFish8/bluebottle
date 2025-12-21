
run-debug:
    cargo run -p bluebottle

run-release:
    cargo run -p bluebottle --release

build-release:
    cargo build -p bluebottle --release

build-dev:
    cargo build -p bluebottle

run-components-example:
    cargo run --example components

test:
    cargo nextest run --workspace

format:
    cargo +nightly fmt --all