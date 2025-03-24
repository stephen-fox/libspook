# libspook

libspook loads other libraries by taking the place of a phantom library.

Refer to [Seung Kang's blog post][sk-blog-post] on phantom DLL hijacking for
more information on the subject.

[sk-blog-post]: https://shonk.sh/posts/chasing-ghosts/

## Building

Note: It is strongly advised to use [`rustup`](https://rustup.rs/)
to install the Rust compiler. Changing compiler targets may not work
correctly otherwise.

1. `rustup install stable-i686-pc-windows-msvc`
2. `cargo build`

## Using

Library gets created in:
  `PROJECT-PATH/target/i686-pc-windows-msvc/debug/libspook.dll`

Copy the library into a directory in the library search paths.
Make sure to rename it to match the name of the phantom
(missing) library.
