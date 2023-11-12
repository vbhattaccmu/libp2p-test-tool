# libp2p-test-tool

`libp2p-test-tool` is a command line tool for interacting with a libp2p network. It does the following:-

- It checks and logs unreachable peers in the network.
- It logs newly connected peers, tries to identify it and resolves its IP.
- The logs are collected on separate CSVs whose paths are configurable from the tool itself.

## Installation steps

The tool can be installed by the following either of the two steps described below:-

### By building from public repository

The setup requires you to install Rust. For more details you can install it [here](https://www.rust-lang.org/tools/install).

Once you have Rust installed, the tool can be built with the following steps:-

- Clone the public repistory

  ```sh
  git clone https://github.com/vbhattaccmu/libp2p-test-tool.git
  ```

- Navigate to `libp2p-test-tool` directory and build the tool

  ```sh
  cd libp2p-test-tool

  cargo build --release
  ```

- Navigate to `target/release` and execute the binary

```sh
./libp2p-test-tool --help
```

The output will be as follows:-

```sh
Libp2p Network Interaction CLI

A command line tool for interacting with a libp2p network.

Usage: libp2p_test_tool <COMMAND>

Commands:
  generate-network-report
  help                     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

```sh
Usage: libp2p_test_tool generate-network-report [OPTIONS]

Options:
      --bootstrap-node-addrs <BOOTSTRAP_NODE_ADDRS>
          A set of bootstrapped libp2p node addresses from the network you want to generate metrics from.
          If not supplied the tool falls back to dialing the Avail network bootstrapped node.
      --just-connected <JUST_CONNECTED>
          Path to store a CSV report on newly connected nodes in the network.
      --unreachable <UNREACHABLE>
          Path to store a CSV report on non-reachable nodes in the network.
  -h, --help
          Print help
```

### By pulling image from DockerHub

Alternatively, you can use docker to run the tool. First pull it from DockerHub

```sh
docker pull vbhattac/libp2p-test-tool:latest
```

Or, you can add the image to your docker compose directly as follows:-

```sh
libp2p-testing-tool:
image: vbhattac/libp2p-test-tool:latest
command: generate-network-report
networks:
    net:
    ipv4_address: 172.16.3.6
volumes:
    - .${VOLUME_PATH}:/home
ports:
    - "7071:7071"
restart: on-failure
```

Here `VOLUME_PATH` is a volume mount to extract all logs. The env variable needs to be set separately beforehand on your terminal or from an external program.

## Example Usage

Command:-

```sh
./libp2p_test_tool generate-network-report --bootstrap-node-addrs "/ip4/127.0.0.1/udp/39000/quic-v1, /ip4/172.16.3.4/udp/37000/quic-v1"
```

Output:-

```sh
2023-11-12 03:58:19 Launching Libp2p network interaction tool...
2023-11-12 03:58:19 [2023-11-11T19:58:19Z INFO  libp2p_mdns::behaviour::iface] creating instance on iface 172.16.3.6
2023-11-12 03:58:19 [2023-11-11T19:58:19Z INFO  libp2p_mdns::behaviour] discovered: 12D3KooWBKFYLfJSp74AKkKpz8CgZhNChY3FhVNmCxiUrsdHCfQD /ip4/172.16.3.4/udp/37000/quic-v1
...
```

## Example usage of the tool in a network

As a used case, the tool has been tested on Avail network, whose artifacts are placed in `tests/compose/avail_setup_with_tool.yml`. The Avail network needs a fullnode, bootstrapped node and a few light clients to get started. Their images have been included in the docker compose files.

The testing with network can be performed in the following ways:-

### By manually building the network from artifacts

From the top level of `libp2p-test-tool` directory:-

```sh
export VOLUME_PATH=/home

docker compose -f tests/compose/avail-setup-with-tool.yml up -d
```

if you tail the logs of the libp2p-test-tool container:-

```
docker logs --tail 50 -f compose-libp2p-testing-tool-1
```

You would see something like this:-

```sh
Launching Libp2p network interaction tool...
[2023-11-10T17:18:50Z INFO  libp2p_mdns::behaviour::iface] creating instance on iface 172.16.3.6
[2023-11-10T17:18:50Z INFO  libp2p_mdns::behaviour] discovered: 12D3KooWBKFYLfJSp74AKkKpz8CgZhNChY3FhVNmCxiUrsdHCfQD /ip4/172.16.3.4/udp/37000/quic-v1
...
```

### By running the testing module

A set of tests have been designed to showcase operation of the tool to provide solutions to Task 1, 2 and 3.

- test_unreachable_peer_log: This test tests results for Task 1 and logs unreachable peers in a CSV(unreachable.csv) on `VOLUME_PATH`.
  - It spawns a set of services and then disconnects one of the clients whose udp port is not exposed to the host machine.
  - The test checks the existence of disconnected peer in the records for unreachable peers.
- test_new_peer_join_and_ip_resolution: This test tests results for Task 2 and 3 and logs reachable peers
  and their resolved IPs along with timestamps in a CSV (newly_connected.csv) on `VOLUME_PATH`.
  - The test adds a new peer in the middle of operation of the network and then checks its existence in logs for newly connected peers.
  - Since no peer is disconnected, it also asserts if the unreachable peer list is not empty.

First, make sure you have [Docker](https://docs.docker.com/engine/install/) and the [Docker Compose plugin](https://docs.docker.com/compose/install/linux/) installed.

The tests require you to run Docker with admin level privileges. It is recommended that the artifacts be installed earlier to prevent delay using
the `avail-setup-with-tool.yml` file inside `tests/compose`.

The tests can be executed by the following command:-

Command:-

```sh
cargo test -- --test-threads=1
```

Output:-

```sh
Running tests/tests.rs (target/debug/deps/tests-ec04733da131681c)

running 2 tests
test test_new_peer_join_and_ip_resolution ... ok
test test_unreachable_peer_log ... ok
```

## Solution commits

The solutions are covered across 3 major commits. They are provided below:-

Task 1: Link: [665e44430fbd59](https://github.com/vbhattaccmu/libp2p-test/tree/665e44430fbd599db4e50614989d4f0135f668aa)

Description: `Solution to Task 1: Added OutgoingConnectionError tracking which logs multi addresses which cannot be dialled.`

Task 2: Link: [d711e86f9a2cd5](https://github.com/vbhattaccmu/libp2p-test/tree/d711e86f9a2cd5551724c02f23ed09c217f1b037)

Description: `Solution to Task 2: Log newly connected peers through identify.`

Task 3A: Link: [7f7bf5d9c052bb3](https://github.com/vbhattaccmu/libp2p-test/tree/7f7bf5d9c052bb374a2a654557197d44187913e8)

Description: `Partial Solution to Task 3: Add dns config to both tcp and quic in transport.`

Task 3B: Link: [c9a1335d25c211](https://github.com/vbhattaccmu/libp2p-test/tree/c9a1335d25c211050159039f97f0381f7e094f1e)

Description: `Remaining part to solution to Task 3: Resolve IP for newly connected peers.`

## Time Logs

The time boxed for each set of commits includes time spent in coding as well as testing the changes in a separate repository.

- 6-7th November, 2023: Design of the protocol for CSV writer, Tasks 1, 2 and 3 and coming up with initial module skeleton. ( ~2 hours )
- 8th November, 2023: Commits [f74be0c86db2fc](https://github.com/vbhattaccmu/libp2p-test/tree/f74be0c86db2fc652060c6c338ede279993e5d8b) to [cb28ee65ebacfb2](https://github.com/vbhattaccmu/libp2p-test/tree/cb28ee65ebacfb274e2ec6ac1acabc8f81ee7ce8) ( ~2 hours )
- 9-10th November, 2023: Commits [16e727a479653a2](https://github.com/vbhattaccmu/libp2p-test/tree/16e727a479653a2731d63b85333b4de0fa5c4aaa) to [2b8d2aaf2a762](https://github.com/vbhattaccmu/libp2p-test/tree/2b8d2aaf2a762af04ab395b67721d7f3998e1116) ( ~2.5 hours )
- 11-12th November, 2023: Commits [665e44430fbd5](https://github.com/vbhattaccmu/libp2p-test/tree/665e44430fbd599db4e50614989d4f0135f668aa) to [c9a1335d25c211](https://github.com/vbhattaccmu/libp2p-test/tree/c9a1335d25c211050159039f97f0381f7e094f1e) ( ~2 hours )

## Supported OSes for the testing module

The tool is OS agnostic but the tests work with the following OSes. The testing module requires you to start your docker engine with admin level privileges.

|           OS           |         Specs (Hardware)          | Compatible |
| :--------------------: | :-------------------------------: | :--------: |
|         macOS          | Chip: Apple M1 Pro, Memory: 16 GB |    Yes     |
| Ubuntu 22.04 LTS Image | Chip: Apple M1 Pro, Memory: 16 GB |    Yes     |

The testing module still needs to upgraded for other OSes.
