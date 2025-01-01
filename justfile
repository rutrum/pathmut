default: run

test *ARGS:
    cargo test -- {{ARGS}}

watch:
    #watchexec -c -w src -- just test
    watchexec -c -w src -- just run

run *ARGS:
    cargo run -- {{ARGS}}

dry-publish:
    cargo publish --dry-run

build:
    cargo build
