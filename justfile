default: run

test *ARGS:
    cargo build
    cargo test -- {{ARGS}}

watch *ARGS:
    watchexec -c -w src -- just test {{ARGS}}

run *ARGS:
    cargo run -- {{ARGS}}

dry-publish:
    cargo publish --dry-run

build:
    cargo build
