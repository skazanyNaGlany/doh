# doh

A powerful Kubernetes log aggregation tool that streams and processes logs from multiple contexts simultaneously.

`doh` acts as an intelligent wrapper around `kubectl` and `stern`, providing enhanced log processing capabilities including JSON parsing, message filtering, and multi-context support.

## Features

- **Multi-context support**: Aggregate logs from multiple Kubernetes contexts simultaneously
- **Intelligent JSON parsing**: Automatically detects and formats JSON log messages
- **Advanced filtering**: Filter logs by container name, skip invalid messages, and more
- **Message enhancement**: Clean up timestamps, pretty-print JSON objects, and format output
- **Flexible output**: Display to stdout, save to file, or both
- **Real-time streaming**: Follow logs in real-time with the `--follow` option
- **Non-blocking I/O**: Efficient concurrent processing of multiple log streams

## Prerequisites

Before using `doh`, ensure you have the following tools installed and available in your PATH:

- **kubectl** - The Kubernetes command-line tool
- **stern** - Multi-pod and multi-container log tailing for Kubernetes
  - Install from: https://github.com/stern/stern

## Installation

### From Source

```bash
git clone <repository-url>
cd doh
cargo build --release
```

The compiled binary will be available at `target/release/doh`.

### Development Build

```bash
cargo build
cargo run -- --help
```

## Usage

### Basic Usage

```bash
doh -- <pod-query>
```

### Examples

**Stream logs from nginx pods:**
```bash
doh -- nginx
```

**Get logs from multiple contexts:**
```bash
doh -c staging,production -- myapp
```

**Get logs from all available contexts:**
```bash
doh -c all -- myapp
```

**Save logs to file with auto-generated filename:**
```bash
doh -f -- myapp
```

**Save logs to specific file:**
```bash
doh -f myapp-logs.txt -- myapp
```

**Filter logs from specific containers:**
```bash
doh -i app,sidecar -- myapp
```

**Follow logs in real-time:**
```bash
doh -g -- myapp
```

**Get logs from the last 30 minutes with pretty JSON formatting:**
```bash
doh -t 30m -p true -- myapp
```

**Process all contexts simultaneously (use with caution):**
```bash
doh -c all -a true -- myapp
```

## Command Line Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--help` | `-h` | Show help message | |
| `--context <string>[,...]` | `-c` | Select context(s) separated by comma or use "all" | `default` |
| `--all-at-once <bool>` | `-a` | Gather logs from all contexts simultaneously (use with caution) | `false` |
| `--skip-invalid-messages <bool>` | `-s` | Skip non-JSON messages from Stern | `false` |
| `--blank-line-after-entry <bool>` | `-b` | Add blank line after each log entry | `false` |
| `--include-container <string>[,...]` | `-i` | Include logs from specific container(s) | `all` |
| `--save <filename>` | `-f` | Save logs to file (empty for auto-generated name) | |
| `--work-dir` | `-w` | Set working directory | |
| `--fix-up-messages <bool>` | `-m` | Remove redundant data like timestamps | `true` |
| `--pretty-print-objects <bool>` | `-p` | Pretty print JSON objects (experimental) | `false` |
| `--since <duration>` | `-t` | Return logs newer than duration (for example 5s, 2m, 3h, etc.) | `1h` |
| `--space-after-message <bool>` | `-r` | Add space after each message | `true` |
| `--follow` | `-g` | Wait for new messages (real-time streaming) | |
| `--quiet` | `-q` | Don't output to stdout (useful with `--save`) | |

## Log Processing Features

### JSON Message Handling

`doh` automatically detects and processes different types of log messages:

1. **Plain JSON logs**: Automatically parsed and formatted
2. **Timestamped messages**: Extracts timestamps and separates message content
3. **Exception logs**: Special formatting for logs with `exc_info` and `message` fields
4. **Proxy logs**: Specialized formatting for HTTP proxy logs with request details

### Output Format

Each log entry follows this format:
```
<context> <pod_name> <container_name> <timestamp>    <message>
```

Example:
```
production myapp-deployment-abc123 app 2023-10-15T10:30:45Z    Starting application server
staging myapp-deployment-def456 sidecar 2023-10-15T10:30:46Z    proxy started
```

## Architecture

`doh` is built with a modular architecture:

- **Command Streaming**: Non-blocking execution of multiple `stern` processes
- **JSON Processing**: Intelligent parsing and formatting of structured log data
- **Context Management**: Discovery and management of Kubernetes contexts
- **Message Processing**: Regex-based cleanup and enhancement of log messages

## Performance Considerations

- **Multi-context processing**: Use `--all-at-once` carefully as it can generate significant network traffic
- **Container filtering**: Use `--include-container` to reduce log volume
- **Time ranges**: Use `--since` to limit the time window for log retrieval
- **Output options**: Use `--quiet` with `--save` to reduce terminal output overhead

## Troubleshooting

### Common Issues

**"kubectl not found"**: Ensure `kubectl` is installed and in your PATH
**"stern not found"**: Install `stern` from https://github.com/stern/stern
**No contexts found**: Check your `kubectl` configuration with `kubectl config get-contexts`
**Permission denied**: Ensure you have proper RBAC permissions for the target namespaces

### Debug Information

Add these options for more verbose output:
- Use `--save` to capture logs for analysis
- Check the auto-generated log files for complete output

## Contributing

This project is written in Rust. To contribute:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test` (TODO)
5. Check formatting: `cargo fmt`
6. Run linter: `cargo clippy`
7. Submit a pull request

## License

See [LICENSE](LICENSE) file for details.
