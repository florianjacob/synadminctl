# synadminctl
Work in progress synapse admin API command line interface that doubles as playground for a new ruma-client API.

[lib.rs](src/lib.rs) contains the API ideas for [ruma-client](https://github.com/ruma/ruma-client/), adhering to the [sans-io](https://sans-io.readthedocs.io/) principles of being agnostic from any form of I/O.
Protocol interactions are modeled as finite state machines, which are implemented as in the [Type-level Programming in Rust
](https://willcrichton.net/notes/type-level-programming/) blog post by Will Crichton.
[main.rs](src/main.rs) shows how the library is used, by providing the synapse admin API command line interface.
[endpoints.rs](src/endpoints.rs) contains endpoint definitions for the synapse admin API, which would eventually be replaced by definitions using [ruma-api](https://crates.io/crates/ruma-api).
