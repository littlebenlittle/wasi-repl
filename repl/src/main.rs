use wasi_common::sync::snapshots::preview_0::add_wasi_unstable_to_linker;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    /*
    let loader_path = {
        let args: Vec<String> = std::env::args().collect();
        args[1].clone()
    };
    */
    let loader_path = String::from("../target/wasm32-wasi/debug/hello_world.wasm");

    let engine = wasmtime::Engine::new(wasmtime::Config::new().wasm_component_model(true))?;
    let mut store = wasmtime::Store::new(&engine, ());

    let component = wasmtime::component::Component::from_file(&engine, loader_path)?;
    let linker = wasmtime::component::Linker::new(&engine);

    let instance = linker.instantiate(&mut store, &component)?;
    let func = instance
        .get_func(&mut store, "load")
        .expect("no export `load`")
        .typed::<(String,), (Vec<u8>,)>(&store)?;
    
    func.call(&mut store, ("blah.txt".into(),))?;
    return Ok(())

    /*
    let mut load = |s: String| -> Result<Vec<u8>> { Ok(func.call(&mut store, (s,))?.0) };

    println!("starting repl");
    loop {
        print!("> ");
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        let mut words = line.split(" ").filter(|a| *a != "");
        if let Some(cmd) = words.next() {
            let data = load(cmd.into())?;
            println!("{}", std::str::from_utf8(&data)?)
        }
    }
    */
}
