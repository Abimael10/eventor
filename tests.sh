#!/bin/bash

# Automated Test Runner for Eventor Server
# Usage: ./run_tests.sh

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
SERVER_HOST="127.0.0.1"
SERVER_PORT="9092"
RUST_PROJECT_DIR="."
TEST_SCRIPT="test_eventor_server.py"
SERVER_LOG="server.log"
SERVER_PID_FILE="server.pid"

print_banner() {
    echo -e "${CYAN}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                    🚀 EVENTOR SERVER TEST SUITE              ║"
    echo "║                     Automated Test Runner                    ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

print_step() {
    echo -e "${BLUE}➤ $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

check_dependencies() {
    print_step "Checking dependencies..."
    
    # Check if cargo is installed
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo (Rust) is not installed. Please install Rust first."
        exit 1
    fi
    
    # Check if python3 is installed
    if ! command -v python3 &> /dev/null; then
        print_error "Python3 is not installed. Please install Python3 first."
        exit 1
    fi
    
    # Check if Cargo.toml exists
    if [[ ! -f "Cargo.toml" ]]; then
        print_error "Cargo.toml not found. Please run this script from the Rust project directory."
        exit 1
    fi
    
    print_success "All dependencies are available"
}

cleanup_server() {
    if [[ -f "$SERVER_PID_FILE" ]]; then
        SERVER_PID=$(cat "$SERVER_PID_FILE")
        if ps -p $SERVER_PID > /dev/null 2>&1; then
            print_step "Stopping server (PID: $SERVER_PID)..."
            kill $SERVER_PID
            sleep 2
            
            # Force kill if still running
            if ps -p $SERVER_PID > /dev/null 2>&1; then
                print_warning "Force killing server..."
                kill -9 $SERVER_PID
            fi
        fi
        rm -f "$SERVER_PID_FILE"
    fi
    
    # Also kill any process using the port
    if command -v lsof &> /dev/null; then
        local port_pid=$(lsof -ti:$SERVER_PORT 2>/dev/null || true)
        if [[ -n "$port_pid" ]]; then
            print_step "Killing process using port $SERVER_PORT (PID: $port_pid)..."
            kill -9 $port_pid 2>/dev/null || true
        fi
    fi
}

build_server() {
    print_step "Building Rust server..."
    
    if cargo build --release; then
        print_success "Server built successfully"
    else
        print_error "Failed to build server"
        exit 1
    fi
}

start_server() {
    print_step "Starting Eventor server on $SERVER_HOST:$SERVER_PORT..."
    
    # Start server in background and capture PID
    cargo run --release > "$SERVER_LOG" 2>&1 &
    SERVER_PID=$!
    echo $SERVER_PID > "$SERVER_PID_FILE"
    
    print_step "Server started with PID: $SERVER_PID"
    print_step "Waiting for server to be ready..."
    
    # Wait for server to be ready (max 10 seconds)
    local count=0
    while [[ $count -lt 20 ]]; do
        if nc -z $SERVER_HOST $SERVER_PORT 2>/dev/null; then
            print_success "Server is ready and accepting connections"
            return 0
        fi
        sleep 0.5
        count=$((count + 1))
        echo -n "."
    done
    
    echo ""
    print_error "Server failed to start or is not accepting connections"
    print_error "Server log:"
    cat "$SERVER_LOG"
    cleanup_server
    exit 1
}

create_test_script() {
    if [[ ! -f "$TEST_SCRIPT" ]]; then
        print_step "Creating test script..."
        
        cat > "$TEST_SCRIPT" << 'EOF'
#!/usr/bin/env python3
"""
Quick test script for the Eventor server implementation.
"""

import socket
import struct
import time
import threading

class EventorTestClient:
    def __init__(self, host="127.0.0.1", port=9092):
        self.host = host
        self.port = port
        self.correlation_id_counter = 1
    
    def connect(self):
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect((self.host, self.port))
        return sock
    
    def get_next_correlation_id(self):
        correlation_id = self.correlation_id_counter
        self.correlation_id_counter += 1
        return correlation_id
    
    def build_api_versions_request(self, api_version=3):
        correlation_id = self.get_next_correlation_id()
        api_key = 18  # APIVersions
        client_id = "test-client"
        
        request_body = bytearray()
        request_body.extend(struct.pack(">H", api_key))
        request_body.extend(struct.pack(">H", api_version))
        request_body.extend(struct.pack(">I", correlation_id))
        request_body.extend(struct.pack(">B", len(client_id) + 1))
        request_body.extend(client_id.encode('utf-8'))
        request_body.extend(struct.pack(">B", 0))  # Tagged fields
        
        message_size = len(request_body)
        full_request = struct.pack(">I", message_size) + request_body
        return full_request, correlation_id
    
    def build_describe_topic_partitions_request(self, topic_name="test-topic"):
        correlation_id = self.get_next_correlation_id()
        api_key = 75
        api_version = 0
        
        request_body = bytearray()
        request_body.extend(struct.pack(">H", api_key))
        request_body.extend(struct.pack(">H", api_version))
        request_body.extend(struct.pack(">I", correlation_id))
        
        client_id = "test-client"
        request_body.extend(struct.pack(">H", len(client_id)))
        request_body.extend(client_id.encode('utf-8'))
        
        request_body.extend(struct.pack(">B", 2))  # 1 topic + 1
        request_body.extend(struct.pack(">B", len(topic_name) + 1))
        request_body.extend(topic_name.encode('utf-8'))
        request_body.extend(struct.pack(">B", 0))  # Tagged fields
        request_body.extend(struct.pack(">B", 0))  # Null cursor
        request_body.extend(struct.pack(">B", 0))  # Tagged fields
        
        message_size = len(request_body)
        full_request = struct.pack(">I", message_size) + request_body
        return full_request, correlation_id
    
    def send_request_and_get_response(self, request):
        sock = self.connect()
        try:
            sock.send(request)
            size_bytes = sock.recv(4)
            if len(size_bytes) != 4:
                raise Exception("Failed to read message size")
            
            message_size = struct.unpack(">I", size_bytes)[0]
            response_body = b""
            while len(response_body) < message_size:
                chunk = sock.recv(message_size - len(response_body))
                if not chunk:
                    raise Exception("Connection closed unexpectedly")
                response_body += chunk
            
            return size_bytes + response_body
        finally:
            sock.close()

def test_api_versions():
    print("🧪 Testing APIVersions...")
    client = EventorTestClient()
    request, expected_correlation_id = client.build_api_versions_request(api_version=3)
    response = client.send_request_and_get_response(request)
    
    # Parse correlation ID from response
    correlation_id = struct.unpack(">I", response[4:8])[0]
    error_code = struct.unpack(">H", response[8:10])[0]
    
    assert correlation_id == expected_correlation_id, f"Correlation ID mismatch"
    assert error_code == 0, f"Expected error code 0, got {error_code}"
    print("✅ APIVersions test passed")

def test_describe_topic_partitions():
    print("🧪 Testing DescribeTopicPartitions...")
    client = EventorTestClient()
    request, expected_correlation_id = client.build_describe_topic_partitions_request("unknown-topic")
    response = client.send_request_and_get_response(request)
    
    # Parse correlation ID from response
    correlation_id = struct.unpack(">I", response[4:8])[0]
    assert correlation_id == expected_correlation_id, f"Correlation ID mismatch"
    print("✅ DescribeTopicPartitions test passed")

def test_concurrent_connections():
    print("🧪 Testing concurrent connections...")
    
    def worker(worker_id, results):
        try:
            client = EventorTestClient()
            request, corr_id = client.build_api_versions_request()
            response = client.send_request_and_get_response(request)
            correlation_id = struct.unpack(">I", response[4:8])[0]
            results.append(correlation_id == corr_id)
        except Exception as e:
            results.append(False)
    
    results = []
    threads = []
    
    for i in range(5):
        thread = threading.Thread(target=worker, args=(i, results))
        threads.append(thread)
        thread.start()
    
    for thread in threads:
        thread.join()
    
    successful = sum(results)
    print(f"✅ Concurrent test: {successful}/5 connections successful")
    assert successful == 5, f"Not all concurrent connections succeeded"

def test_multiple_requests():
    print("🧪 Testing multiple requests on same connection...")
    
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(("127.0.0.1", 9092))
    
    try:
        client = EventorTestClient()
        
        # First request
        request1, corr_id1 = client.build_api_versions_request(api_version=3)
        sock.send(request1)
        size_bytes = sock.recv(4)
        message_size = struct.unpack(">I", size_bytes)[0]
        response1_body = sock.recv(message_size)
        response1 = size_bytes + response1_body
        correlation_id1 = struct.unpack(">I", response1[4:8])[0]
        
        # Second request
        request2, corr_id2 = client.build_describe_topic_partitions_request("test-topic")
        sock.send(request2)
        size_bytes = sock.recv(4)
        message_size = struct.unpack(">I", size_bytes)[0]
        response2_body = sock.recv(message_size)
        response2 = size_bytes + response2_body
        correlation_id2 = struct.unpack(">I", response2[4:8])[0]
        
        assert correlation_id1 == corr_id1 and correlation_id2 == corr_id2
        print("✅ Multiple requests test passed")
        
    finally:
        sock.close()

def main():
    print("🚀 Running Eventor Server Tests")
    print("=" * 50)
    
    try:
        test_api_versions()
        test_describe_topic_partitions()
        test_concurrent_connections()
        test_multiple_requests()
        
        print("\n🎉 All tests passed!")
        print("📊 Test Summary:")
        print("  ✅ APIVersions request handling")
        print("  ✅ DescribeTopicPartitions request handling")
        print("  ✅ Concurrent connections (5 clients)")
        print("  ✅ Multiple requests per connection")
        print("  ✅ Correlation ID handling")
        
    except Exception as e:
        print(f"\n❌ Test failed: {e}")
        import traceback
        traceback.print_exc()
        exit(1)

if __name__ == "__main__":
    main()
EOF
        
        chmod +x "$TEST_SCRIPT"
        print_success "Test script created"
    fi
}

run_tests() {
    print_step "Running test suite..."
    echo ""
    
    if python3 "$TEST_SCRIPT"; then
        print_success "All tests passed! 🎉"
        return 0
    else
        print_error "Some tests failed"
        return 1
    fi
}

show_server_logs() {
    if [[ -f "$SERVER_LOG" ]]; then
        echo ""
        print_step "Server logs:"
        echo -e "${PURPLE}"
        tail -20 "$SERVER_LOG"
        echo -e "${NC}"
    fi
}

main() {
    print_banner
    
    # Set up cleanup trap
    trap cleanup_server EXIT
    
    check_dependencies
    cleanup_server  # Clean up any existing server
    create_test_script
    build_server
    start_server
    
    echo ""
    
    if run_tests; then
        echo ""
        print_success "🎊 Test suite completed successfully!"
        echo -e "${GREEN}"
        echo "╔══════════════════════════════════════════════════════════════╗"
        echo "║  Your Eventor server implementation is working correctly! 🚀 ║"
        echo "╚══════════════════════════════════════════════════════════════╝"
        echo -e "${NC}"
    else
        echo ""
        print_error "Test suite failed"
        show_server_logs
        exit 1
    fi
}

# Handle script arguments
case "${1:-}" in
    "clean")
        print_step "Cleaning up..."
        cleanup_server
        rm -f "$SERVER_LOG" "$TEST_SCRIPT"
        print_success "Cleanup completed"
        exit 0
        ;;
    "logs")
        show_server_logs
        exit 0
        ;;
    "help"|"-h"|"--help")
        echo "Usage: $0 [clean|logs|help]"
        echo ""
        echo "Commands:"
        echo "  (no args)  Run the full test suite"
        echo "  clean      Clean up server processes and temporary files"
        echo "  logs       Show recent server logs"
        echo "  help       Show this help message"
        exit 0
        ;;
    "")
        main
        ;;
    *)
        print_error "Unknown command: $1"
        echo "Use '$0 help' for usage information"
        exit 1
        ;;
esac
EOF