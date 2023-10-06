#![feature(inline_const)]

fn main() {
    const {
        assert!(-9.223372036854776e18f64 as i64 == 0x8000000000000000u64 as i64);
    }
}
