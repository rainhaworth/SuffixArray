# SuffixArray
This project is implemented in Rust. Much of the implementation was informed by the Rust documentation found at https://doc.rust-lang.org/std/index.html. Small sections dedicated to file i/o were taken directly from the docs.

A large number of StackOverflow posts additionally helped with more minute implementation details, but I unfortunately did not keep very good track of them.

## Building
```
cargo build --bin buildsa
```

```
cargo build --bin querysa
```

## Executing
```
cargo run --bin buildsa -- <reference path> <output filename> [--preftab k]
```
```
cargo run --bin querysa -- <index path> <queries path> <mode> <output filename>
```
