use crate::commands::Command;
use anyhow::{Context as _, Error, Result};
use clap::Parser;
use std::{fs, path::PathBuf};
use wasmi::{format_c_header, precompile_wasm};

/// Precompiles a WebAssembly module into portable Wasmi IR bytecode
/// and exports it as a static C array header file (.h) or binary (.vasm/.wasm)
/// suitable for direct execution from MCU Flash memory (XIP).
#[derive(Parser)]
pub struct ExportCCommand {
    /// The WebAssembly file (.wasm or .wat) to precompile.
    #[clap(value_name = "WASM_FILE")]
    wasm_file: PathBuf,

    /// Output file path.
    ///
    /// If the path ends in `.h` or `.hpp`, a C header containing a `static const uint8_t` array is written.
    /// Otherwise, the raw precompiled binary (.vasm / .wasm) is written.
    #[clap(short = 'o', long = "output", value_name = "OUTPUT_FILE")]
    output: Option<PathBuf>,

    /// The prefix/variable name for the generated C array and header guard (e.g. `MY_MODULE`).
    ///
    /// Defaults to the file stem of the input Wasm file.
    #[clap(short = 'p', long = "prefix", value_name = "PREFIX")]
    prefix: Option<String>,
}

impl Command for ExportCCommand {
    fn execute(self) -> Result<(), Error> {
        let wasm_bytes = fs::read(&self.wasm_file)
            .with_context(|| format!("failed to read input file: {}", self.wasm_file.display()))?;

        let precompiled = precompile_wasm(&wasm_bytes)
            .with_context(|| format!("failed to precompile Wasm module: {}", self.wasm_file.display()))?;

        let stem = self
            .wasm_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("wasm");
        let prefix = self.prefix.as_deref().unwrap_or(stem);

        match self.output {
            Some(out_path) => {
                let is_header = out_path
                    .extension()
                    .map_or(false, |ext| ext == "h" || ext == "hpp");
                if is_header {
                    let header_content = format_c_header(&precompiled, prefix);
                    fs::write(&out_path, header_content.as_bytes()).with_context(|| {
                        format!("failed to write C header to: {}", out_path.display())
                    })?;
                    println!(
                        "Successfully exported static C header to: {} ({} bytes of bytecode)",
                        out_path.display(),
                        precompiled.len()
                    );
                } else {
                    fs::write(&out_path, &precompiled).with_context(|| {
                        format!(
                            "failed to write precompiled binary to: {}",
                            out_path.display()
                        )
                    })?;
                    println!(
                        "Successfully exported precompiled binary to: {} ({} bytes)",
                        out_path.display(),
                        precompiled.len()
                    );
                }
            }
            None => {
                let header_content = format_c_header(&precompiled, prefix);
                print!("{header_content}");
            }
        }

        Ok(())
    }
}
