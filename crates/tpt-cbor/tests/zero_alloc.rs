//! Verifies that decoding borrowed data performs no heap allocations.
//!
//! This file contains exactly one test, because the counter is process-wide.
//! Parallel tests would otherwise add their own allocations to the count.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use tpt_cbor::{Config, Decoder};

struct Counting;

static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: forwarded unchanged to the system allocator.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: forwarded unchanged to the system allocator.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: forwarded unchanged to the system allocator.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

#[test]
fn borrowed_decoding_allocates_nothing() {
    // A document with a text string, a byte string, nested arrays and a map.
    // Only the borrowing and skipping paths are used, so no allocation is expected.
    let doc: &[u8] = &[
        0xa2, // map(2)
        0x61, 0x61, // "a"
        0x64, b'w', b'a', b'v', b'e', // "wave"
        0x61, 0x62, // "b"
        0x82, 0x44, 0x01, 0x02, 0x03, 0x04, 0x05, // [h'01020304', 5]
    ];

    let before = ALLOCS.load(Ordering::SeqCst);

    let mut d = Decoder::with_config(doc, Config::strict());
    assert_eq!(d.decode_map_len(), Ok(2));
    assert_eq!(d.decode_str(), Ok("a"));
    assert_eq!(d.decode_str(), Ok("wave"));
    assert_eq!(d.decode_str(), Ok("b"));
    assert_eq!(d.decode_array_len(), Ok(2));
    assert_eq!(d.decode_bytes(), Ok(&[1u8, 2, 3, 4][..]));
    assert_eq!(d.decode_u64(), Ok(5));
    assert_eq!(d.finish(), Ok(()));

    // Full validation without building a tree.
    let mut d = Decoder::new(doc);
    assert_eq!(d.skip_item(), Ok(()));
    assert_eq!(d.finish(), Ok(()));

    let after = ALLOCS.load(Ordering::SeqCst);
    assert_eq!(after - before, 0, "borrowed decoding must not allocate");
}
