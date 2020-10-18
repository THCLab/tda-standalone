# tda-standalone
Trusted Digital Assistant - standalone version, leveraging KERI, OCA and DDE

## Usage

To run

    cargo run -- -H localhost -P 1234

## Development

For development you can use cargo watch and run code like this:

    cargo watch -x 'run -- -P 1234'

By default application runs on localhost and port 49152


To control TDA you can use telnet connecting on the setup port and send
commands. Supported commands:

SEND host port - send last event to given TDA (via TCP)
ROTA - generate rotate event

