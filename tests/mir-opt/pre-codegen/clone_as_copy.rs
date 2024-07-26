//@ compile-flags: -Cdebuginfo=full

// Check if we have transformed the nested clone to the copy in the complete pipeline.

#[derive(Clone)]
struct AllCopy {
    a: i32,
    b: u64,
    c: [i8; 3],
}

#[derive(Clone)]
struct NestCopy {
    a: i32,
    b: AllCopy,
    c: [i8; 3],
}

// EMIT_MIR clone_as_copy.clone_as_copy.PreCodegen.after.mir
#[inline(never)]
fn clone_as_copy(v: &NestCopy) -> NestCopy {
    // CHECK-LABEL: fn clone_as_copy(
    // CHECK-NOT: = AllCopy { {{.*}} };
    // CHECK-NOT: = NestCopy { {{.*}} };
    // CHECK: _0 = (*_1);
    // CHECK: return;
    v.clone()
}
