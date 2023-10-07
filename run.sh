./x build --rust-profile-generate=../tmp/profiles --stage 2 library 2> build.log
./build/host/stage2/bin/rustc ./test.rs # -Z treat-err-as-bug
