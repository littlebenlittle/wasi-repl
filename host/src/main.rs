use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let mut executor = Executor::new("../target/wasm32-wasi/debug/fs_loader.wasm")?;
    executor.repl()
}

struct Host {
    ctx: wasmtime_wasi::WasiCtx,
    table: wasmtime_wasi::ResourceTable,
}

impl wasmtime_wasi::WasiView for Host {
    fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
        &mut self.ctx
    }
    fn table(&mut self) -> &mut wasmtime_wasi::ResourceTable {
        &mut self.table
    }
}

impl Host {
    fn new() -> anyhow::Result<Self> {
        let dir = wasi_common::sync::Dir::from_std_file(
            std::fs::File::open("../build").with_context(|| "whatever")?,
        );
        Ok(Self {
            ctx: wasmtime_wasi::WasiCtxBuilder::new()
                .preopened_dir(
                    dir,
                    wasmtime_wasi::DirPerms::READ,
                    wasmtime_wasi::FilePerms::READ,
                    "/",
                )
                .inherit_stdout()
                .build(),
            table: wasmtime_wasi::ResourceTable::new(),
        })
    }
}

enum Input {
    Command(String, Vec<String>),
    Empty,
    Exit,
}

fn parse(line: &str) -> Input {
    let mut words = line.split(" ").filter(|a| *a != "");
    match words.next() {
        Some(cmd) => {
            return Input::Command(cmd.to_string(), words.map(|a| a.to_string()).collect())
        }
        None => return Input::Empty,
    }
}

struct Executor {
    engine: wasmtime::Engine,
    linker: wasmtime::component::Linker<Host>,
    store: wasmtime::Store<Host>,
    loader: wasmtime::component::Component,
}

impl Executor {
    fn new(loader_path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let engine = wasmtime::Engine::default();
        let store = wasmtime::Store::new(&engine, Host::new()?);
        let mut linker = wasmtime::component::Linker::new(&engine);
        let loader = wasmtime::component::Component::from_file(&engine, loader_path)?;
        wasmtime_wasi::command::sync::add_to_linker(&mut linker)?;
        Ok(Self {
            engine,
            store,
            linker,
            loader,
        })
    }

    fn repl(&mut self) -> anyhow::Result<()> {
        let mut rl = rustyline::DefaultEditor::new()?;
        loop {
            match rl.readline("> ") {
                Ok(line) => match parse(&line) {
                    Input::Command(cmd, args) => println!(
                        "{}",
                        match self.execute(cmd, args) {
                            Ok(s) => s,
                            Err(e) => e.to_string(),
                        }
                    ),
                    Input::Exit => return Ok(()),
                    Input::Empty => {}
                },
                Err(rustyline::error::ReadlineError::Eof) => return Ok(()),
                Err(rustyline::error::ReadlineError::Interrupted) => return Ok(()),
                Err(e) => anyhow::bail!(e),
            }
        }
    }

    fn execute(&mut self, cmd: String, args: Vec<String>) -> anyhow::Result<String> {
        println!("loading command `{}`", cmd);
        let bytecode = self.load(&cmd)?;
        println!("building component for command `{}`", cmd);
        let component = wasmtime::component::Component::new(&self.engine, bytecode)?;
        println!("instantiating component for command `{}`", cmd);
        let instance = self.linker.instantiate(&mut self.store, &component)?;
        let result = instance
            .get_func(&mut self.store, "eval")
            .expect("no export `eval`")
            .typed::<(Vec<String>,), (String,)>(&self.store)?
            .call(&mut self.store, (args,))?
            .0;
        return Ok(result);
    }

    fn load(&mut self, cmd: &str) -> anyhow::Result<Vec<u8>> {
        println!("instantiating loader");
        let instance = self.linker.instantiate(&mut self.store, &self.loader)?;
        return instance
            .get_func(&mut self.store, "load")
            .expect("no export `load`")
            .typed::<(String,), (Result<Vec<u8>, String>,)>(&mut self.store)?
            .call(&mut self.store, (cmd.to_owned(),))?
            .0
            .or_else(|e| Result::Err(anyhow::Error::msg(e)));
    }
}

fn blah() -> wasmtime::Result<()> {
    let engine = wasmtime::Engine::default();
    let mut store = wasmtime::Store::new(&engine, Host::new()?);
    let mut linker = wasmtime::component::Linker::new(&engine);
    wasmtime_wasi::command::sync::add_to_linker(&mut linker)?;

    let component = wasmtime::component::Component::from_file(
        &engine,
        "../target/wasm32-wasi/debug/fs_loader.wasm",
    )?;

    {
        let instance = linker.instantiate(&mut store, &component)?;
        let ls = instance
            .get_func(&mut store, "ls")
            .expect("no export `ls`")
            .typed::<(), (Option<String>,)>(&store)?;
        println!("calling ls");
        let result = ls.call(&mut store, ())?.0.unwrap_or("no error".into());
        println!("error: {}", result);
    }

    {
        let instance = linker.instantiate(&mut store, &component)?;
        let load = instance
            .get_func(&mut store, "load")
            .expect("no export `load`")
            .typed::<(String,), (Result<Vec<u8>, String>,)>(&store)?;
        println!("calling load");
        let result: String;
        match load.call(&mut store, ("blah.txt".into(),))?.0 {
            Ok(v) => result = std::str::from_utf8(&v)?.to_owned(),
            Err(e) => anyhow::bail!(e),
        }
        println!("result: {}", result);
    }

    return Ok(());
}
