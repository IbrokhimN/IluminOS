use wasmi::{Engine, Module, Store, Linker};
use crate::{println, print_color};
use crate::framebuffer::{GREEN, RED, YELLOW};

static DEMO_WASM: &[u8] = include_bytes!("demo.wasm");

// вызвать функцию модуля с одним аргументом i32 -> i32
fn call_i32_i32(func_name: &str, arg: i32) -> Result<i32, &'static str> {
    let engine = Engine::default();
    let module = Module::new(&engine, DEMO_WASM).map_err(|_| "parse error")?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|_| "instantiate error")?
        .start(&mut store)
        .map_err(|_| "start error")?;

    let func = instance
        .get_typed_func::<i32, i32>(&store, func_name)
        .map_err(|_| "function not found")?;

    func.call(&mut store, arg).map_err(|_| "call trap")
}

// вызвать add(a, b)
fn call_add(a: i32, b: i32) -> Result<i32, &'static str> {
    let engine = Engine::default();
    let module = Module::new(&engine, DEMO_WASM).map_err(|_| "parse error")?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|_| "instantiate error")?
        .start(&mut store)
        .map_err(|_| "start error")?;

    let func = instance
        .get_typed_func::<(i32, i32), i32>(&store, "add")
        .map_err(|_| "function not found")?;

    func.call(&mut store, (a, b)).map_err(|_| "call trap")
}

pub fn run_demo() {
    print_color!(YELLOW, "running embedded wasm module via wasmi...\n");
    println!();

    // add
    match call_add(15, 27) {
        Ok(r) => {
            print_color!(GREEN, "wasm add(15, 27) = ");
            println!("{}", r);
        }
        Err(e) => print_color!(RED, "add error: {}\n", e),
    }

    // factorial
    match call_i32_i32("factorial", 5) {
        Ok(r) => {
            print_color!(GREEN, "wasm factorial(5) = ");
            println!("{}", r);
        }
        Err(e) => print_color!(RED, "factorial error: {}\n", e),
    }

    // fibonacci
    match call_i32_i32("fib", 10) {
        Ok(r) => {
            print_color!(GREEN, "wasm fib(10) = ");
            println!("{}", r);
        }
        Err(e) => print_color!(RED, "fib error: {}\n", e),
    }

    println!();
    print_color!(GREEN, "wasm execution complete!\n");
}
