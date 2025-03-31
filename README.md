# libspook

libspook injects libraries into a process by taking advantage of a very
specific form of [phantom DLL hijacking][mitre-phantom-dll]. It assumes
the vulnerable process will gracefully ignore phantom libraries that
return `false` from [`DllMain`][ms-dllmain-doc]. Returning false results
in a null pointer being returned to the code that attempted to load
libspook, giving the vulnerable code a chance to ignore the load failure
while simultaneously allowing libspook to execute its code.

This project currently only supports Windows, though support for
Unix-like systems can be added in the future.

Refer to [Seung Kang's blog post][sk-post] for a real-world example
of phantom DLL hijacking.

[mitre-phantom-dll]: https://attack.mitre.org/techniques/T1574/001/
[ms-dllmain-doc]: https://learn.microsoft.com/en-us/windows/win32/dlls/dllmain
[sk-post]: https://shonk.sh/posts/chasing-ghosts/

## Features

- Libraries are injected based on a configuration file and the name
  of the current process' executable. This allows a single phantom
  library to be reused across multiple programs
- Uses a simple `ini` configuration file (with comment support!)
- No external dependencies

## Building

Note: It is strongly recommended to use [`rustup`](https://rustup.rs/)
to install the Rust compiler. Changing compiler targets may not work
correctly otherwise.

1. `rustup install stable-i686-pc-windows-msvc`
2. `cargo build`

Library gets created in:
  `PROJECT-PATH/target/i686-pc-windows-msvc/debug/libspook.dll`

In addition to the `debug` configuration setting (discussed below),
additional debug information can be obtained at runtime by building
libspook with the arguments: `--features debug`.

## Usage

1. Find an application that attempts to load a non-existent, "phantom"
   library. Refer to [HackTricks blog post][hacktricks-post]
   and [Seung's blog post][sk-post] for examples
2. Compile libspook
3. Copy libspook into a directory in the library search paths.
   On Windows, this can be a directory named by the `PATH` variable.
   Make sure to rename the library to match the name of the phantom
   (missing) library
4. Create a configuration file in `~/.libspook/libspook.conf`
   (on Windows, `~` means your user account's directory)
5. Create an entry in the configuration file according according
   to the [Configuration](#configuration) section
6. Start the program that attempts to load the phantom library

[hacktricks-post]: https://book.hacktricks.wiki/en/windows-hardening/windows-local-privilege-escalation/dll-hijacking/index.html

## Configuration

libspook reads configuration from `~/.libspook/libspook.conf` when it
is loaded. The file uses a simple `ini` syntax consisting of sections
named after the executable that loaded it and an optional `general`
section. Comments can be specified using `#`.

For example, the following configuration file loads `example.dll`
when libspook is loaded by `foo.exe`:

```ini
# This is an example comment :)
[foo.exe]
load = C:\Users\user\projects\example\example.dll
```

#### `general` section

This section configures general libspook settings. The following parameters
can be specified in this section:

- `debug` (`bool`, default: `false`) - When set to `true`, display message
  boxes containing information about the process and libspook

#### process section

Sections named after an executable configure what libspook does
when said executable loads libspook. For example, a section named
`[example.exe]` specifies the behavior for the `example.exe` process.

The following parameters can be specified in this section:

- `load` (`string`, no default) - Load the specified library into
  the current process
- `allow_init_failure` (`bool`, default: `false`) - Allows the
  previously-specified library to return false when loaded.
  Only affects the previously-defined library
