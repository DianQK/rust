./x build --stage 2 library --keep-stage 0 --keep-stage-std 1 2> build.log
./build/host/stage2/bin/rustc ./test.rs # -Z treat-err-as-bug

