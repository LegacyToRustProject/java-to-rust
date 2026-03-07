# java-to-rust

**AI-powered Java EE → Rust conversion agent.**

## Why Java

- Millions of enterprise Java EE / EJB monoliths still running in production
- JVM memory overhead: GBs per application. Rust: MBs.
- Cloud migration bottleneck: Java monoliths are too expensive to run on containers
- Spring Boot modernized new projects, but legacy EE apps are stuck
- Nobody wants to rewrite them by hand. AI changes that.

## How It Works

```
Java project (source + running instance)
    ↓ 1. Parse & analyze (class hierarchy, dependencies, annotations)
    ↓ 2. AI converts each package/module to Rust
    ↓ 3. cargo check (must compile)
    ↓ 4. Run both Java & Rust with same inputs, compare outputs
    ↓ 5. Diff? → AI fixes → goto 3
    ↓ 6. Repeat until all outputs match
Verified Rust binary
```

## Key Challenges

| Java Feature | Conversion Strategy |
|---|---|
| Garbage collection | Rust ownership + RAII |
| Class inheritance | Traits + composition |
| Generics (type erasure) | Rust generics (monomorphized) |
| Annotations / reflection | Proc macros + compile-time |
| Checked exceptions | Result<T, E> |
| Synchronized blocks | Mutex, RwLock, channels |
| JPA / Hibernate | SeaORM / Diesel |
| Servlet / EJB | Axum / Actix handlers |

## Target

- Legacy Java EE / EJB applications
- Spring monoliths too large to containerize efficiently
- Android backend services seeking performance gains

## Status

**Concept.** Architecture design in progress.

## Part of [LegacyToRust Project](https://github.com/LegacyToRustProject)

## License

MIT
