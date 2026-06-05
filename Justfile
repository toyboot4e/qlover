# Just a task runner
# <https://github.com/casey/just>

# shows this help message
help:
    @just -l

# builds the library
build *args:
    cargo build {{args}}

[private]
alias b := build

# builds the documentation
doc *args:
    cargo doc {{args}} # -- open

[private]
alias d := doc

[private]
do *args:
    cargo doc --open {{args}}

# runs the documentation test
doctest *args:
    cargo test --doc {{args}}

[private]
alias dt := doctest

[private]
format *args:
    cargo fmt {{args}}

[private]
alias fmt := format

[private]
alias f := format

# runs the tests
test *args:
    cargo test {{args}}

[private]
alias t := test

# many tewts
mt *args:
    PROPTEST_CASES=10000 cargo test {{args}}

# runs the executable
run *args:
    cargo run {{args}}

[private]
alias r := run

# watches change and runs `cargo doc`. requires `cargo-watch`.
watch *args:
    cargo watch -x doc {{args}}

[private]
alias w := watch

