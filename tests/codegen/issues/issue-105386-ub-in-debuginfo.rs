// compile-flags: --crate-type=lib -O -Cdebuginfo=2 -Cno-prepopulate-passes -Zmir-enable-passes=-ScalarReplacementOfAggregates
// MIR SROA will decompose the closure
#![feature(stmt_expr_attributes)]

pub struct S([usize; 8]);

#[no_mangle]
pub fn outer_function(x: S, y: S) -> usize {
    (#[inline(always)]|| {
        let _z = x;
        y.0[0]
    })()
}

// FIXME: We need to find a new test case?
// Check that we do not attempt to load from the spilled arg before it is assigned to
// when generating debuginfo.
// CHECK-LABEL: @outer_function
// CHECK: [[spill:%.*]] = alloca %"{closure@{{.*.rs}}:9:23: 9:25}"
