# Safe-code attempt log (rustc 1.85.1, 4eb161250, 2025-03-15)

## Attempt 1 — naive rust-lang/rust#25860 `static UNIT` trick — FAILED to compile
Source:
    static UNIT: &'static &'static () = &&();
    fn helper<'a, 'b, T>(_: &'a &'b (), v: &'b T) -> &'a T { v }
    fn extend<'a, T>(x: &'a T) -> &'static T {
        let f: fn(_, &'a T) -> &'static T = helper;
        f(UNIT, x)
    }
Exact error:
    error: lifetime may not live long enough
     --> safe_uaf.rs:5:12
      |
    4 | fn extend<'a, T>(x: &'a T) -> &'static T {
      |           -- lifetime `'a` defined here
    5 |     let f: fn(_, &'a T) -> &'static T = helper;
      |            ^^^^^^^^^^^^^^^^^^^^^^^^^^ type annotation requires that `'a` must outlive `'static`
    error: aborting due to 1 previous error
Why: the monomorphic `fn(_, &'a T) -> &'static T` annotation forces `'a: 'static`,
which rustc correctly rejects. Modern rustc catches the naive form.

## Attempt 2 — cve-rs higher-ranked (HRTB) variant — COMPILES + runs (headline)
Change vs. attempt 1: annotate `f` as `for<'x> fn(_, &'x T) -> &'b T`. The
higher-ranked bound hides the `'a: 'static` constraint; rustc fails to propagate
the implied bound (rust-lang/rust#25860), so arbitrary lifetime extension type-checks.
See safe_uaf.rs. Logic lines (non-blank, non-comment): 12. `grep -c unsafe`: 0.
