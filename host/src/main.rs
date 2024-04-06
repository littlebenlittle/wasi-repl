fn main() -> wasmtime::Result<()> {
    // Instantiate the engine and store
    let engine = wasmtime::Engine::default();
    let mut store = wasmtime::Store::new(&engine, ());

    // Load the component from disk
    let bytes = std::fs::read("../target/wasm32-wasi/debug/hello_world.wasm")?;
    let component = wasmtime::component::Component::new(&engine, bytes)?;

    // Configure the linker
    let linker = wasmtime::component::Linker::new(&engine);

    // Instantiate the component
    let instance = linker.instantiate(&mut store, &component)?;

    // Call the `greet` function
    let func = instance.get_func(&mut store, "greet").expect("greet export not found");
    let mut result = [wasmtime::component::Val::String("".into())];
    func.call(&mut store, &[], &mut result)?;

    // This should print out `Greeting: [String("Hello, Alice!")]`
    println!("Greeting: {:?}", result);

    Ok(())
}
