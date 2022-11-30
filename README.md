
## Architecture Decision Record

Hello 👋 If you are interested in how this project was conceived, we invite you to take look to the [ADR document](ADR.md)

# p2p-handshake
A CLI tool for making handshakes to p2p nodes

## How to build

## How to use this tool
## Contributing

### Checking the code

The code should be properly formatted and linted. This project makes use of [clippy](https://github.com/rust-lang/rust-clippy) and [cargo fmt](https://github.com/rust-lang/rustfmt) for that.
In order to enforce our development workflows, one could just configure the following [cargo make](https://github.com/sagiegurari/cargo-make) check as a [pre-commit hook](https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks):

```bash
cargo make --makefile workflow.toml code-check
```

### How to run the tests

A node/s from the [list of nodes](https://bitnodes.io/) should be elected. After that, just run:

```bash
TEST_NODES="<ipaddr:port> <ipaddr:port> ..." cargo test
```