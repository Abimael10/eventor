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

    loop {
        //Initial buffer to read just the message_size
        let mut initial_bytes = vec![0; MESSAGE_SIZE_LEN];

        let total_message_size = match stream.read_exact(&mut initial_bytes) {
            Ok(()) => {
                u32::from_be_bytes(
                    initial_bytes.as_slice()
                        .try_into()
                        .unwrap_or_else(|_| [0; 4])
                )
            }
            Err(e) => {
                println!("Client disconnected: {}", e);
                break;
            }
        };
        println!("Total message size indicated: {} bytes", total_message_size);

        // Basic validation: ensure the message is at least as long as the header (minus the 4 bytes we already read)
        if total_message_size < (HEADER_LEN - MESSAGE_SIZE_LEN) as u32 {
            println!("Invalid message size {} < {}, breaking connection", total_message_size, HEADER_LEN - MESSAGE_SIZE_LEN);
            break;
        }

        //Now, read the rest of the message (api_key, api_version, correlation_id, and if any a body)
        //The total_message_size includes everything after the initial 4 bytes, so we read exactly that amount
        let remaining_bytes = total_message_size as usize;
        let mut full_request_buffer = vec![0; remaining_bytes];
        
        if let Err(e) = stream.read_exact(&mut full_request_buffer) {
            println!("Error reading request body: {}", e);
            break;
        }

        let correlation_id_offset_in_remaining = API_KEY_LEN + API_VERSION_LEN;

        // Verify bounds before slicing to prevent panic
        if remaining_bytes < correlation_id_offset_in_remaining + CORRELATION_ID_LEN {
            println!("Message too short to contain correlation ID, breaking connection");
            break;
        }

        //Extract the correlation ID from bytes 8-11
        let correlation_id_bytes_slice = &full_request_buffer[
            correlation_id_offset_in_remaining
                ..
                correlation_id_offset_in_remaining + CORRELATION_ID_LEN
        ];

        let correlation_id = match correlation_id_bytes_slice.try_into() {
            Ok(bytes) => u32::from_be_bytes(bytes),
            Err(_) => {
                println!("Failed to get 4 bytes for correlation ID, breaking connection");
                break;
            }
        };

        //Extract the API version
        let api_version_offset = API_KEY_LEN; //Skip API key, those are 2 bytes
        let api_version_bytes = &full_request_buffer[
            api_version_offset
                ..
                api_version_offset + API_VERSION_LEN
        ];

        let api_version = match api_version_bytes.try_into() {
            Ok(bytes) => u16::from_be_bytes(bytes),
            Err(_) => {
                println!("Failed to get 2 bytes for API version, breaking connection");
                break;
            }
        };

        //Validate API version
        let error_code: u16 = if api_version <= 4 { 0 } else { 35 };

        println!("Extracted Correlation ID (u32): {}", correlation_id);
        println!("Extracted Correlation ID (bytes): {:?}", correlation_id_bytes_slice);

        //Build the response
        //Response format: [4 bytes: message_size][4 bytes: correlation_id_from_request][2 bytes:
        //error_code][4 bytes: api_count][2 bytes: api_key][2 bytes: min_version][2 bytes:
        //max_version][1 bytes: tagged_fields] ---- Deprecated this format but will keep in case I
        //need something more simple later on ----
        // With 2 APIs: correlation_id(4) + error_code(2) + api_count(1) + api1(7) + api2(7) + throttle_time(4) + tagged_fields(1) = 26 bytes
        let response_message_size: u32 = 26;
        let response_message_size_bytes = response_message_size.to_be_bytes();
        let correlation_id_response_bytes = correlation_id.to_be_bytes();
        let error_code_bytes = error_code.to_be_bytes();

        //Handle API versions request
        //The API version requires to have a compact array containing the required API count which is
        //2 APIs now. Compact array format works like this: [actual count + 1] encoded as varint
        //For 2 APIs: 2 + 1 = 3, encoded as varint 0x03
        let api_count_array: u8 = 3;
        
        // First API: APIVersions
        let api_key_1: u16 = 18; //APIversions
        let min_version_1: u16 = 0; //minimal version since I only work with version 0 through 4
        let max_version_1: u16 = 4;
        let api_tagged_fields_1: u8 = 0;
        
        // Second API: DescribeTopicPartitions ..
        let api_key_2: u16 = 75; //DescribeTopicPartitions
        let min_version_2: u16 = 0;
        let max_version_2: u16 = 0;
        let api_tagged_fields_2: u8 = 0;
        
        let throttle_time_ms: u32 = 0;
        let response_tagged_fields: u8 = 0;

        let mut response = Vec::new();
        response.extend_from_slice(&response_message_size_bytes);
        response.extend_from_slice(&correlation_id_response_bytes);
        response.extend_from_slice(&error_code_bytes);
        //Adding API versions info to the response
        response.extend_from_slice(&[api_count_array]); //This is already converted so we add it as a
                                                        //borrowed format of array.
        
        // First API: APIVersions
        response.extend_from_slice(&api_key_1.to_be_bytes());
        response.extend_from_slice(&min_version_1.to_be_bytes());
        response.extend_from_slice(&max_version_1.to_be_bytes());
        response.extend_from_slice(&[api_tagged_fields_1]);
        
        // Second API: DescribeTopicPartitions
        response.extend_from_slice(&api_key_2.to_be_bytes());
        response.extend_from_slice(&min_version_2.to_be_bytes());
        response.extend_from_slice(&max_version_2.to_be_bytes());
        response.extend_from_slice(&[api_tagged_fields_2]);
        
        response.extend_from_slice(&throttle_time_ms.to_be_bytes());
        response.extend_from_slice(&[response_tagged_fields]);

        println!("Sending response: {:?}", response);

        //Send the response back to the client
        if let Err(e) = stream.write_all(&response) {
            println!("Error writing response: {}", e);
            break;
        }

        //Just to pass the fucking test, those idiots set a rule up their fucking asshole, 5:08 AM
        //dealing with this shit
        if let Err(e) = stream.flush() {
            println!("Error flushing stream: {}", e);
            break;
        }
        println!("Response sent.");

        //stream.shutdown(Shutdown::Both)?; // Shutdown both read and write, commented out since now we
        //will handle multiple requests in the client.
    }
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
