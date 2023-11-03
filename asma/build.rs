use std::path::PathBuf;

use anyhow::Result;
use slint_build::CompilerConfiguration;
use vergen::EmitBuilder;

fn main() -> Result<()> {
    slint_build::print_rustc_flags().unwrap();

    let build_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("{build_root}");
    slint_build::compile_with_config(
        "ui/appwindow.slint",
        CompilerConfiguration::new().with_include_paths(vec![[
            format!("{build_root}/res"),
            format!("{build_root}/ui"),
        ]
        .iter()
        .map(PathBuf::from)
        .collect()]),
    )
    .unwrap();
    EmitBuilder::builder().all_build().emit()
}
