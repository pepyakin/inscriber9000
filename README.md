# Inscriber 9000

inscriber 9000 serves all your inscribing needs. It will take whatever you want to inscribe, sign
it with your private key, and then send it to the chain.

## Getting Started

To install you need to have rust installed. (See https://rustup.rs/). Then you can install
inscriber9000 with:

```
cargo install --force inscriber9000
```

Usage:

```
inscriber9000 \
    --private-key <your private key> \
    --chain=kusama \
    --remark='{"p":"ksm-20","op":"mint","tick":"sama","amt":"1000"}'` \
    --inflight-num=100
```

To obtain your private key, you can use the `subkey` tool:


```
$ subkey inspect "noodle able degree toast undo ...."

Secret phrase:       noodle able degree toast undo
...
```

Alternatively, you could check out the repo and run it with cargo:

```
cargo run \
    --private-key <your private key> \
    --chain=kusama \
    --remark='{"p":"ksm-20","op":"mint","tick":"sama","amt":"1000"}'` \
    --inflight-num=100
```
