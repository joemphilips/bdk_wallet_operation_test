## how to run

```sh
# Create simple bip84 descriptor wallet and send from it.
cargo run -- --mode=send_bitcoin

# Same as above but this time wallet does not contain private key and use a signer interface.
# It also takes a backup for the wallet in a json format in a file `/path/to/tmp/wallet.bck`
cargo run -- \
  --mode=send_from_watchonly \
  --datadir=/path/to/tmp
```

## Notes

* It only uses segwit v0, and it assumes bip84 derivation path.
* It is mostly based on [rpcwallet example](https://github.com/bitcoindevkit/bdk/blob/master/examples/rpcwallet.rs)
* In case of `send_bitcoin` mode, it prints out information necessary for recovering
  Recovering funds from other


