# IPFS REPL

This project aims to provide a REPL (read-execute-print loop) for IPFS,
similar to `sh`-family of shells.

A low-level interaction looks like:

```
> Qxabc123... "hello world!"
hello world!
```

The REPL achieves this by resolving the CID of the first word, verifying
that is it is an executable WASM component, and running that component
with the rest of the words as arguments. So if `Qxabc123...` is the CID
of an echo program, we get the output above.

Keeping track of CIDs for your favorite commands is infeasible for our
human wetware, so we borrow some ideas from Unix and introduce a `PATH`
variable that allows us to give aliases to CIDs.

```
> PATH=Qxdef456...
> echo "hello world!"
hello world!
```

Ok, so this really didn't solve our bootstrapping problem. Now we have
to upload our desired aliases to get their CID to so we can set the
`PATH`. So by default, `PATH` comes with just one pre-set alias `ipfs`,
set the CID of a bare-bones IPFS client defined in `./ipfs-client/` in
this repo.

A more complete working example:

```
> ipfs put path.env
Qxpath...
> PATH=Qxpath...
> echo "hello world!"
hello world!
```

And that right there is the MVP (minimum viable product) for the IPFS REPL
project. Looking forward, adding some modern shell features like interpolation
would save us some arthritis:

```
> PATH=$(ipfs put <(echo=Qxecho...)) echo "hello world!"
hello world!
```

Another imprtant early milestone is security. WASM components are designed
to empower untrusted computation, so we should be able to set permissions
on commands. This probably looks like setting a `PERMS` variable that maps
the CIDs of commands to permission sets for that command, but this remains
an undecided feature.
