# apps-ports

A CLI tool to find and stop applications using specific ports. Never get "port already in use" errors again!

## Features

- 🔍 **List all processes** using network ports
- 🎯 **Check specific port** usage
- ⚡ **Kill processes** using ports with confirmation
- 📊 **Nice table output** with process details
- 🔒 **Safe operation** with user confirmation
- 🛡️ **Sudo fallback** for protected processes

## Installation

### Pre-built Binaries (Recommended)

Download the latest binary for your platform from the [releases page](https://github.com/eyalev/apps-ports/releases):

- Linux x86_64
- macOS (Intel)
- macOS (Apple Silicon)
- Windows

### From Source

```bash
# Install from GitHub
cargo install --git https://github.com/eyalev/apps-ports

# Or clone and build locally
git clone https://github.com/eyalev/apps-ports
cd apps-ports
cargo install --path .
```

## Usage

### List all processes using ports
```bash
apps-ports
# or
apps-ports --list
```

Example output:
```
+------+-------+--------------+----------------+
| port | pid   | process_name | command        |
+------+-------+--------------+----------------+
| 3000 | 12264 | node         | node server.js |
| 8080 | 15432 | java         | java -jar app.jar |
+------+-------+--------------+----------------+
```

### Check which process is using a specific port
```bash
apps-ports --port 3000
# or
apps-ports -p 3000
```

### Kill process using a specific port
```bash
apps-ports --kill 3000
# or
apps-ports -k 3000
```

The tool will:
1. Show you which process is using the port
2. Ask for confirmation before killing
3. Try with regular permissions first
4. Offer sudo fallback if needed

### Help
```bash
apps-ports --help
```

## How it works

The tool uses both `netstat` and `lsof` commands to comprehensively find processes using network ports. It provides detailed information including:

- Port number
- Process ID (PID)
- Process name
- Full command line

## Requirements

- Linux or macOS (Windows support coming soon)
- `netstat` command (usually pre-installed)
- `lsof` command (usually pre-installed)

## License

MIT License - feel free to use this tool in your projects!

## Contributing

Issues and pull requests are welcome! Feel free to suggest improvements or report bugs.