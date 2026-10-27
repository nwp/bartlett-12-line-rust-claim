# bartlett-12-line-rust-claim

Does Rust's *safe subset* really guarantee memory safety? A ~12-line, 100%-safe
use-after-free, written to test a claim made on X.

## The claim

Jonathan Bartlett ([@jb_61820](https://x.com/jb_61820/status/2114879522256634104),
replying to [@codewith55](https://x.com/codewith55/status/2114726146109772038)):

> "The guarantees are fake. Even when restricting yourself to the safe subset it
> actually has no real safety guarantees. You can write a memory unsafe app in
> about 12 lines."

## Verdict

**The narrow, literal claim holds — with an important caveat.**

- You *can* write a program in **12 lines of logic**, using **zero `unsafe`**
  (`grep -c unsafe safe_uaf.rs` → `0`), that **compiles cleanly** on stock
  `rustc 1.85.1` and, at runtime, **reads memory through a dangling reference to a
  freed allocation** — a genuine use-after-free. See `safe_uaf.rs` below.
- **Caveat:** this is *not* a hole the safety model permits by design. It triggers
  a **known, long-standing compiler soundness bug**
  ([rust-lang/rust#25860](https://github.com/rust-lang/rust/issues/25860) — the
  lifetime-variance / implied-bounds hole, the same one the
  [`cve-rs`](https://github.com/Speykious/cve-rs) crate exploits). The code is
  deliberately contrived; normal safe Rust never hits it, and the type checker
  rejects the *naive* version of the trick (see "Attempts" below).

So **"you can write a memory-unsafe program in ~12 lines of safe Rust" is literally
true today.** But **"the guarantees are fake" overstates it**: this is one tracked
compiler bug reachable only by code written specifically to reach it, not a design
gap that ordinary programs fall into.

## Environment

```
$ rustc --version
rustc 1.85.1 (4eb161250 2025-03-15) (built from a source tarball)
```

## The headline program — `safe_uaf.rs` (safe, no `unsafe`)

```rust
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
```

- Logic lines (non-blank, non-comment): **12**
- `grep -c unsafe safe_uaf.rs`: **0**

### Compile + run

```
$ grep -c unsafe safe_uaf.rs
0

$ rustc safe_uaf.rs -o safe_uaf        # compiles, exit 0, no errors
$ ./safe_uaf
[deadbeef, deadbeef, deadbeef, deadbeef]
$ ./safe_uaf; echo $?
[deadbeef, deadbeef, deadbeef, deadbeef]
0
```

The freed `Vec`'s heap buffer is re-handed to `_reuse`, and we read it back through
`dangling` — classic use-after-free reading reused memory. Captured over 3 runs the
no-opt output was stable: `[deadbeef, deadbeef, deadbeef, deadbeef]` each time.

With optimizations the corruption is more visible — the `Vec` header itself is
reinterpreted, so both the length and contents come out as garbage that **changes
every run** (reading live stack/heap bytes):

```
$ rustc -O safe_uaf.rs -o safe_uaf_O
$ ./safe_uaf_O    # run 1
[55d2d3ab572b, 828810bd76e50101]
$ ./safe_uaf_O    # run 2
[55d72c5c1da0, 838b663f2c64b37e]
$ ./safe_uaf_O    # run 3
[55e5cdba5126, eeefbf46e2c7c17b]
```

No crash/segfault was observed (exit code `0` in every run, both modes); the
unsafety manifests as **silent memory corruption / garbage reads**, which is the
more dangerous outcome.

## The contrast — `unsafe_ptr.rs` (explicit `unsafe`)

For comparison, the trivial version using an explicit `unsafe` block. It proves
nothing about *safe* Rust — it just asks the compiler's permission to do the same
dangling read:

```rust
// The trivial version: an explicit `unsafe` block. Proves nothing about safe Rust.
fn main() {
    let p: *const Vec<u64>;
    { let v = vec![1u64, 2, 3, 4]; p = &v; }
    let _reuse = vec![0xdead_beefu64; 4];
    println!("{:x?}", unsafe { &*p });
}
```

- Logic lines (non-blank, non-comment): **6**
- `grep -c unsafe unsafe_ptr.rs`: **2**
- Output (stable): `[deadbeef, deadbeef, deadbeef, deadbeef]`, exit `0`.

## Attempts

See `ATTEMPTS.md` for the full log. Summary:

1. **Naive #25860 `static UNIT` trick → REJECTED.** Annotating the translator as a
   monomorphic `fn(_, &'a T) -> &'static T` makes rustc demand `'a: 'static`:
   `error: lifetime may not live long enough ... requires that 'a must outlive 'static`.
   Modern rustc catches this form.
2. **Higher-ranked (HRTB) variant → COMPILES + runs.** Changing the annotation to
   `for<'x> fn(_, &'x T) -> &'b T` hides the constraint behind a higher-ranked
   bound; rustc fails to propagate the implied bound and type-checks the lifetime
   laundering. This is the headline `safe_uaf.rs`.

## Bottom line

The 12-line number checks out. The honest framing: safe Rust's guarantees are not
"fake," but they are not *absolute* either — there are known, open compiler
soundness bugs (tracked for years) that contrived safe code can exploit to defeat
them. For real-world code the guarantees hold; for an adversary writing to the bug,
they don't.
