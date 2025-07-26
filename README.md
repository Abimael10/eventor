# Eventor - Kafka Protocol Event Stream Processor

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen?style=for-the-badge)]()

A lightweight, high-performance event stream processor implementing the Kafka wire protocol in Rust. Eventor provides a minimal but compliant Kafka server implementation that can handle multiple concurrent connections and process streaming events efficiently.

## 🚀 Features

- **Kafka Wire Protocol Compliance** - Implements core Kafka protocol specifications
- **Multi-threaded Connection Handling** - Concurrent client support with thread-per-connection model
- **APIVersions Support** - Advertises supported API versions to clients
- **DescribeTopicPartitions** - Handles topic metadata requests with proper error responses
- **Correlation ID Tracking** - Maintains request/response correlation for reliable messaging
- **Unknown Topic Handling** - Graceful error responses for non-existent topics
- **Persistent Connections** - Supports multiple requests per connection
- **Memory Safe** - Built with Rust's safety guarantees

## 📋 Supported Kafka APIs

| API | Key | Version | Status | Description |
|-----|-----|---------|--------|-------------|
| APIVersions | 18 | 0-4 | ✅ | Returns supported API versions |
| DescribeTopicPartitions | 75 | 0 | ✅ | Describes topic partition metadata |

## 🛠️ Installation

### Prerequisites

- **Rust 1.70+** - [Install Rust](https://rustup.rs/)
- **Python 3.6+** - For running tests

### Build from Source

```bash
git clone https://github.com/Abimael10/eventor.git
cd eventor
cargo build --release
```

## 🚀 Quick Start

### Start the Server

```bash
cargo run --release
```

The server will start listening on `127.0.0.1:9092` by default.

### Run Tests

```bash
chmod +x tests.sh
./tests.sh
```

### Manual Testing with netcat

```bash
# Test basic connectivity
nc -v 127.0.0.1 9092
```

## 🧪 Testing

The project includes a test suite that validates protocol compliance and concurrent behavior.

### Automated Test Suite

```bash
./tests.sh
```

**Test Coverage:**
- ✅ APIVersions request/response handling
- ✅ DescribeTopicPartitions with unknown topics
- ✅ Correlation ID validation
- ✅ Concurrent connections (5 simultaneous clients)
- ✅ Multiple requests per connection
- ✅ Error handling for unsupported operations
- ✅ Protocol message framing

### Manual Testing

Create a simple Python client:

```python
import socket
import struct

# Connect to server
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.connect(("127.0.0.1", 9092))

# Build APIVersions request
api_key = 18  # APIVersions
api_version = 3
correlation_id = 1
client_id = "test-client"

request_body = bytearray()
request_body.extend(struct.pack(">H", api_key))
request_body.extend(struct.pack(">H", api_version))
request_body.extend(struct.pack(">I", correlation_id))
request_body.extend(struct.pack(">B", len(client_id) + 1))
request_body.extend(client_id.encode('utf-8'))
request_body.extend(struct.pack(">B", 0))  # Tagged fields

# Send request
message_size = len(request_body)
full_request = struct.pack(">I", message_size) + request_body
sock.send(full_request)

# Read response
size_bytes = sock.recv(4)
message_size = struct.unpack(">I", size_bytes)[0]
response = sock.recv(message_size)

print(f"Response: {response.hex()}")
sock.close()
```

## 🏗️ Architecture

### Protocol Implementation

Eventor implements the Kafka binary protocol with proper message framing:

```
Message Format:
┌─────────────┬──────────────┬─────────────┬──────────────┬─────────────┐
│ Message Size│   API Key    │ API Version │ Correlation  │   Payload   │
│   (4 bytes) │  (2 bytes)   │  (2 bytes)  │  ID (4 bytes)│  (variable) │
└─────────────┴──────────────┴─────────────┴──────────────┴─────────────┘
```

### Connection Handling

- **Thread-per-connection** model for handling multiple clients
- **Persistent connections** supporting multiple requests
- **Graceful error handling** with proper connection cleanup
- **Message size validation** to prevent buffer overflows

### Response Generation

- **Correlation ID preservation** for request/response matching
- **Protocol-compliant error codes** (UNKNOWN_TOPIC_OR_PARTITION, UNSUPPORTED_VERSION)
- **Flexible message formats** supporting tagged fields
- **Proper byte ordering** (big-endian network byte order)

## 🔧 Configuration

Currently, the server configuration is compile-time defined:

```rust
const SERVER_ADDRESS: &str = "127.0.0.1:9092";
const MESSAGE_SIZE_LEN: usize = 4;
const API_KEY_LEN: usize = 2;
const API_VERSION_LEN: usize = 2;
const CORRELATION_ID_LEN: usize = 4;
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Apache Kafka** for the protocol specifications
- **Rust Community** for excellent documentation and tooling
- **Confluent** for Kafka protocol documentation

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/yourusername/eventor/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/eventor/discussions)
- **Documentation**: [Project Wiki](https://github.com/yourusername/eventor/wiki)

## 📈 Status

**Current Version**: 0.1.0
**Status**: Alpha - Basic functionality implemented, suitable for development and testing.

---

Built with ❤️ in Rust 🦀