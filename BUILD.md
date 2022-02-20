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
cbindgen src/lib.rs -l c > libclient.h
```

Now that we have `libclient_lib.a` and `libclient.h`, we can copy them into our flutter plugin repository.

Getting back up to our project root

```bash
cd ..
```

As an example,

```bash
# This assumes that we have a flutter-rus-plugin git repo in the directory path at the same directory level as our project root

cp gotham-client/target/universal/release/libclient_lib.a ../flutter-rust-plugin/ios/

cp gotham-client/libclient.h ../flutter-rust-plugin/ios/Classes/
```