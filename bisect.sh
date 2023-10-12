export RUSTFLAGS_NOT_BOOTSTRAP="-C llvm-args=-opt-bisect-limit=-1 -Z llvm-opt-bisect-limit-cgu=rustc_mir_build.63d28fcded2a05ed-cgu.007"
./x build --stage 2 library --keep-stage 0 --keep-stage-std 1 2> build.log
./build/host/stage2/bin/rustc ./tests/ui/consts/const_prop_slice_pat_ice.rs
