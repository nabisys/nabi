# nabi-net

Network I/O for Nabi: TCP, UDP, Unix sockets.

Part of the [Nabi async runtime](https://nabi.run).

This crate builds on `nabi-io` to provide high-level network primitives. TCP and UDP use the platform completion path (io_uring on Linux, IOCP on Windows); Unix domain sockets are available on Unix targets. Socket options are type-safe via `socket2`, while runtime I/O paths go through Nabi's completion-based operation dispatch.

## Overview

- `tcp/` — `TcpListener`, `TcpStream`, split read/write halves, connection options
- `udp/` — `UdpSocket`, multishot receive, connection options
- `unix/` — `UnixListener`, `UnixStream`, `UnixDatagram`, peer credentials
- `addr/` — socket addresses, hostname resolution, parsing

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
