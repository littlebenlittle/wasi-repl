use std::collections::BTreeMap;

use anyhow::Context;

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
        Some(cmd) => match cmd {
            "exit" => Input::Exit,
            cmd => Input::Command(cmd.to_string(), words.map(|a| a.to_string()).collect()),
        },
        None => Input::Empty,
    }
}

fn main() -> anyhow::Result<()> {
    let mut rl = rustyline::DefaultEditor::new()?;
    let mut cache = BTreeMap::<String, wasmtime::component::Component>::new();
    let engine = wasmtime::Engine::default();
    let mut loader_store = wasmtime::Store::new(&engine, LoaderHost::new("../build")?);
    let mut loader_linker = wasmtime::component::Linker::new(&engine);
    wasmtime_wasi::command::sync::add_to_linker(&mut loader_linker)?;
    let loader_component = wasmtime::component::Component::from_file(
        &engine,
        "../target/wasm32-wasi/debug/fs_loader.wasm",
    )?;
    let mut command_store = wasmtime::Store::new(&engine, CommandHost::new()?);
    let mut command_linker = wasmtime::component::Linker::new(&engine);
    wasmtime_wasi::command::sync::add_to_linker(&mut command_linker)?;
    loop {
        match rl.readline("> ") {
            Err(rustyline::error::ReadlineError::Eof) => return Ok(()),
            Err(rustyline::error::ReadlineError::Interrupted) => return Ok(()),
            Err(e) => anyhow::bail!(e),
            Ok(line) => match parse(&line) {
                Input::Exit => return Ok(()),
                Input::Empty => {}
                Input::Command(cmd, args) => {
                    if let Some(component) = cache.get(&cmd) {
                        println!(
                            "{}",
                            match command_linker
                                .instantiate(&mut command_store, &component)?
                                .get_typed_func::<(Vec<String>,), (String,)>(&mut command_store, "eval")
                                .expect("no export func `eval`")
                                .call(&mut command_store, (args,))
                            {
                                Err(e) => e.to_string(),
                                Ok((msg,)) => msg,
                            }
                        );
                        continue;
                    }
                    println!(
                        "{}",
                        match loader_linker
                            .instantiate(&mut loader_store, &loader_component)?
                            .get_typed_func::<(String,), (Result<Vec<u8>, String>,)>(
                                &mut loader_store,
                                "load",
                            )
                            .expect("no export `load`")
                            .call(&mut loader_store, (cmd.clone(),))
                        {
                            Err(e) => e.to_string(),
                            Ok((Err(msg),)) => msg,
                            Ok((Ok(bytecode),)) => {
                                match wasmtime::component::Component::from_binary(
                                    &engine, &bytecode,
                                ) {
                                    Err(e) => e.to_string(),
                                    Ok(component) => {
                                        cache.insert(cmd, component.clone());
                                        match command_linker
                                            .instantiate(&mut command_store, &component)
                                        {
                                            Err(e) => e.to_string(),
                                            Ok(inst) => match inst
                                                .get_typed_func::<(Vec<String>,), (String,)>(
                                                    &mut command_store,
                                                    "eval",
                                                ) {
                                                Err(e) => e.to_string(),
                                                Ok(func) => {
                                                    match func.call(&mut command_store, (args,)) {
                                                        Err(e) => e.to_string(),
                                                        Ok((msg,)) => msg,
                                                    }
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    )
                }
            },
        }
    }
}
