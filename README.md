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


## Docker

TDA standalone for ease of testing and deployment is package into docker
container.

When running docker container make sure that you would setup properly netowrking
between docker and host as well as other TDA's which you would like to communicate
with. The simplest scenario is to run docker with host network:

    docker run --rm -p 1234:49152 --name tda --network host hcf/tda-standalone

### Build container

    DOCKER_BUILDKIT=1 docker build . -t hcf/tda-standalone
