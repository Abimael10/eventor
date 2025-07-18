#![allow(unused_imports)]

use std::io::{self, Write, Read};
use std::net::{TcpListener, TcpStream, Shutdown};
use std::convert::TryInto; //To use try_into() on slices

const MESSAGE_SIZE_LEN: usize = 4;
const API_KEY_LEN: usize = 2;
const API_VERSION_LEN: usize = 2;
const CORRELATION_ID_LEN: usize = 4;

const HEADER_LEN: usize = MESSAGE_SIZE_LEN + API_KEY_LEN + API_VERSION_LEN + CORRELATION_ID_LEN; // 4 + 2 + 2 + 4 = 12 bytes

/// Parses topic name from DescribeTopicPartitions request
fn parse_topic_name(request_buffer: &[u8]) -> String {
    // After api_key(2) + api_version(2) + correlation_id(4) = 8 bytes
    // Then we have: topic_count(1) + topic_name_length(varint) + topic_name + other_fields
    let header_offset = API_KEY_LEN + API_VERSION_LEN + CORRELATION_ID_LEN; // 8 bytes
    
    if request_buffer.len() < header_offset + 2 {
        return String::new();
    }
    
    let topic_count_offset = header_offset;
    let topic_name_len_offset = topic_count_offset + 1; // Skip compact array count
    
    if request_buffer.len() <= topic_name_len_offset {
        return String::new();
    }
    
    // Compact string encoding: length + 1, then string bytes
    let topic_name_len = request_buffer[topic_name_len_offset] as usize;
    if topic_name_len == 0 {
        return String::new();
    }
    
    let actual_topic_name_len = topic_name_len - 1; // Compact string: stored_len - 1 = actual_len
    let topic_name_start = topic_name_len_offset + 1;
    let topic_name_end = topic_name_start + actual_topic_name_len;
    
    if request_buffer.len() < topic_name_end {
        return String::new();
    }
    
    String::from_utf8_lossy(&request_buffer[topic_name_start..topic_name_end]).to_string()
}

/// Builds DescribeTopicPartitions response for unknown topic
fn build_describe_topic_partitions_response(correlation_id: u32, topic_name: &str) -> Vec<u8> {
    let mut response = Vec::new();
    
    // Response structure according to Kafka protocol v0:
    // [message_size][correlation_id][throttle_time_ms][topics][next_cursor][tagged_fields]
    // 
    // Topic structure:
    // [error_code][name][topic_id][is_internal][partitions][topic_authorized_operations][tagged_fields]
    
    let correlation_id_bytes = correlation_id.to_be_bytes();
    let throttle_time_ms: u32 = 0;
    let topic_count: u8 = 2; // compact array: 1 topic + 1 = 2
    let topic_name_len: u8 = (topic_name.len() + 1) as u8; // compact string: len + 1
    let topic_id = [0u8; 16]; // 16 zero bytes for null UUID
    let error_code: u16 = 3; // UNKNOWN_TOPIC_OR_PARTITION
    let is_internal: u8 = 0; // false
    let partitions_count: u8 = 1; // compact array: 0 partitions + 1 = 1
    let topic_authorized_operations: u32 = 0; // No operations
    let topic_tagged_fields: u8 = 0;
    let next_cursor: u8 = 0; // null next_cursor (encoded as 0 for nullable)
    let response_tagged_fields: u8 = 0;
    
    // Calculate message size: everything after the message_size field
    // Response Header v1: correlation_id(4) + header_tag_buffer(1) + throttle_time(4) + topic_count(1) + [topic: error_code(2) + topic_name_len(1) + topic_name + topic_id(16) + is_internal(1) + partitions(1) + topic_authorized_operations(4) + topic_tagged_fields(1)] + next_cursor(1) + response_tagged_fields(1)
    let message_size = 4 + 1 + 4 + 1 + (2 + 1 + topic_name.len() + 16 + 1 + 1 + 4 + 1) + 1 + 1;
    let header_tag_buffer: u8 = 0; // TAG_BUFFER for Response Header v1
    
    // Build response
    response.extend_from_slice(&(message_size as u32).to_be_bytes());
    response.extend_from_slice(&correlation_id_bytes);
    response.extend_from_slice(&[header_tag_buffer]); // Response Header v1 TAG_BUFFER
    response.extend_from_slice(&throttle_time_ms.to_be_bytes());
    response.extend_from_slice(&[topic_count]);
    
    // Topic data
    response.extend_from_slice(&error_code.to_be_bytes());
    response.extend_from_slice(&[topic_name_len]);
    response.extend_from_slice(topic_name.as_bytes());
    response.extend_from_slice(&topic_id);
    response.extend_from_slice(&[is_internal]);
    response.extend_from_slice(&[partitions_count]);
    response.extend_from_slice(&topic_authorized_operations.to_be_bytes());
    response.extend_from_slice(&[topic_tagged_fields]);
    
    // Next cursor (null) and response tagged fields
    response.extend_from_slice(&[next_cursor]);
    response.extend_from_slice(&[response_tagged_fields]);
    
    response
}

/// Builds APIVersions response
fn build_api_versions_response(correlation_id: u32, api_version: u16) -> Vec<u8> {
    let error_code: u16 = if api_version <= 4 { 0 } else { 35 };
    
    let response_message_size: u32 = 26;
    let response_message_size_bytes = response_message_size.to_be_bytes();
    let correlation_id_response_bytes = correlation_id.to_be_bytes();
    let error_code_bytes = error_code.to_be_bytes();

    let api_count_array: u8 = 3; // 2 APIs + 1 = 3
    
    // First API: APIVersions
    let api_key_1: u16 = 18;
    let min_version_1: u16 = 0;
    let max_version_1: u16 = 4;
    let api_tagged_fields_1: u8 = 0;
    
    // Second API: DescribeTopicPartitions
    let api_key_2: u16 = 75;
    let min_version_2: u16 = 0;
    let max_version_2: u16 = 0;
    let api_tagged_fields_2: u8 = 0;
    
    let throttle_time_ms: u32 = 0;
    let response_tagged_fields: u8 = 0;

    let mut response = Vec::new();
    response.extend_from_slice(&response_message_size_bytes);
    response.extend_from_slice(&correlation_id_response_bytes);
    response.extend_from_slice(&error_code_bytes);
    response.extend_from_slice(&[api_count_array]);
    
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
    
    response
}

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

        //Extract the API key
        let api_key_bytes = &full_request_buffer[0..API_KEY_LEN];
        let api_key = match api_key_bytes.try_into() {
            Ok(bytes) => u16::from_be_bytes(bytes),
            Err(_) => {
                println!("Failed to get 2 bytes for API key, breaking connection");
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

        println!("Extracted Correlation ID (u32): {}", correlation_id);
        println!("Extracted Correlation ID (bytes): {:?}", correlation_id_bytes_slice);
        println!("Extracted API Key: {}", api_key);
        println!("Extracted API Version: {}", api_version);

        // Build response based on API key
        let response = if api_key == 18 {
            // APIVersions request
            println!("Handling APIVersions request");
            build_api_versions_response(correlation_id, api_version)
        } else if api_key == 75 {
            // DescribeTopicPartitions request
            println!("Handling DescribeTopicPartitions request");
            let topic_name = parse_topic_name(&full_request_buffer);
            println!("Parsed topic name: '{}'", topic_name);
            build_describe_topic_partitions_response(correlation_id, &topic_name)
        } else {
            // Unknown API key - return error
            println!("Unknown API key: {}", api_key);
            vec![0, 0, 0, 8, 
                 (correlation_id >> 24) as u8, (correlation_id >> 16) as u8, (correlation_id >> 8) as u8, correlation_id as u8,
                 0, 35, 0, 0] // Error code 35 = UNSUPPORTED_VERSION
        };

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
