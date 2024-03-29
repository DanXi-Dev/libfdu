# libfdu
A universal SDK for FDU.

## ☢ This repository has been archived ☢
Due to no time, no passion and so on, we decide to terminate any development on this repository. It is far from usable so you may want to look other similar projects instead.

## Building
You need **[Rust](https://www.rust-lang.org) Nightly** installed:

```shell
$ rustup default nightly
```

Build the library by running:

```shell
$ cargo build
```

or 

```shell
$ cargo build --release
```

You will find the library files `*.dll & *.dll.lib`, `*.dylib`, or `*.so` in the `target/debug` or `target/release` directory, and C header is `bindings.h` in the project root directory.

## Testing
Some examples are available in `src/lib.rs`.

You are able to run these tests by running:

```shell
$ cargo test
```

If more precise control on testing is needed, you can run all or some of them in your IDE. (e.g. [CLion](https://www.jetbrains.com/clion/), [Visual Studio Code](https://code.visualstudio.com/))


## Contribution
You can contribute to the project by opening an issue or creating a pull request.

To get familiar with the library, you are encouraged to read the comments in the source code directly.
