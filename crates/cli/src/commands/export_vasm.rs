use crate::commands::Command;
use anyhow::{Context as _, Error, Result};
use clap::Parser;
use std::{fs, path::PathBuf};
use wasmi::precompile_wasm;

/// Precompiles a WebAssembly module into portable Wasmi IR bytecode
/// and writes the scrambled compact bytecode as a pure binary (.vasm) file.
#[derive(Parser)]
pub struct ExportVasmCommand {
    /// The WebAssembly file (.wasm or .wat) to precompile.
    #[clap(value_name = "WASM_FILE")]
    wasm_file: PathBuf,

    /// Output file path for the .vasm binary file.
    #[clap(short = 'o', long = "output", value_name = "OUTPUT_FILE")]
    output: PathBuf,
}

impl Command for ExportVasmCommand {
    fn execute(self) -> Result<(), Error> {
        let wasm_bytes = fs::read(&self.wasm_file)
            .with_context(|| format!("failed to read input file: {}", self.wasm_file.display()))?;

        let bytes = precompile_wasm(&wasm_bytes).with_context(|| {
            format!(
                "failed to precompile Wasm module: {}",
                self.wasm_file.display()
            )
        })?;

        fs::write(&self.output, &bytes)
            .with_context(|| format!("failed to write binary to: {}", self.output.display()))?;

        println!(
            "Successfully exported vasm binary to: {} ({} bytes)",
            self.output.display(),
            bytes.len()
        );

        Ok(())
    }
}
