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

    // register with server, obtain UUID
    let _ = match client_0.register_with_server().await {
        Ok(valid_uuid) => println!("your uuid is: {}", valid_uuid),
        Err(err) => {
            let err_msg = format!("Error getting UUID from server. {}", err);
            panic!("{}", err_msg);
        }
    };

    let mut client_1 = client_0.clone();
    let mut client_2 = client_0.clone();

    let handles = vec![
        tokio::spawn(async move {
            client_0.incoming_traff_loop().await;
        }),
        tokio::spawn(async move {
            client_1.display_loop().await;
        }),
        tokio::spawn(async move {
            client_2.outgoing_traff_loop().await;
        }),
    ];

    for handle in handles {
        handle.await.unwrap();
    }
    // so that we wait for execution (loops forever)
    // handle.await.unwrap();
}

/* ===== SOME TEST CODE ======================================================*/

// used for testing purposes *ONLY*. Don't use for anything else.
/*
impl Client {
    /// adds a peer to the peer map
    pub async fn __add_peer(&mut self, peer_uuid: Uuid, peer_addr: SocketAddr) {
        let mut locked_queue = self.peer_map.lock().await;
        locked_queue.insert(peer_uuid, peer_addr);
    }

    /// this is just for prototyping purposes. don't use ever again afterwards.
    pub async fn __basic_send(&mut self, peer_uuid: &Uuid) {
        let out_socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
    
        let msg = format!("Hello");
        let mut msg_json = json::JsonValue::new_object();
        msg_json["src_uuid"] = json::JsonValue::from(
            "75442486-0878-440c-9db1-a7006c25a39f".to_string());
        msg_json["dst_uuid"] = json::JsonValue::from(
            "75442486-0878-440c-9db1-a7006c25a39f".to_string());
        msg_json["creation_time"] = json::JsonValue::from(0.to_string());
        msg_json["data"] = json::JsonValue::from(msg);
        
        let map_lock = self.peer_map.lock().await;
        let addr = map_lock.get(peer_uuid).unwrap();
        match out_socket.send_to(msg_json.dump().as_bytes(), addr).await {
            Ok(_) => println!("Succeeded..."),
            Err(_) => println!("Failed..."),
        }
    }
}
*/

