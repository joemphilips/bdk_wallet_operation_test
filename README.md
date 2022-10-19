## how to run

```sh
# Create simple bip84 descriptor wallet and send from it.
cargo run -- --mode=send_bitcoin

# Same as above but this time wallet does not contain private key and use a signer interface.
cargo run -- --mode=send_from_watchonly
```

## Notes

* It only uses segwit v0.
* It is mostly based on [rpcwallet example](https://github.com/bitcoindevkit/bdk/blob/master/examples/rpcwallet.rs)
* In case of `send_bitcoin` mode, it prints out information necessary for recovering
funds. that is, bip39 master seed phrase and descriptor including the derivation path from it. I did not do that for watchonly wallet since it is trivial.


