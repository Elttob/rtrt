use anyhow::Result;
use spirv_builder::{MetadataPrintout, SpirvBuilder};

fn build_shaders() -> Result<()> {
    SpirvBuilder::new("shaders", "spirv-unknown-vulkan1.2")
        .print_metadata(MetadataPrintout::Full)
        .build()?;

    Ok(())
}

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=shaders/src");
    let build = true;
    if build {
        build_shaders()?;
    }
    Ok(())
}