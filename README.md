# `magic-loc-central`

This repo is the messaging central for the `magic-loc-rs` firmware. It decodes serial packets from a raw serial stream and forwards them to the appropriate handler.

We use ZeroMQ for messaging between the central and all downstream task nodes (e.g. sensor fusion, visualization, etc).

## Usage

Stream (for a monitor node):
```
Usage: magic-loc-stream [OPTIONS] --serial-ports <SERIAL_PORTS>...

Options:
  -v, --verbose...                      Increase verbosity, and can be used multiple times
  -s, --serial-ports <SERIAL_PORTS>...  Serial port devices
  -h, --help                            Print help
  -V, --version                         Print version
```

Central (for IMU+UWB tag nodes):
```
Usage: magic-loc-central [OPTIONS] --serial-ports <SERIAL_PORTS>...

Options:
  -v, --verbose...                      Increase verbosity, and can be used multiple times
  -z, --zmq-addr <ZMQ_ADDR>             ZMQ listen address [default: tcp://*:5555]
  -s, --serial-ports <SERIAL_PORTS>...  Serial port devices
  -h, --help                            Print help
  -V, --version                         Print version
```

# LICENSE

```
Copyright 2023 Fan Jiang

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
