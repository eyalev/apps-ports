# apps-ports

A CLI tool to find and stop applications using specific ports. Never get "port already in use" errors again!

## Features

- 🔍 **List all processes** using network ports
- 🎯 **Check specific port** usage
- ⚡ **Kill processes** using ports with confirmation
- 🐳 **Docker container support** - Kill Docker containers instead of just processes
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

### Kill Docker container using a specific port
For Docker containers running on ports (detected via docker-proxy processes):
```bash
apps-ports -k 8080 --kill-docker-container
```

This will:
1. Detect if the process is a docker-proxy
2. Find the associated Docker container
3. Stop the entire container instead of just killing the proxy process
4. Optionally remove the stopped container

The tool will:
1. Show you which process is using the port
2. Ask for confirmation before killing
3. Try with regular permissions first
4. Offer sudo fallback if needed

### Help
```bash
apps-ports --help
```

## Output Formats

The tool offers multiple output formats for different use cases:

### Table Format (default)
```bash
apps-ports -p 3000
```
Displays results in a formatted table with columns for port, PID, process name, command, Docker ID, and Docker image.

### Simple Format (`-s` or `--simple`) - **Recommended for terminals**
```bash
apps-ports -p 3000 --simple
```
**Output:** `3000:12264 node (node server.js)`

One-line format perfect for terminal viewing, especially with long data.

### Compact Format (`-c` or `--compact`)
```bash
apps-ports -p 3000 --compact
```
Multi-line detailed view with clear labeling.

### JSON Format (`-j` or `--json`)
```bash
apps-ports -p 3000 --json
```
Perfect for scripting and automation.

## Running with Elevated Privileges

Many processes (especially Docker containers) run with elevated privileges and require `sudo` to see process details.

### System-wide Installation (Recommended)
Install the tool system-wide so it's available to both your user and `sudo`:

```bash
# Install system-wide
sudo cp ~/.local/bin/apps-ports /usr/local/bin/

# Now you can use with sudo
sudo apps-ports -p 8080 --simple
```

### Alternative Methods
If you can't install system-wide, use these alternatives:

```bash
# Method 1: Use full path with sudo
sudo ~/.local/bin/apps-ports -p 8080

# Method 2: Preserve environment PATH
sudo env PATH=$PATH apps-ports -p 8080
```

### Docker Container Detection
When processes show `(elevated privileges required)`, they're often Docker containers:

```bash
# Regular user - limited info
apps-ports -p 8080
# Output: "elevated privileges required"

# With sudo - full Docker info
sudo apps-ports -p 8080 --simple
# Output: "8080:363030 docker-proxy (/usr/bin/docker-proxy...) [🐳 82fee02d]"
```

## How it works

The tool uses `ss`, `netstat`, and `lsof` commands to comprehensively find processes using network ports. It provides detailed information including:

- Port number
- Process ID (PID) 
- Process name
- Full command line
- Docker container ID and image (when applicable)

## Requirements

- Linux or macOS (Windows support coming soon)
- `ss` or `netstat` command (usually pre-installed)
- `lsof` command (usually pre-installed)
- `docker` command (optional, for Docker container detection)

## License

MIT License - feel free to use this tool in your projects!

## Contributing

Issues and pull requests are welcome! Feel free to suggest improvements or report bugs.