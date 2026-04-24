# nabi-fs

Filesystem I/O for Nabi: files, directories, pipes, splice/tee.

Part of the [Nabi async runtime](https://nabi.run).

This crate builds on `nabi-io` to provide async filesystem primitives. File and directory operations use io_uring's full opcode surface on Linux (including `OPENAT2`, `STATX`, `FALLOCATE`, `FADVISE`), while Windows uses IOCP with `CreateFile2`. Pipe operations include `splice`/`tee` on Linux for zero-copy transfers between file descriptors.

## Overview

- `file/` — `File` with vectored I/O, registered buffers, `sync`/`fdatasync`, `fadvise`, `fallocate`
- `dir/` — `Dir` handle, async `ReadDir` via `GETDENTS`
- `path/` — `rename`, `remove`, `symlink`, `link`, path-based operations
- `pipe/` — async pipes with `splice` and `tee` (Linux zero-copy)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
