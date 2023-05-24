use std::{path::PathBuf, fs};

use anyhow::Result;
use spirv_builder::{MetadataPrintout, SpirvBuilder, ModuleResult};

fn transform_path(old_path: &PathBuf) -> Option<PathBuf> {
    Some(["in", "spirv", old_path.file_name()?.to_str()?].iter().collect::<PathBuf>())
}

fn build_shaders() -> Result<()> {
    let compile_result = SpirvBuilder::new("shaders", "spirv-unknown-vulkan1.2")
        .print_metadata(MetadataPrintout::None)
        .build()?;

    let modules = match compile_result.module {
        ModuleResult::SingleModule(path) => vec![path],
        ModuleResult::MultiModule(map) => map.into_iter().map(|(_, path)| path).collect::<Vec<_>>()
    };

    for module in modules {
        let new_path = transform_path(&module).ok_or(anyhow::anyhow!("Path could not be transformed."))?;
        fs::copy(module, new_path)?;
    }

    Ok(())
}

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=shaders/src/lib.rs");
    fs::remove_dir_all("in/spirv")?;
    fs::create_dir("in/spirv")?;
    let build = true;
    if build {
        build_shaders()?;
    }
    Ok(())
}