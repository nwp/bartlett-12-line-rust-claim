// The trivial version: an explicit `unsafe` block. Proves nothing about safe Rust.
fn main() {
    let p: *const Vec<u64>;
    { let v = vec![1u64, 2, 3, 4]; p = &v; }
    let _reuse = vec![0xdead_beefu64; 4];
    println!("{:x?}", unsafe { &*p });
}
