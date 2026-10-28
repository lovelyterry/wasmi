//! Minimal lightweight runtime library for executing .vasm and .zasm bytecode directly from Flash / memory.
//!
//! Provides both a safe Rust API and C-compatible ABI exports (`staticlib`) for direct linking
//! in MCU C/C++ firmware projects.

use std::{
    io::Write as _,
    process,
    time::{SystemTime, UNIX_EPOCH},
};
pub use wasmi::{Caller, Config, Engine, Error, Extern, Func, Linker, Module, Store, Val};

/// Registers lightweight embedded WASI functions for the runtime.
pub fn register_minimal_wasi(
    linker: &mut Linker<()>,
    store: &mut Store<()>,
) -> Result<(), wasmi::Error> {
    const WASI: &str = "wasi_snapshot_preview1";

    // 1. clock_time_get(clock_id: i32, precision: i64, time_ptr: i32) -> i32
    linker.define(
        WASI,
        "clock_time_get",
        Func::wrap(
            &mut *store,
            |mut caller: Caller<()>, _clock_id: i32, _precision: i64, time_ptr: i32| -> i32 {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64;
                let Some(Extern::Memory(mem)) = caller.get_export("memory") else {
                    return 28; // EINVAL
                };
                let offset = time_ptr as usize;
                let mem_bytes = mem.data_mut(&mut caller);
                if offset + 8 <= mem_bytes.len() {
                    mem_bytes[offset..offset + 8].copy_from_slice(&now.to_le_bytes());
                    0 // Success
                } else {
                    21 // EFAULT
                }
            },
        ),
    )?;

    // 2. fd_write(fd: i32, iovs_ptr: i32, iovs_len: i32, nwritten_ptr: i32) -> i32
    linker.define(
        WASI,
        "fd_write",
        Func::wrap(
            &mut *store,
            |mut caller: Caller<()>,
             fd: i32,
             iovs_ptr: i32,
             iovs_len: i32,
             nwritten_ptr: i32|
             -> i32 {
                let Some(Extern::Memory(mem)) = caller.get_export("memory") else {
                    return 28;
                };
                let mem_bytes = mem.data(&caller);
                let mut total_written = 0u32;
                for i in 0..iovs_len as usize {
                    let offset = (iovs_ptr as usize) + i * 8;
                    if offset + 8 > mem_bytes.len() {
                        return 21;
                    }
                    let buf_ptr = u32::from_le_bytes(mem_bytes[offset..offset + 4].try_into().unwrap())
                        as usize;
                    let buf_len = u32::from_le_bytes(mem_bytes[offset + 4..offset + 8].try_into().unwrap())
                        as usize;
                    if buf_ptr + buf_len > mem_bytes.len() {
                        return 21;
                    }
                    let slice = &mem_bytes[buf_ptr..buf_ptr + buf_len];
                    if fd == 1 || fd == 2 {
                        let _ = std::io::stdout().write_all(slice);
                    }
                    total_written += buf_len as u32;
                }
                let mem_mut = mem.data_mut(&mut caller);
                let nw_offset = nwritten_ptr as usize;
                if nw_offset + 4 <= mem_mut.len() {
                    mem_mut[nw_offset..nw_offset + 4].copy_from_slice(&total_written.to_le_bytes());
                    0
                } else {
                    21
                }
            },
        ),
    )?;

    // 3. fd_close(fd: i32) -> i32
    linker.define(
        WASI,
        "fd_close",
        Func::wrap(&mut *store, |_caller: Caller<()>, _fd: i32| -> i32 { 0 }),
    )?;

    // 4. fd_fdstat_get(fd: i32, stat_ptr: i32) -> i32
    linker.define(
        WASI,
        "fd_fdstat_get",
        Func::wrap(
            &mut *store,
            |mut caller: Caller<()>, _fd: i32, stat_ptr: i32| -> i32 {
                let Some(Extern::Memory(mem)) = caller.get_export("memory") else {
                    return 28;
                };
                let mem_bytes = mem.data_mut(&mut caller);
                let offset = stat_ptr as usize;
                if offset + 24 <= mem_bytes.len() {
                    mem_bytes[offset..offset + 24].fill(0);
                    0
                } else {
                    21
                }
            },
        ),
    )?;

    // 5. fd_seek(fd: i32, offset: i64, whence: i32, newoffset_ptr: i32) -> i32
    linker.define(
        WASI,
        "fd_seek",
        Func::wrap(
            &mut *store,
            |_caller: Caller<()>, _fd: i32, _offset: i64, _whence: i32, _new_ptr: i32| -> i32 { 0 },
        ),
    )?;

    // 6. proc_exit(exit_code: i32)
    linker.define(
        WASI,
        "proc_exit",
        Func::wrap(&mut *store, |_caller: Caller<()>, exit_code: i32| {
            if exit_code != 0 {
                process::exit(exit_code);
            }
        }),
    )?;

    // 7. environ_sizes_get(count_ptr: i32, buf_size_ptr: i32) -> i32
    linker.define(
        WASI,
        "environ_sizes_get",
        Func::wrap(
            &mut *store,
            |mut caller: Caller<()>, count_ptr: i32, size_ptr: i32| -> i32 {
                let Some(Extern::Memory(mem)) = caller.get_export("memory") else {
                    return 28;
                };
                let mem_bytes = mem.data_mut(&mut caller);
                let cp = count_ptr as usize;
                let sp = size_ptr as usize;
                if cp + 4 <= mem_bytes.len() && sp + 4 <= mem_bytes.len() {
                    mem_bytes[cp..cp + 4].copy_from_slice(&0u32.to_le_bytes());
                    mem_bytes[sp..sp + 4].copy_from_slice(&0u32.to_le_bytes());
                    0
                } else {
                    21
                }
            },
        ),
    )?;

    // 8. environ_get(environ_ptr: i32, environ_buf_ptr: i32) -> i32
    linker.define(
        WASI,
        "environ_get",
        Func::wrap(
            &mut *store,
            |_caller: Caller<()>, _env_ptr: i32, _buf_ptr: i32| -> i32 { 0 },
        ),
    )?;

    // 9. args_sizes_get(argc_ptr: i32, argv_buf_size_ptr: i32) -> i32
    linker.define(
        WASI,
        "args_sizes_get",
        Func::wrap(
            &mut *store,
            |mut caller: Caller<()>, argc_ptr: i32, buf_ptr: i32| -> i32 {
                let Some(Extern::Memory(mem)) = caller.get_export("memory") else {
                    return 28;
                };
                let mem_bytes = mem.data_mut(&mut caller);
                let ap = argc_ptr as usize;
                let bp = buf_ptr as usize;
                if ap + 4 <= mem_bytes.len() && bp + 4 <= mem_bytes.len() {
                    mem_bytes[ap..ap + 4].copy_from_slice(&0u32.to_le_bytes());
                    mem_bytes[bp..bp + 4].copy_from_slice(&0u32.to_le_bytes());
                    0
                } else {
                    21
                }
            },
        ),
    )?;

    // 10. args_get(argv_ptr: i32, argv_buf_ptr: i32) -> i32
    linker.define(
        WASI,
        "args_get",
        Func::wrap(
            &mut *store,
            |_caller: Caller<()>, _argv_ptr: i32, _buf_ptr: i32| -> i32 { 0 },
        ),
    )?;

    Ok(())
}

/// Executes a .vasm bytecode buffer from static memory (Flash/XIP) using `_start`.
pub fn run_vasm(static_bytes: &'static [u8]) -> Result<(), wasmi::Error> {
    let config = Config::default();
    let engine = Engine::new(&config);
    let module = Module::new_static(&engine, static_bytes)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    register_minimal_wasi(&mut linker, &mut store)?;
    let instance = linker.instantiate_and_start(&mut store, &module)?;
    if let Some(start) = instance.get_func(&store, "_start") {
        let mut results = [];
        start.call(&mut store, &[], &mut results)?;
    }
    Ok(())
}

/// Invokes a specific exported function in a .vasm module residing in static Flash memory.
pub fn run_vasm_invoke(
    static_bytes: &'static [u8],
    func_name: &str,
    params: &[Val],
    results: &mut [Val],
) -> Result<(), wasmi::Error> {
    let config = Config::default();
    let engine = Engine::new(&config);
    let module = Module::new_static(&engine, static_bytes)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    register_minimal_wasi(&mut linker, &mut store)?;
    let instance = linker.instantiate_and_start(&mut store, &module)?;
    let func = instance
        .get_func(&store, func_name)
        .ok_or_else(|| wasmi::Error::from(wasmi::errors::FuncError::ExportedFuncNotFound))?;
    func.call(&mut store, params, results)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// C-ABI Exports (for static library linking from MCU C/C++ firmware)
// ---------------------------------------------------------------------------

/// C-ABI: Executes `_start` directly from a static Flash byte array.
///
/// Returns 0 on success, negative value on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vasm_run_bytes(bytes: *const u8, len: usize) -> i32 {
    if bytes.is_null() || len == 0 {
        return -1;
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    let static_bytes: &'static [u8] = unsafe { std::mem::transmute(slice) };
    match run_vasm(static_bytes) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}
