# Step by step build

## Building gotham-client

```bash
cd gotham-client
```

```bash
make init
```

For iOS

```bash
make ios
```

Once compilation completes, we will get

```bash
    Finished release [optimized] target(s) in 2m 53s
[INFO  cargo_lipo::lipo] Creating universal library for gotham-client
[DONE] target/universal/release/libexample.a
```

Then, we use `cbindgen` to generate a C header file

```bash
cbindgen src/lib.rs -l c > libexample.h
```

Now that we have `libexample.a` and `libexample.h`, we can copy them into our flutter plugin repository.
