//! Utilities for AOT precompiling Wasm modules into portable Wasmi IR
//! suitable for direct execution from MCU Flash memory (XIP).

use crate::{
    Config,
    Engine,
    Error,
    Module,
    engine::{CompilationMode, TranslationError},
};
use alloc::{format, string::String, vec::Vec};

/// Encodes an unsigned 32-bit integer into LEB128 format.
fn encode_u32_leb128(mut value: u32, buf: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// Decodes an unsigned 32-bit integer from LEB128 format at `*offset`.
fn decode_u32_leb128(slice: &[u8], offset: &mut usize) -> Option<u32> {
    let mut result: u32 = 0;
    let mut shift: u32 = 0;
    loop {
        if *offset >= slice.len() {
            return None;
        }
        let byte = slice[*offset];
        *offset += 1;
        result |= ((byte & 0x7F) as u32) << shift;
        if (byte & 0x80) == 0 {
            break;
        }
        shift += 7;
        if shift >= 35 {
            return None;
        }
    }
    Some(result)
}

/// Writes a Wasm section with `id` and `data` into `out`.
fn write_section(id: u8, data: &[u8], out: &mut Vec<u8>) {
    out.push(id);
    encode_u32_leb128(data.len() as u32, out);
    out.extend_from_slice(data);
}

/// Writes a Wasm custom section with `name` and `data` into `out`.
fn write_custom_section(name: &str, data: &[u8], out: &mut Vec<u8>) {
    let mut payload = Vec::new();
    encode_u32_leb128(name.len() as u32, &mut payload);
    payload.extend_from_slice(name.as_bytes());
    payload.extend_from_slice(data);
    write_section(0, &payload, out);
}

/// Creates a minimal dummy Wasm Code section with `func_count` empty function bodies.
///
/// Each dummy body consists of 2 bytes: `0x00` (0 locals) and `0x0b` (`end`).
fn create_dummy_code_section(func_count: u32) -> Vec<u8> {
    let mut data = Vec::new();
    encode_u32_leb128(func_count, &mut data);
    for _ in 0..func_count {
        // Body size: 2 bytes
        data.push(0x02);
        // 0 local declarations
        data.push(0x00);
        // Wasm 'end' opcode
        data.push(0x0b);
    }
    data
}

/// Precompiles a Wasm binary (`.wasm` or `.wat`) into a portable Wasmi IR bundle.
///
/// The resulting binary is a valid WebAssembly file where the original code section
/// is replaced by a minimal dummy section, and the precompiled Wasmi register-machine
/// bytecode is embedded into a `"wasmi_ir"` custom section.
///
/// The output binary is completely position-independent, pointer-free, and can be
/// burned directly into MCU Flash memory as a static C array.
pub fn precompile_wasm(wasm_bytes: &[u8]) -> Result<Vec<u8>, Error> {
    #[cfg(feature = "wat")]
    let wasm_bytes = if wasm_bytes.starts_with(b"vasm") {
        wasm_bytes
    } else {
        &wat::parse_bytes(wasm_bytes)?[..]
    };

    // 1. Compile module eagerly using portable bytecode mode.
    let mut config = Config::default();
    config.set_portable_bytecode(true);
    config.compilation_mode(CompilationMode::Eager);
    let engine = Engine::new(&config);
    let module = {
        #[cfg(feature = "validate")]
        {
            Module::new(&engine, wasm_bytes)?
        }
        #[cfg(not(feature = "validate"))]
        {
            unsafe { Module::new_unchecked(&engine, wasm_bytes)? }
        }
    };

    // 2. Build the `"wasmi_ir"` payload.
    let mut ir_payload = Vec::new();
    ir_payload.extend_from_slice(b"WIR1");
    let func_count = module.module_header().engine_funcs.len() as u32;
    ir_payload.extend_from_slice(&func_count.to_le_bytes());

    for engine_func in module.module_header().engine_funcs.iter() {
        let entry = engine
            .resolve_func(engine_func)
            .expect("compiled function entry must exist in eager mode");
        let compiled = unsafe { entry.assume_compiled() };
        ir_payload.extend_from_slice(&compiled.len_local_slots().to_le_bytes());
        ir_payload.extend_from_slice(&compiled.len_stack_slots().to_le_bytes());
        let ops = compiled.ops();
        ir_payload.extend_from_slice(&(ops.len() as u32).to_le_bytes());
        ir_payload.extend_from_slice(ops);
    }

    // 3. Reconstruct binary embedding `"wasmi_ir"`, starting with `vasm` magic header.
    if wasm_bytes.len() < 8 || (&wasm_bytes[..4] != b"\0asm" && &wasm_bytes[..4] != b"vasm") {
        return Err(Error::from(TranslationError::LazyCompilationFailed));
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"vasm\x01\x00\x00\x00"); // vasm magic + version 1

    let mut offset = 8;
    let mut emitted_ir = false;

    while offset < wasm_bytes.len() {
        let section_id = wasm_bytes[offset];
        offset += 1;
        let mut leb_offset = offset;
        let section_len = decode_u32_leb128(wasm_bytes, &mut leb_offset)
            .ok_or_else(|| Error::from(TranslationError::LazyCompilationFailed))?
            as usize;
        let section_data_start = leb_offset;
        let section_data_end = section_data_start + section_len;
        if section_data_end > wasm_bytes.len() {
            return Err(Error::from(TranslationError::LazyCompilationFailed));
        }
        let section_data = &wasm_bytes[section_data_start..section_data_end];
        offset = section_data_end;

        if section_id == 10 {
            // Code section:
            // Emit custom section "wasmi_ir" before the Code section.
            write_custom_section("wasmi_ir", &ir_payload, &mut out);
            emitted_ir = true;
            // Emit minimal dummy Code section.
            let dummy_code = create_dummy_code_section(func_count);
            write_section(10, &dummy_code, &mut out);
        } else if section_id == 0 {
            // Strip debug sections, symbol name tables, and old wasmi_ir
            let mut name_offset = 0;
            if let Some(name_len) = decode_u32_leb128(section_data, &mut name_offset) {
                let name_end = name_offset + name_len as usize;
                if name_end <= section_data.len() {
                    let name = &section_data[name_offset..name_end];
                    if name == b"wasmi_ir"
                        || name.starts_with(b".debug_")
                        || name == b"name"
                        || name == b"producers"
                        || name == b"target_features"
                    {
                        continue;
                    }
                }
            }
            write_section(section_id, section_data, &mut out);
        } else {
            write_section(section_id, section_data, &mut out);
        }
    }

    if !emitted_ir {
        write_custom_section("wasmi_ir", &ir_payload, &mut out);
    }

    Ok(out)
}

/// Formats a byte array into a C header string containing a `static const uint8_t` array.
pub fn format_c_header(data: &[u8], var_name: &str) -> String {
    let mut out = String::new();
    let upper = var_name.to_ascii_uppercase();
    out.push_str("/* Auto-generated by Wasmi AOT Precompiler */\n");
    out.push_str("/* Suitable for direct execution from MCU Flash memory (XIP) */\n\n");
    out.push_str(&format!("#ifndef {upper}_BYTECODE_H\n"));
    out.push_str(&format!("#define {upper}_BYTECODE_H\n\n"));
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <stddef.h>\n\n");
    out.push_str("#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n");
    out.push_str(&format!("/* Total size: {} bytes */\n", data.len()));
    out.push_str(&format!("static const uint8_t {}_BYTECODE[] = {{\n", upper));
    for chunk in data.chunks(12) {
        out.push_str("    ");
        for b in chunk {
            out.push_str(&format!("0x{:02x}, ", b));
        }
        out.push('\n');
    }
    out.push_str("};\n\n");
    out.push_str(&format!(
        "static const size_t {}_BYTECODE_LEN = sizeof({}_BYTECODE);\n\n",
        upper, upper
    ));
    out.push_str("#ifdef __cplusplus\n}\n#endif\n\n");
    out.push_str(&format!("#endif /* {upper}_BYTECODE_H */\n"));
    out
}
