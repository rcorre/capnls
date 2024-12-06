# Protobuf Language Server

`capnls` is a [Language Server](https://microsoft.github.io/language-server-protocol/) for [protobuf](https://protobuf.dev/).

`capnls` was originally hosted at https://git.sr.ht/~rrc/capnls, but was moved to https://github.com/rcorre/capnls ease contribution.
The sourcehut repo is maintained as a mirror.

# Features

- Diagnostics (from `protoc`)
- Goto Definition (for fields and imports)
- Document/Workspace Symbols
- Completion (keywords, imports, types, and options)
- Find References

# Prerequisites

Ensure [`protoc`](https://github.com/protocolbuffers/protobuf#protobuf-compiler-installation) is on your `$PATH`.

# Installation

```
cargo install --git https://github.com/rcorre/capnls
```

Ensure the cargo binary path (usually `~/.cargo/bin`) is on `$PATH`.
Finally, [configure capnls in your editor](#editor-setup).

# Configuration

Create a file named ".capnls.toml" at your workspace root, and specify the proto import paths that are passed to the `-I`/`--proto_path` flag of `protoc`.
These can be absolute, or local to the workspace.
Make sure to include the "well known" types ("google/protobuf/*.proto").
This is often "/usr/include" on a unix system.

```toml
proto_paths=["some/workspace/path", "/usr/include"]
```

If this is omitted, `capnls` will make a best-effort attempt to add local include paths.
In general, prefer explicitly specifying paths.

## Logging

Set the environment variable `RUST_LOG` to one of ERROR, WARN, INFO, DEBUG, or TRACE.
See [env_logger](https://docs.rs/env_logger/latest/env_logger/#enabling-logging) for more details.

# Editor Setup

This assumes that `capnls` and `protoc` are on your `$PATH`.

## Helix

```toml
# ~/.config/helix/languages.toml

[language-server.capnls]
command = "capnls"

[[language]]
name = "protobuf"
language-servers = ['capnls']
# Unrelated to capnls, you may want to use clang-format as a formatter
formatter = { command = "clang-format" , args = ["--assume-filename=a.proto"]}
```

You can also enable `capnls` in other languages, allowing you to search for protobuf messages without having a protobuf file open:

```toml
# ~/.config/helix/languages.toml

# Search for protobuf symbols in C++ files using <space>S
[[language]]
name = "cpp"
language-servers = [ "clangd", { name = "capnls", only-features = ["workspace-symbols"] } ]
```

# Similar Projects

- [buf-language-server](https://github.com/bufbuild/buf-language-server)
- [protocol-buffers-language-server](https://github.com/micnncim/protocol-buffers-language-server)
- [protobuf-language-server](https://github.com/lasorda/protobuf-language-server)
- [pbkit](https://github.com/pbkit/pbkit)