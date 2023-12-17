# Inscriber 9000

inscriber9000 serves all your inscribing needs. It will take whatever you want to inscribe, sign
it with your private key, and then send it to the chain.

> [!CAUTION]
> Use at your own risk. It's likely broken.

## Getting Started

To run inscriber9000 you need to have rust installed. (See https://rustup.rs/). Then you can run:

```
cargo run \
    --private-key <your private key> \
    --chain=kusama \
    --remark='{"p":"ksm-20","op":"mint","tick":"sama","amt":"1000"}'`
```

To obtain your private key, you can use the `subkey` tool and copy "Secret seed" from the output:


```
$ subkey inspect "noodle able degree toast undo ...."

Secret phrase:       noodle able degree toast undo
0x62f368839aa66ff80379dc9110f92c12b66914f959e2f7ea3e6d2589be5dc594

$ subkey inspect "///Alice"
Secret Key URI `///Alice` is account:
  Network ID:        substrate
  Secret seed:       0x62f368839aa66ff80379dc9110f92c12b66914f959e2f7ea3e6d2589be5dc594
```
