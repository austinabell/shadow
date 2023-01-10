# shadow

Binary to run a command and track the resource usage within a terminal UI. This is intended to be more ergonomic and convenient than running commands and system monitoring tools independently.

## Installation

```
cargo install --path .
```

<!-- TODO crates io installation if published -->

## Usage

Simply prefix a command with `shadow`, which start the command as a new process and monitor the system information about that process.

This can also be done after a command is run using:

```sh
shadow !!
```

to re-run using `shadow` to monitor.

Try using shadow with an example binary:

```
shadow cargo run --example simulate
```

## TODO

- [ ] Configure how much data to keep
- [ ] Configure polling frequency
- [ ] Add support for network usage by pid(s) (platform agnostic solutions not clear)
- [ ] Define standard log format
	- [ ] Configure writing to file or piping the serialized data
	- [ ] Allow showing terminal UI from stream of logs
- [ ] More data
	- [ ] Total time
	- [ ] Average CPU
	- [ ] Average memory
	- [ ] Memory delta per second
	- [ ] Bytes written/read per second
- [ ] Pick up spawned processes to include with output
- [ ] 