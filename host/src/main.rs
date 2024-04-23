use std::collections::BTreeMap;

use anyhow::Context;
use wasmtime::component;

use std::path::Path;

struct LoaderHost {
    ctx: wasmtime_wasi::WasiCtx,
    table: wasmtime_wasi::ResourceTable,
}

impl wasmtime_wasi::WasiView for LoaderHost {
    fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
        &mut self.ctx
    }
    fn table(&mut self) -> &mut wasmtime_wasi::ResourceTable {
        &mut self.table
    }
}

impl LoaderHost {
    fn new(dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        Ok(Self {
            ctx: wasmtime_wasi::WasiCtxBuilder::new()
                .preopened_dir(
                    wasi_common::sync::Dir::from_std_file(
                        std::fs::File::open(dir.as_ref()).with_context(|| {
                            format!("failed to open {}", dir.as_ref().to_str().unwrap())
                        })?,
                    ),
                    wasmtime_wasi::DirPerms::READ,
                    wasmtime_wasi::FilePerms::READ,
                    "/",
                )
                .build(),
            table: wasmtime_wasi::ResourceTable::new(),
        })
    }
}

struct CommandHost {
    ctx: wasmtime_wasi::WasiCtx,
    table: wasmtime_wasi::ResourceTable,
}

impl wasmtime_wasi::WasiView for CommandHost {
    fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
        &mut self.ctx
    }
    fn table(&mut self) -> &mut wasmtime_wasi::ResourceTable {
        &mut self.table
    }
}

impl CommandHost {
    fn new() -> anyhow::Result<Self> {
        Ok(Self {
            ctx: wasmtime_wasi::WasiCtxBuilder::new().build(),
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
        Some(cmd) => match cmd {
            "exit" => Input::Exit,
            cmd => Input::Command(cmd.to_string(), words.map(|a| a.to_string()).collect()),
        },
        None => Input::Empty,
    }
}

fn main() -> anyhow::Result<()> {
    Evaluator::new()?.repl()
}

struct Evaluator {
    engine: wasmtime::Engine,
    command_store: wasmtime::Store<CommandHost>,
    command_linker: wasmtime::component::Linker<CommandHost>,
    loader_store: wasmtime::Store<LoaderHost>,
    loader_linker: wasmtime::component::Linker<LoaderHost>,
    loader_component: wasmtime::component::Component,
    cache: std::collections::BTreeMap<String, wasmtime::component::Component>,
}

impl Evaluator {
    fn new() -> anyhow::Result<Self> {
        let cache = BTreeMap::<String, wasmtime::component::Component>::new();
        let engine = wasmtime::Engine::default();
        let loader_store = wasmtime::Store::new(&engine, LoaderHost::new("../build")?);
        let mut loader_linker = wasmtime::component::Linker::new(&engine);
        wasmtime_wasi::command::sync::add_to_linker(&mut loader_linker)?;
        let loader_component = wasmtime::component::Component::from_file(
            &engine,
            "../target/wasm32-wasi/debug/fs_loader.wasm",
        )?;
        let command_store = wasmtime::Store::new(&engine, CommandHost::new()?);
        let mut command_linker = wasmtime::component::Linker::new(&engine);
        wasmtime_wasi::command::sync::add_to_linker(&mut command_linker)?;
        Ok(Self {
            engine,
            cache,
            command_linker,
            command_store,
            loader_linker,
            loader_store,
            loader_component,
        })
    }

    fn repl(&mut self) -> anyhow::Result<()> {
        let mut rl = rustyline::DefaultEditor::new()?;
        loop {
            match rl.readline("> ") {
                Err(rustyline::error::ReadlineError::Eof) => return Ok(()),
                Err(rustyline::error::ReadlineError::Interrupted) => return Ok(()),
                Err(e) => anyhow::bail!(e),
                Ok(line) => match parse(&line) {
                    Input::Exit => return Ok(()),
                    Input::Empty => {}
                    Input::Command(cmd, args) => {
                        println!("{}", self.eval(cmd, args)?)
                    }
                },
            }
        }
    }

    fn eval(&mut self, cmd: String, args: Vec<String>) -> anyhow::Result<String> {
        let result = match self.cache.get(&cmd) {
            Some(component) => match self
                .command_linker
                .instantiate(&mut self.command_store, &component)?
                .get_typed_func::<(Vec<String>,), (String,)>(&mut self.command_store, "eval")
                .expect("no export func `eval`")
                .call(&mut self.command_store, (args,))
            {
                Err(e) => e.to_string(),
                Ok((msg,)) => msg,
            },
            None => match self
                .loader_linker
                .instantiate(&mut self.loader_store, &self.loader_component)?
                .get_typed_func::<(String,), (Result<Vec<u8>, String>,)>(
                    &mut self.loader_store,
                    "load",
                )
                .expect("no export `load`")
                .call(&mut self.loader_store, (cmd.clone(),))
            {
                Err(e) => e.to_string(),
                Ok((Err(msg),)) => msg,
                Ok((Ok(bytecode),)) => {
                    match wasmtime::component::Component::from_binary(&self.engine, &bytecode) {
                        Err(e) => e.to_string(),
                        Ok(component) => {
                            self.cache.insert(cmd, component.clone());
                            self.exec_component(args, component)?
                        }
                    }
                }
            },
        };
        Ok(result)
    }

    fn exec_component(
        &mut self,
        args: Vec<String>,
        component: component::Component,
    ) -> anyhow::Result<String> {
        let result = match self
            .command_linker
            .instantiate(&mut self.command_store, &component)
        {
            Err(e) => e.to_string(),
            Ok(inst) => match inst
                .get_typed_func::<(Vec<String>,), (String,)>(&mut self.command_store, "eval")
            {
                Err(e) => e.to_string(),
                Ok(func) => match func.call(&mut self.command_store, (args,)) {
                    Err(e) => e.to_string(),
                    Ok((msg,)) => msg,
                },
            },
        };
        Ok(result)
    }
}
