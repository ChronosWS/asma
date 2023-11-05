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

From the root of this repository, you can then build and run on the command line:

```
cargo run
```

Or, open the root of the repository in VSCode and (after everything gets itself set up the first time), choose `Run`->`Run Without Debugging`.