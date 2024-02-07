/*
 * File: main.rs
 * Author: Ethan Graham
 * Date: 07 Feb. 2024
 *
 * Description: Main entrypoint for protocol run by client
 */
pub mod client;

use client::Client;
use tokio;
use std::env;

/// calls the Client functions/methods
#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    assert_eq!(args.len(), 2, "Try: ./client_protocol <port_number>");
    
    let port: u16 = args[1].parse()
        .expect("Please enter a valid <port_number>");

    let _client_obj = match Client::build(port).await {
        Ok(obj) => obj,
        Err(_) => return
    };
}

