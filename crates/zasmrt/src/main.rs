//! zasmrt CLI executable.

use std::{env, fs, path::Path, process};
use zasmrt::{Val, run_vasm, run_vasm_invoke};

fn print_usage() {
    eprintln!("Usage: zasmrt <file.vasm | file.zasm> [--invoke <function_name> [args...]]");
    eprintln!();
    eprintln!("Ultra-minimal lightweight runtime for executing .vasm and .zasm bytecode directly from Flash / memory.");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args[1] == "-h" || args[1] == "--help" {
        print_usage();
        return;
    }

    let mut vasm_file: Option<&str> = None;
    let mut invoke_func: Option<&str> = None;
    let mut func_args: Vec<&str> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--invoke" {
            if i + 1 < args.len() {
                invoke_func = Some(&args[i + 1]);
                i += 2;
                while i < args.len() {
                    func_args.push(&args[i]);
                    i += 1;
                }
                break;
            } else {
                eprintln!("Error: missing function name after --invoke");
                process::exit(1);
            }
        } else if vasm_file.is_none() {
            vasm_file = Some(&args[i]);
            i += 1;
        } else {
            func_args.push(&args[i]);
            i += 1;
        }
    }

    let Some(path_str) = vasm_file else {
        print_usage();
        process::exit(1);
    };

    let path = Path::new(path_str);
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error: failed to read '{path_str}': {e}");
            process::exit(1);
        }
    };

    // Leak to simulate an immutable, zero-copy &'static [u8] buffer in Flash memory (XIP)
    let static_bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());

    if let Some(name) = invoke_func {
        let mut wasmi_params = Vec::new();
        for arg_str in &func_args {
            if let Ok(v) = arg_str.parse::<i32>() {
                wasmi_params.push(Val::I32(v));
            } else if let Ok(v) = arg_str.parse::<f32>() {
                wasmi_params.push(Val::F32(v.into()));
            } else if let Ok(v) = arg_str.parse::<i64>() {
                wasmi_params.push(Val::I64(v));
            } else if let Ok(v) = arg_str.parse::<f64>() {
                wasmi_params.push(Val::F64(v.into()));
            }
        }
        let mut results = vec![Val::I32(0); 1];
        if let Err(e) = run_vasm_invoke(static_bytes, name, &wasmi_params, &mut results) {
            eprintln!("Error executing '{name}': {e}");
            process::exit(1);
        }
        for (i, res) in results.iter().enumerate() {
            if i > 0 {
                print!(" ");
            }
            match res {
                Val::I32(v) => print!("{v}"),
                Val::I64(v) => print!("{v}"),
                Val::F32(v) => print!("{v}"),
                Val::F64(v) => print!("{v}"),
                _ => print!("{:?}", res),
            }
        }
        println!();
    } else {
        if let Err(e) = run_vasm(static_bytes) {
            eprintln!("Error executing vasm: {e}");
            process::exit(1);
        }
    }
}
