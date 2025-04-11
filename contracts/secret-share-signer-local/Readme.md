### Running Integration Tests

1. Make sure the submodule at packages/tss-snap is initialized and updated:

    ```bash
    git submodule update --init --recursive
    ```
2. Install wasm-pack:

    ```bash
    cargo install wasm-pack
    ```
3. Start a local Secret Network node:

    ```bash
    (cd tests && make run-localsecret)
    ```

On Apple Silicon, this will not function. Use Gitpod instead for a quick dev environment: https://gitpod.io/#https://github.com/scrtlabs/GitpodLocalSecret

4. In [tests](tests) run:

    ```bash
    yarn build-mpc-ecdsa-wasm
    yarn install
    yarn test
    ```
Note: `yarn build-mpc-ecdsa-wasm` won't function as intended when a submodule in a different workspace; build it in its own folder.

Apple Silicon environments may fail to build secp256k1-sys. If so:

```
echo 'export AR=/opt/homebrew/opt/llvm/bin/llvm-ar' >> ~/.zshrc
echo 'export CC=/opt/homebrew/opt/llvm/bin/clang' >> ~/.zshrc
```

or, alternatively, add this to your .cargo/config.toml:
```
[env]
AR = "/opt/homebrew/opt/llvm/bin/llvm-ar"
CC = "/opt/homebrew/opt/llvm/bin/clang"
```

Note: You can overrride [tests/.env](tests/.env) variables by creating a [tests/.env.local](tests/.env.local) to set options like:
```bash
ENDPOINT=http://localhost:1317
CHAIN_ID=secretdev-1
```
