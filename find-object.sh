for bitcode in build/x86_64-unknown-linux-gnu/stage1-rustc/x86_64-unknown-linux-gnu/release/deps/rustc_mir_build-*-cgu.*.rcgu.no-opt.bc; do
    if llvm-nm -U $bitcode | grep -q "_RNCINvMs4_NtNtNtCs8zlSoIwVZ9h_15rustc_mir_build4thir7pattern15deconstruct_patNtB8_11Constructor5splitINtNtNtNtCse4hYLEadXti_4core4iter8adapters3map3MapIB1C_INtNtNtB1K_5slice4iter4IterNtNtBa_10usefulness8PatStackENCNvMs2_B2X_NtB2X_6Matrix5heads0ENvMs7_B8_NtB8_16DeconstructedPat4ctorEEs0_0Be_"; then
        echo $bitcode
    fi
done

# build/x86_64-unknown-linux-gnu/stage1-rustc/x86_64-unknown-linux-gnu/release/deps/rustc_mir_build-3f733e244805fff3.rustc_mir_build.63d28fcded2a05ed-cgu.007.rcgu.no-opt.bc
