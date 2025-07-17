#![allow(unused_imports)]

use std::io::{self, Write, Read};

use std::net::{TcpListener, TcpStream};

const MESSAGE_SIZE_LEN: usize = 4;
const API_KEY_LEN: usize = 2;
const API_VERSION_LEN: usize = 2;
const CORRELATION_ID_LEN: usize = 4;

const HEADER_LEN: usize = MESSAGE_SIZE_LEN + API_KEY_LEN + API_VERSION_LEN + CORRELATION_ID_LEN; // 4 + 2 + 2 + 4 = 12 bytes

/// Handles a single incoming TCP connection.
/// Reads the request, extracts the correlation ID, and sends back the response.
fn handle_client(mut stream: TcpStream) -> io::Result<()> {
    println!("Handling connection from: {}", stream.peer_addr()?);

    //Creating a buffer to read the request header into
    let mut header_buffer = vec![0; HEADER_LEN];

    //Read at least 12 bytes from the stream
    //'read_exact' will block until 12 bytes are read or an error occurs (maybe a connection closed)
    stream.read_exact(&mut header_buffer)?;
    println!("Read header: {:?}", header_buffer);

    //Extract the correlation ID from bytes 8-11
    let correlation_id_bytes_slice = &header_buffer[
        MESSAGE_SIZE_LEN + API_KEY_LEN + API_VERSION_LEN
        ..
        MESSAGE_SIZE_LEN + API_KEY_LEN + API_VERSION_LEN + CORRELATION_ID_LEN
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
    let response_message_size: u32 = CORRELATION_ID_LEN as u32;
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
