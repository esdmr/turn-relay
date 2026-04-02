# Multi-peer, UDP Relay over TURN

```mermaid
flowchart LR
	gui[GUI
TURN_RELAY_HEADLESS≠1]

	control[HTTP Controller
TURN_RELAY_CONTROL=1]

	subgraph Worker
		coord(Coordinator)

		relay[Relay]

		peers[Peer 1
Peer 2
⋮
Peer n]
	end

	gui -- command --> coord
	coord -- service --> gui

	control -- command --> coord
	coord -- service --> control

	coord -- command --> relay
	coord -- command --> peers
	relay -- service --> coord
	peers -- service --> coord

	coord -- upstream --> relay
	peers -- upstream --> coord
	relay -- downstream --> coord
	coord -- downstream --> peers
```

This project relays UDP packets over a TURN server, to bypass NAT restrictions. Support for multiple peers allows reusing the TURN allocation for multiple peers.

## Getting started

Install Rust tool chain, then run:

```sh
cargo run
```

To build an executable, run:

```sh
cargo build --release
```

The built file should be under `target/release`.
