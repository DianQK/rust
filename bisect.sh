# good 13250
# bad 13500

set -e
rm -rf rustc-ice-*.txt
limit=$1

if [ -z "$limit" ]; then
    echo "need limit"
    exit 126
else
    echo "limit: $1"
fi

export RUSTFLAGS_NOT_BOOTSTRAP="-C llvm-args=-opt-bisect-limit=$limit -Z llvm-opt-bisect-limit-cgu=rustc_mir_build.63d28fcded2a05ed-cgu.007"
./x build --stage 2 library --keep-stage 0 --keep-stage-std 1 || exit 125
./build/host/stage2/bin/rustc ./tests/ui/consts/const_prop_slice_pat_ice.rs || exit 1

