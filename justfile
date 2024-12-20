default: run

test:
    cargo test

watch:
    #watchexec -c -w src -- just test
    watchexec -c -w src -- just run

run *ARGS:
    cargo run -- {{ARGS}}

dry-publish:
    cargo publish --dry-run
