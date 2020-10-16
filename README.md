# tda-standalone
Trusted Digital Assistant - standalone version, leveraging KERI, OCA and DDE

## Usage

To run server-only instance at localhost:1234 use:
```
cargo run -- -H "localhost:1234" -s
```

To run client-only instance at localhost:1234 use:
```
cargo run -- -H "localhost:1234" -c
```
By default application runs in client mode.