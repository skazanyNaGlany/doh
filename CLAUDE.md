# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`doh` is a Rust command-line tool for downloading and processing logs from Kubernetes clusters. It acts as a wrapper around `kubectl` and `stern` to aggregate logs from multiple contexts, parse JSON messages, and format output with various filtering options.

## Architecture

The application follows a modular structure:

- **Main execution flow**: `main.rs` handles argument parsing, binary validation, and orchestrates the entire log collection process
- **Command streaming**: `command_streamer/` module provides infrastructure for running and monitoring multiple external commands (kubectl/stern) concurrently
- **Kubernetes integration**: `kubectl/` module handles Kubernetes context discovery and management
- **Message processing**: Various modules handle JSON parsing, regex matching, and log formatting:
  - `stern_json.rs` - Parses JSON output from stern
  - `message_regex.rs` - Regex patterns for message cleanup
  - `json_utils.rs` - JSON manipulation utilities

## Key Dependencies

The tool requires two external binaries to be installed:
- `kubectl` - Kubernetes CLI tool
- `stern` - Multi-pod log tailing tool (from https://github.com/stern/stern)

## Common Commands

Build the project:
```bash
cargo build
```

Run with development profile:
```bash
cargo run -- [options] -- <pod-query>
```

Build release version:
```bash
cargo build --release
```

Run tests (TODO):
```bash
cargo test
```

Check code formatting:
```bash
cargo fmt --check
```

Run linter:
```bash
cargo clippy
```

## Core Workflow

1. **Argument parsing**: Uses custom `ArgParser` to handle complex CLI arguments
2. **Context discovery**: Retrieves available Kubernetes contexts via `kubectl`
3. **Command execution**: Spawns `stern` processes for each context using `MultiCommandStreamer`
4. **Stream processing**: Continuously reads JSON logs from stern processes
5. **Message formatting**: Applies various transformations (timestamp removal, JSON pretty-printing, etc.)
6. **Output**: Displays formatted logs to stdout and optionally saves to file

## Key Features

- Multi-context log aggregation (can process logs from multiple Kubernetes contexts)
- JSON message parsing and formatting
- Container filtering with `--include-container`
- Message cleanup and pretty-printing
- Log file output with automatic filename generation
- Follow mode for real-time log streaming
