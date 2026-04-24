# nabi-tls

TLS for Nabi: rustls and native-tls backends.

Part of the [Nabi async runtime](https://nabi.run).

This crate provides TLS support layered on top of `nabi-net`. Two backends are available via Cargo features: `rustls` (default, pure-Rust with the `ring` crypto provider) and `native-tls` (platform-native — OpenSSL, SecureTransport, SChannel). Backends expose full native APIs through their own modules; `Stream` and `Handshake` types are shared across backends for transport-level integration with Nabi's completion-based I/O.

## Features

- `rustls` (default) — pure-Rust TLS via [rustls](https://github.com/rustls/rustls) with `ring` crypto
- `native-tls` — platform-native TLS (OpenSSL, SecureTransport, SChannel)

Both features can be enabled simultaneously; backends coexist in separate modules.

## Overview

- `connector` / `acceptor` / `stream` — backend-agnostic integration types
- `config/` — server and client TLS configuration, cipher selection
- `handshake/` — handshake state machine, error types
- `rustls/` — rustls-specific connector, acceptor, and configuration
- `native/` — native-tls-specific connector, acceptor, and configuration

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
