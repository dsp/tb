# tb-rs Style Guide

This document outlines the coding style and conventions for the native Rust TigerBeetle client.
The style follows TigerBeetle's [TIGER_STYLE.md](../docs/TIGER_STYLE.md), adapted for Rust.

## Rust
- Use rust edition 2021
- Format with standard rustfmt

## Design Goals

Safety, performance, and developer experience. In that order.

This is a TigerBeetle client. TigerBeetle is financial infrastructure that must be correct. The
client inherits this responsibility. A bug in the client can cause data corruption, lost
transactions, or incorrect balances.

## Fixed-Size Types

**Use explicitly-sized types like `u32` for everything. Avoid architecture-specific `usize`.**

The wire protocol uses fixed-size types. The client must match exactly. Using `usize` introduces
platform-dependent behavior (32-bit vs 64-bit) that can cause subtle bugs.

```rust
// Good: Protocol constants use fixed-size types
pub const HEADER_SIZE: u32 = 256;
pub const MESSAGE_SIZE_MAX: u32 = 1024 * 1024;

// Good: Function returns fixed-size type
pub fn encode(buffer: &mut [u8], events: &[u8], element_size: u32) -> u32

// Good: Replica index uses u8 (matches protocol)
async fn send_to_replica(&mut self, replica: u8, msg: &Message) -> Result<()>
```

**Only cast to `usize` at Rust standard library boundaries:**

```rust
// Acceptable: usize required by Vec::with_capacity
let mut buffer = vec![0u8; total_size as usize];

// Acceptable: usize required for array indexing
let idx = replica as usize;
self.connections[idx] = Some(conn);

// Acceptable: Array type parameters require usize
const HEADER_SIZE_USIZE: usize = HEADER_SIZE as usize;
pub fn as_bytes(&self) -> &[u8; HEADER_SIZE_USIZE]
```

## Testing

**Every module must have tests. Every function should be tested.**

Tests are not optional. They are how we know the code works. They are documentation that executes.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Test the happy path
    }

    #[test]
    fn test_function_name_edge_case() {
        // Test edge cases
    }

    #[test]
    fn test_function_name_invalid_input() {
        // Test error handling
    }
}
```

**Test both the positive and negative space:**

- Test valid inputs produce correct outputs
- Test invalid inputs produce correct errors
- Test boundary conditions (empty, one, many, max)
- Test the transitions between valid and invalid

**Compile-time assertions for struct sizes:**

```rust
const _: () = assert!(std::mem::size_of::<Header>() == HEADER_SIZE as usize);
const _: () = assert!(std::mem::size_of::<Account>() == 128);
```

## Memory Allocation

**Prefer pre-allocation. Avoid allocation in hot paths.**

TigerBeetle pre-allocates all memory at startup. While Rust clients cannot be as strict (we use
`Vec` for dynamic responses), we should minimize allocations:

```rust
// Good: Pre-allocate with known capacity
let mut buffer = Vec::with_capacity(MESSAGE_SIZE_MAX as usize);

// Good: Reuse buffers where possible
buffer.clear();
buffer.extend_from_slice(data);

// Avoid: Growing vectors in loops
for item in items {
    result.push(process(item)); // May reallocate each iteration
}

// Better: Pre-allocate
let mut result = Vec::with_capacity(items.len());
for item in items {
    result.push(process(item));
}
```

## Error Handling

**All errors must be handled. Use `Result` for fallible operations.**

```rust
// Good: Return Result for operations that can fail
pub async fn create_accounts(&mut self, accounts: &[Account]) -> Result<Vec<CreateAccountsResult>>

// Good: Propagate errors with ?
let reply = self.send_request(msg).await?;

// Good: Handle specific error cases
match result {
    Ok(value) => process(value),
    Err(ClientError::Timeout) => retry(),
    Err(ClientError::Evicted(reason)) => handle_eviction(reason),
    Err(e) => return Err(e),
}
```

## Assertions

**Assert preconditions, postconditions, and invariants.**

```rust
// Good: Assert function preconditions
pub fn encode(buffer: &mut [u8], events: &[u8], element_size: u32) -> u32 {
    assert!((buffer.len() as u32) >= total_size);
    // ...
}

// Good: Assert compile-time invariants
const _: () = assert!(std::mem::size_of::<Header>() == 256);

// Good: Debug assertions for expensive checks
debug_assert!(header.valid_checksum());
```

## Naming

**Use snake_case. Be descriptive. Include units.**

```rust
// Good: Descriptive names with units
let timeout_ms: u64 = 500;
let body_size: u32 = header.size - HEADER_SIZE;
let trailer_size: u32 = trailer_total_size(element_size, batch_count);

// Good: Clear verb-noun naming for functions
fn calculate_checksum(data: &[u8]) -> u128
fn send_to_replica(replica: u8, msg: &Message) -> Result<()>
fn parse_results<R: Copy>(data: &[u8]) -> Vec<R>
```

## Comments

**Comments explain why, not what. Code shows what.**

```rust
// Good: Explains why
// Multi-batch encoding wraps the request body with a trailer containing
// batch count and element counts. All TigerBeetle state machine operations
// use this format.
pub fn is_multi_batch(self) -> bool

// Good: Documents safety invariants
/// # Safety
/// The slice must be exactly 256 bytes.
pub fn from_bytes(bytes: &[u8; HEADER_SIZE_USIZE]) -> &Header

// Avoid: Restating what the code does
// Increment counter by one
counter += 1;
```

## Unsafe Code

**Minimize unsafe. Document safety invariants. Prefer safe abstractions.**

```rust
// Good: Unsafe contained with documented invariants
/// Get the header as a byte slice.
///
/// # Safety
/// This is safe because Header is #[repr(C)] and HEADER_SIZE matches
/// the struct size (verified by compile-time assertion).
pub fn as_bytes(&self) -> &[u8; HEADER_SIZE_USIZE] {
    unsafe { &*(self as *const Header as *const [u8; HEADER_SIZE_USIZE]) }
}

// Good: Compile-time verification of unsafe assumptions
const _: () = assert!(std::mem::size_of::<Header>() == HEADER_SIZE as usize);
```

## Dependencies

**Minimize dependencies. Each dependency is a liability.**

Current dependencies:
- `aegis` - Checksum algorithm (required, no pure Rust alternative)
- `bitflags` - Flag types (small, stable, widely used)
- `futures-core` - Async traits (minimal, no runtime dependency)
- `rand` - Random number generation (client ID, hedging)

Do not add dependencies without careful consideration. Ask:
- Is there a simpler way without the dependency?
- Is the dependency well-maintained?
- Does it have its own dependency tree?
- What is the security surface?

## Code Organization

**Keep functions short. Keep modules focused.**

- Functions should fit on a screen (~70 lines max)
- Each module has a single responsibility
- Public API in `lib.rs`, implementation details in submodules

```
src/
├── lib.rs              # Public API, re-exports
├── error.rs            # Error types
├── transport.rs        # Async transport traits
├── client.rs           # Client state machine
└── protocol/
    ├── mod.rs          # Protocol re-exports
    ├── header.rs       # Message header
    ├── types.rs        # Account, Transfer, etc.
    ├── operation.rs    # Command/Operation enums
    ├── checksum.rs     # Aegis128L checksum
    ├── message.rs      # Message serialization
    └── multi_batch.rs  # Multi-batch encoding
```

## Integration Tests

**Integration tests verify end-to-end behavior against a real TigerBeetle server.**

```rust
// Run with: TB_ADDR=127.0.0.1:3000 cargo test --test integration_test

#[tokio::test]
async fn test_create_and_lookup_accounts() {
    let Some(mut client) = create_client().await else {
        eprintln!("Skipping test: TB_ADDR not set");
        return;
    };

    // Test actual protocol interaction
    let account = Account { ... };
    let results = client.create_accounts(&[account]).await.unwrap();
    assert!(results.is_empty(), "Expected success");
}
```

## The Bottom Line

Write code as if the next person to maintain it is a mass murderer who knows where you live.

Actually, write code as if you will maintain it in six months, having forgotten everything about
it. Be kind to your future self.
