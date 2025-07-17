// This directive tells the Rust compiler to suppress warnings about unused imports
// This is useful during development when we might import modules but not use them yet
// In a production codebase, this should be removed and unused imports cleaned up
#![allow(unused_imports)]

// Import the Write trait from std::io module
// This trait provides the write_all() method that allows us to write bytes to a stream
// We need this to send response data back to clients that connect to our server
use std::io::Write;

// Import TcpListener from std::net module
// TcpListener is the main networking primitive that allows us to listen for incoming TCP connections
// This is essential for creating a server that can accept client connections on a specific port
use std::net::TcpListener;

// The main function is the entry point of our Rust program
// This function will run when the program starts and contains all the server logic
fn main() {

    // Create a new TcpListener bound to the local IP address 127.0.0.1 on port 9092
    // 127.0.0.1 is the localhost/loopback address, meaning only connections from this machine are accepted
    // Port 9092 is the standard port for Apache Kafka brokers
    // .unwrap() will panic and crash the program if binding fails (e.g., port already in use)
    // In production code, this should be handled more gracefully with proper error handling
    let listener = TcpListener::bind("127.0.0.1:9092").unwrap();

    // Start an infinite loop that processes incoming connection attempts
    // listener.incoming() returns an iterator that yields Result<TcpStream, Error> for each connection attempt
    // This iterator will block and wait for new connections, yielding them one by one as they arrive
    for stream in listener.incoming() {
        // Use pattern matching to handle both successful connections and errors
        // This is Rust's idiomatic way of handling Result types that can contain either Ok(value) or Err(error)
        match stream {
            // If the connection attempt was successful, we get an Ok variant containing the TcpStream
            // We declare the stream as 'mut' (mutable) because we need to write data to it
            // The TcpStream represents the bidirectional communication channel with the connected client
            Ok(mut stream) => {
                // Print a message to the console to indicate a new client has connected
                // This is useful for debugging and monitoring server activity
                // In production, this would typically be logged to a file instead of printed to stdout
                println!("accepted new connection");
                
                // Send a response back to the client using the Kafka wire protocol format
                // The byte array [0,0,0,0,0,0,0,7] represents a minimal Kafka response:
                // - First 4 bytes (0,0,0,0): Message size in big-endian format, indicating 0 bytes of payload
                // - Next 4 bytes (0,0,0,7): Correlation ID in big-endian format, value 7
                // This is likely a basic response to a Kafka API request, possibly an API versions request
                // write_all() ensures all bytes are written to the stream or returns an error
                // .unwrap() will panic if the write operation fails (e.g., client disconnects)
                // In production, this should handle write errors gracefully
                stream.write_all(&[0,0,0,0,0,0,0,7]).unwrap();
            }
            // If the connection attempt failed, we get an Err variant containing the error details
            // This could happen due to network issues, resource exhaustion, or other system problems
            Err(e) => {
                // Print the error message to the console for debugging purposes
                // The {} placeholder will be replaced with the error description
                // In production, errors should be logged with appropriate severity levels
                // The server continues running even if individual connection attempts fail
                println!("error: {}", e);
            }
        }
        // After handling each connection (successful or failed), the loop continues
        // to wait for the next incoming connection attempt
        // Note: This is a single-threaded server that handles one connection at a time
        // Each client connection is processed sequentially, which limits scalability
    }
    // This point is never reached because the for loop runs indefinitely
    // The server will continue running until the process is terminated externally
}
