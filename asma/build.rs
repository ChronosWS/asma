use anyhow::Result;
use vergen::EmitBuilder;

fn main() -> Result<()> {
    slint_build::compile("ui/appwindow.slint").unwrap();
    EmitBuilder::builder().all_build().emit()
}
