/*
 * File: main.rs
 * Author: Ethan Graham
 * Date: 07 Feb. 2024
 *
 * Description: Main entrypoint for protocol run by client
 */
pub mod client;
pub mod message;

use client::Client;
use tokio;
use std::env;
use std::net::SocketAddr;
use uuid::Uuid;
use std::sync::Arc;

/// calls the Client functions/methods
#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    assert_eq!(args.len(), 2, "Try: ./client_protocol <port_number>");
    
    let port: u16 = args[1].parse()
        .expect("Please enter a valid <port_number>");

    // clones the atomic reference counters, not the data. Underlying data is 
    // shared across threads.
    let mut client_0 = Client::build(port).await.unwrap();
    let mut client_1 = client_0.clone();
    let mut client_2 = client_0.clone();

    let handles = vec![
        tokio::spawn(async move {
            client_0.incoming_traff_loop().await;
        }),
        tokio::spawn(async move {
            client_1.display_loop().await;
        }),
    ];

    for handle in handles {
        handle.await.unwrap();
    }
    // so that we wait for execution (loops forever)
    // handle.await.unwrap();
}

