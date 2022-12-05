default: run

test:
    cargo +nightly test

watch:
    watchexec -c -w src -- just test

run *ARGS :
    cargo +nightly run -- {{ARGS}}

dry-publish:
    cargo +nightly publish --dry-run
