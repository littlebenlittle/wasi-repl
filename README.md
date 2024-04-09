# WASI REPL

This project aims to provide a REPL (read-execute-print loop) for
WASI components.

The REPL is pretty clunky right now, so read the following carefully.

## Loaders

A `loader`'s job is to resolve command names to executable WASI
bytecode. The `wit` interface for a loader is simple:

```wit
interface loader {
    load: func(cmd: string) -> result<list<u8>, string>;
}
```

Right now the REPL is **hard-coded** to use `./components/fs-loader`.
So `cd components/fs-loader && cargo component build`.

## Commands

Commands are WASI components that export the following:

```wit
interface command {
    eval: func(args: list<string>) -> string;
}
```

`fs-loader` imports a pre-opened dir from the host, which is
hard-coded in the REPL to be `../build`, relative to the `host`
directory.

For example you can 

```sh
mkdir build
cd components/echo
cargo component build
ln ../../target/wasm32-wasi/debug/echo.wasm ../../build/
```

Then `cd host && cargo run` to get a prompt:

```
> echo.wasm blah!
blah!
```
