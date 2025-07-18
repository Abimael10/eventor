#![allow(unused_imports)]

use std::io::{self, Write, Read};
use std::net::{TcpListener, TcpStream, Shutdown};
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

    //Extract the API version
    let api_version_offset = API_KEY_LEN; //Skip API key, those are 2 bytes
    let api_version_bytes = &full_request_buffer[
        api_version_offset
        ..
        api_version_offset + API_VERSION_LEN
    ];

    let api_version = u16::from_be_bytes(
        api_version_bytes
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Failed to get 2 bytes for API version."))?
    );

    //Validate API version
    let error_code: u16 = if api_version <= 4 { 0 } else { 35 };

    println!("Extracted Correlation ID (u32): {}", correlation_id);
    println!("Extracted Correlation ID (bytes): {:?}", correlation_id_bytes_slice);

    //Build the response
    //Response format: [4 bytes: message_size][4 bytes: correlation_id_from_request][2 bytes:
    //error_code][4 bytes: api_count][2 bytes: api_key][2 bytes: min_version][2 bytes:
    //max_version][1 bytes: tagged_fields]
    let response_message_size: u32 = 17; // AFTER THE MESSAGE SIZE: correlation ID (4) + error code (2) + api count (4) + api key (2) + api minimal (2) + api max (2) + tagged fields (1) -- So far, after the message size bytes which are 4, we send an extra 17 bytes making a total of 21 bytes for the response.
    let response_message_size_bytes = response_message_size.to_be_bytes();
    let correlation_id_response_bytes = correlation_id.to_be_bytes();
    let error_code_bytes = error_code.to_be_bytes();

    //Handle API versions request
    //The API version requires to have a compact array containing the required API count which is
    //only for now. After looking it up remember the compact array format works like this: [1 API +
    //1] encoded as varint that will leave the digit in u8 type to be equal to 2.
    let api_count_array: u8 = 2;
    let api_key: u16 = 18; //APIversions
    let min_version: u16 = 0; //minimal version since I only work with version 0 through 4
    let max_version: u16 = 4;
    let tagged_fields: u8 = 0; //Empty tagged fields

    let mut response = Vec::with_capacity(MESSAGE_SIZE_LEN + CORRELATION_ID_LEN + 2 /*error code*/ + 1 /*api_count_array*/ + 2 /*api key*/ + 2 /*api min*/ + 2 /*api max*/ + 1 /*tagged fields*/);
    response.extend_from_slice(&response_message_size_bytes);
    response.extend_from_slice(&correlation_id_response_bytes);
    response.extend_from_slice(&error_code_bytes);
    //Addin API versions info to the response
    response.extend_from_slice(&[api_count_array]); //This is already converted so we add it as a
                                                    //borrowed format of array.
    response.extend_from_slice(&api_key.to_be_bytes());
    response.extend_from_slice(&min_version.to_be_bytes());
    response.extend_from_slice(&max_version.to_be_bytes());
    response.extend_from_slice(&[tagged_fields]);

    println!("Sending response: {:?}", response);

    //Send the response back to the client
    stream.write_all(&response)?;

    //Just to pass the fucking test, those idiots set a rule up their fucking asshole, 5:08 AM
    //dealing with this shit
    stream.flush()?;
    println!("Response sent.");

    stream.shutdown(Shutdown::Both)?; // Shutdown both read and write
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
