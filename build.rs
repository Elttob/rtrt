use anyhow::Result;
use spirv_builder::{MetadataPrintout, SpirvBuilder};

fn main() -> Result<()> {
    SpirvBuilder::new("shaders", "spirv-unknown-vulkan1.2")
        .print_metadata(MetadataPrintout::Full)
        .build()?;
    Ok(())
}