# How to Build Ark Server Manager: Ascended

ASM:A uses the Rust toolchain. The repository is also configured for those using Visual Studio Code.

To get up and running, you need the following:

* `rustup` - Install from [rustlang.org](https://www.rust-lang.org/tools/install)
* `rustc` - Version 1.73 at least.  Instructions at [rustlang.org](https://blog.rust-lang.org/2023/10/05/Rust-1.73.0.html)
* Microsoft Build Tools for C++ - [visualstudio.microsoft.com](https://aka.ms/vs/17/release/vs_BuildTools.exe)
*  (optional) Visual Studio Code - [code.visualstudio.com](https://code.visualstudio.com/)

If you are using VSCode, you will also want several extensions:
* `rust-analyzer`
* `Even Better TOML`
* `CodeLLDB`

## Compiling on Windows

If you are compling on Windows for Windows (no cross compiling) then there is no further setup necessary.

## Compiling on Linux

If you are compiling on Linux for Linux (no cross compiling) then you need to install the following
dependencies:

* `libgtk-3-dev` - See [www.gtk.org](https://www.gtk.org/docs/installations/linux/)

## Cross Compiling Linux to Windows

If you are going to build on Linux, there are a number of additional steps you must take. While for many projects simply compiling
against the `x86_64-pc-windows-gnu` target might work, for this one you need to actually compile against the MSVC target due to the
functions some of the dependent crates are linking to - notably `conpty` which makes use of a number of OS-specific functions.

### Toolchain setup

You will need the following packages.  I am assuming an Ubuntu-compatible system here, so your package names may vary:

* `llvm` - Needed for the `llvm-lib` linker. Run `sudo apt install llvm`
* `clang` - Needed for the `clang-cc` compiler.  Run `sudo apt install clang`

In addition you will need the following `cargo` tooling:
* `cargo-xwin` - This cargo tool makes managing the Windows cross-compiling install easy.  Run `cargo install cargo-xwin`.

Finally, you will need the correct rust target installed:
* `x86_64-pc-windows-msvc` - This is the MSVC cross-compiler.  Run `rustup target add x86_64-pc-windows-msvc`

### Compiling

To cross-compile for Windows, you can now do the following (for example):

`cargo xwin build -F conpty --release -p asma --target x86_64-pc-windows-msvc`

## Running

If you are _not_ cross-compiling, you can then build and run on the command line:

```
cargo run
```

Or, open the root of the repository in VSCode and (after everything gets itself set up the first time), choose `Run`->`Run Without Debugging`.

If you _are_ cross-compiling, you will need to copy the binary from `target/x86_64-pc-windows-msvc` to the target machine, or execute it in
the VM of your choosing. 