bacnet-stack-rs
==================================

> This library is a Rust wrapper for the [BACnet Stack](https://github.com/bacnet-stack/bacnet-stack) project, built with [`bindgen`](https://github.com/rust-lang/rust-bindgen), [`cmake`](https://cmake.org/), and love 💚.

The folder `bacnet-sys/bacnet-stack` contains the submodule of the original C library.

Also, this repository is kind of a fork of [this other one](https://github.com/omnioiot/bacnet-stack-rs), updated to the latest version of Rust and bacnet-stack.

# Updating the stack

To update the stack, you need to update the submodule `bacnet-stack` to the latest commit, always check if cargo can still build the project, with `cargo build`, and then run `cargo test` to check if the tests are still passing.
