## Architectural Decision Record 🔴

Lets record here all the relevant decisions for this project. This document is intended to change over time. Git history can be used to see its progression.

* The first implementation for the `p2p-handshake` project will be the [Bitcoin handshake](https://github.com/bitcoinbook/bitcoinbook/blob/develop/ch08.asciidoc#network_handshake). A low level, own TCP message protocol set could be implemented by using the low level [byteorder](https://github.com/BurntSushi/byteorder) crate. But in order to be more practical, lets try the [rust-bitcoin](https://github.com/rust-bitcoin/rust-bitcoin) library. It looks perfect for the use case. It already provides the network messages, serialization and deserialization capabilities out of the box.

* It will be a CLI. There are some parameters that can be tuned for both, the application execution and the handshake itself, like timeouts, protocol message fields values or even the verbosity of the log output. Being a CLI will make it more ergonomic for human interaction. We are going to use the [clap](https://docs.rs/clap/latest/clap/) crate for speeding up things and to provide a proper growth vector for the project.
  
* This tool is going to interact with the network. Thats an IO-bound task. in which certain concurrency/parallelism levels can be implemented. So in order to speedup the process and to not block resources, we will use one of the available async Rust runtimes ([async_std](https://docs.rs/async-std/latest/async_std/), [tokio](https://tokio.rs/) ...). That will also alleviate the overhead of creating native threads for processing messages.

* Its interesting to know whats happening during the execution of the handshake. Good, human/machine readable logs should be present. The [log](https://crates.io/crates/log) crate and a lightweight log implementation like [simple_logger](https://docs.rs/simple_logger/4.0.0/simple_logger/) will be used.




### References

* https://en.bitcoin.it/wiki/Protocol_documentation#Message_structure
* https://en.bitcoin.it/wiki/Protocol_documentation#version
* https://en.bitcoin.it/wiki/Protocol_documentation#verack