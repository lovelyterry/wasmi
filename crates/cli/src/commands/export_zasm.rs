use crate::commands::Command;
use anyhow::{Context as _, Error, Result};
use clap::Parser;
use std::{fs, path::PathBuf};
use wasmi::precompile_zasm;

/// Precompiles a WebAssembly module into 4KB-chunked LZ4 compressed Wasmi IR bytecode
/// and writes the output as a compact binary (.zasm) file.
#[derive(Parser)]
pub struct ExportZasmCommand {
    /// The WebAssembly file (.wasm or .wat) to precompile.
    #[clap(value_name = "WASM_FILE")]
    wasm_file: PathBuf,

    /// Output file path for the .zasm binary file.
    #[clap(short = 'o', long = "output", value_name = "OUTPUT_FILE")]
    output: PathBuf,
}

impl Command for ExportZasmCommand {
    fn execute(self) -> Result<(), Error> {
        let wasm_bytes = fs::read(&self.wasm_file)
            .with_context(|| format!("failed to read input file: {}", self.wasm_file.display()))?;

        let bytes = precompile_zasm(&wasm_bytes).with_context(|| {
            format!(
                "failed to precompile Wasm module to zasm: {}",
                self.wasm_file.display()
            )
        })?;

        fs::write(&self.output, &bytes)
            .with_context(|| format!("failed to write binary to: {}", self.output.display()))?;

        println!(
            "Successfully exported 4KB-chunked compressed zasm binary to: {} ({} bytes)",
            self.output.display(),
            bytes.len()
        );

        Ok(())
    }
}
