default: test

test:
    cargo +nightly test

watch:
    watchexec -c -w src -- just test

run:
    cargo +nightly run
