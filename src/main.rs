#![allow(unused_imports)]

use std::io::{self, Write, Read};
use std::net::{TcpListener, TcpStream};
use std::convert::TryInto; //To use try_into() on slices

const MESSAGE_SIZE_LEN: usize = 4;
const API_KEY_LEN: usize = 2;
const API_VERSION_LEN: usize = 2;
const CORRELATION_ID_LEN: usize = 4;

const HEADER_LEN: usize = MESSAGE_SIZE_LEN + API_KEY_LEN + API_VERSION_LEN + CORRELATION_ID_LEN; // 4 + 2 + 2 + 4 = 12 bytes

/// Handles a single incoming TCP connection.
/// Reads the request, extracts the correlation ID, and sends back the response.
fn handle_client(mut stream: TcpStream) -> io::Result<()> {
    println!("Handling connection from: {}", stream.peer_addr()?);

    //Initial buffer to read just the message_size
    let mut initial_bytes = vec![0; MESSAGE_SIZE_LEN];

    stream.read_exact(&mut initial_bytes)?;
    let total_message_size = u32::from_be_bytes(
        initial_bytes.as_slice()
        .try_into()
        .unwrap_or_else(|_| [0; 4])
    );
    println!("Total message size indicated: {} bytes", total_message_size);

    // Basic validation: ensure the message is at least as long as the header
    if total_message_size < HEADER_LEN as u32 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Message size {} is smaller than minimum header size {}", total_message_size, HEADER_LEN)
        ));
    }

    //Now, read the rest of the message (api_key, api_version, correlation_id, and if any a body)
    //I already read the MESSAGE_SIZE_LEN so it will be `total_message_size - MESSAGE_SIZE_LEN`
    //more bytes
    let remaining_bytes = total_message_size as usize - MESSAGE_SIZE_LEN;
    let mut full_request_buffer = vec![0; remaining_bytes];
    stream.read_exact(&mut full_request_buffer)?;

    let correlation_id_offset_in_remaining = API_KEY_LEN + API_VERSION_LEN;

    // Verify bounds before slicing to prevent panic
    if remaining_bytes < correlation_id_offset_in_remaining + CORRELATION_ID_LEN {
         return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Received message too short to contain full correlation ID"
        ));
    }

    //Extract the correlation ID from bytes 8-11
    let correlation_id_bytes_slice = &full_request_buffer[
        correlation_id_offset_in_remaining
        ..
        correlation_id_offset_in_remaining + CORRELATION_ID_LEN
    ];

    let correlation_id = u32::from_be_bytes(
        correlation_id_bytes_slice
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Failed to get 4 bytes for correlation ID."))?
    );

    println!("Extracted Correlation ID (u32): {}", correlation_id);
    println!("Extracted Correlation ID (bytes): {:?}", correlation_id_bytes_slice);

    //Build the response
    //Response format: [4 bytes: message_size][4 bytes: correlation_id_from_request]
    let response_message_size: u32 = 0;
    let response_message_size_bytes = response_message_size.to_be_bytes();
    let correlation_id_response_bytes = correlation_id.to_be_bytes();

    let mut response = Vec::with_capacity(MESSAGE_SIZE_LEN + CORRELATION_ID_LEN);
    response.extend_from_slice(&response_message_size_bytes);
    response.extend_from_slice(&correlation_id_response_bytes);

    println!("Sending response: {:?}", response);

    //Send the response back to the client
    stream.write_all(&response)?;
    println!("Response sent.");

    Ok(())
}

fn main() -> io::Result<()> {

    let listener = TcpListener::bind("127.0.0.1:9092").unwrap();
    println!("Server listening on: {}", listener.local_addr()?);
    
    //Accept incoming connections
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                //spawn a new thread to handle each connection
                //This allows the server to handle multiple clients concurrently
                std::thread::spawn(move || {
                    if let Err(e) = handle_client(stream) {
                        eprintln!("Error handling client: {}", e);
                    }
                });
            }

            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
    Ok(())
}
