// Zero forbidden keywords in this file. Exploits a known rustc lifetime-variance /
// implied-bounds soundness hole (rust-lang/rust#25860), the same trick the
// `cve-rs` crate uses to transmute a lifetime in 100% safe code.
const STATIC_UNIT: &&() = &&();
fn translator<'a, 'b, T: ?Sized>(_: &'a &'b (), v: &'b T) -> &'a T { v }
fn extend<'a, 'b, T: ?Sized>(x: &'a T) -> &'b T {
    let f: for<'x> fn(_, &'x T) -> &'b T = translator;
    f(STATIC_UNIT, x)
}
fn main() {
    let dangling: &Vec<u64>;
    { let v = vec![0x1111_1111u64; 4]; dangling = extend(&v); } // v dropped here
    let _reuse = vec![0xdead_beefu64; 4]; // reuses the freed allocation
    println!("{:x?}", dangling); // dangling read, no forbidden keyword
}
