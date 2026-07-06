# src-tauri/src/export/gpu/pool.rs

A tiny recycled-buffer pool over an mpsc channel, so the export pipeline reuses frame buffers between decode → composite → encode instead of allocating ~tens of MB per frame.

## BufPool

```rust
pub struct BufPool
```

A pool of pre-allocated `Vec<u8>` buffers recycled between pipeline stages. On `take()`, returns a free buffer if available; allocates a fresh one otherwise (intentionally non-blocking). On `put()`, returns a buffer to the pool for reuse.

## BufPool::new

```rust
pub fn new(count: usize, bytes: usize) -> Self
```

Pre-seeds the pool with `count` zeroed buffers of `bytes` each.

### Inputs

- `count: usize` - number of buffers to pre-allocate. *Why:* reduces alloc pressure in steady state; the pipeline's bounded channel provides backpressure to prevent runaway growth.*
- `bytes: usize` - size of each buffer. *Why:* all buffers in the pool are the same size (e.g., frame size in bytes); mismatch is caller's responsibility.*

### Returns

`Self` - a new pool ready to `take()` and `put()`.

### Implementation

1. Create an unbounded `channel()` (receiver `free`, sender `ret`).
2. Send `count` fresh `vec![0u8; bytes]` buffers to `ret`.
3. Return `Self { free, ret, bytes }`.

## BufPool::take

```rust
pub fn take(&self) -> Vec<u8>
```

Retrieves a free buffer from the pool. If none is available, allocates a fresh one rather than blocking. Intentionally does NOT deadlock under exhaustion; steady-state recycling comes from the bounded pipeline channel's backpressure, not from the pool's internal blocking.

### Returns

`Vec<u8>` - a zeroed buffer of `self.bytes` length. Either recycled from the pool or freshly allocated.

### Implementation

1. Attempt `self.free.try_recv()`.
2. If `Ok(b)`, return `b`.
3. If `Err` (pool empty or channel closed), return `vec![0u8; self.bytes]`.

## BufPool::put

```rust
pub fn put(&self, buf: Vec<u8>)
```

Returns a buffer to the pool for reuse. If the pool is dropped, the buffer is silently discarded.

### Inputs

- `buf: Vec<u8>` - the buffer to return. *Why:* owned (consumed) to prevent use-after-free; the pool takes ownership.*

### Implementation

1. `self.ret.send(buf)` and silently ignore any error (pool closed).

## BufPool::returner

```rust
pub fn returner(&self) -> Sender<Vec<u8>>
```

Returns a clonable handle to the pool's receiver endpoint. Used to give the encoder thread (or any other producer) a way to return buffers without holding a reference to the entire pool.

### Returns

`Sender<Vec<u8>>` - a cloned sender. Sending to this sender deposits buffers into the same pool's receiver channel.

### Implementation

1. Clone `self.ret` and return it.
