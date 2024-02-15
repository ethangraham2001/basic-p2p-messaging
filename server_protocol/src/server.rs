/*
 * File: server_node.rs
 * Author: Ethan Graham
 * Date: 06 Feb. 2024
 *
 * Description: implementation of central server protocol
 */

use std::net::{UdpSocket, SocketAddr};
use std::collections::HashMap;
use json;
use uuid::Uuid;

/// a peer in the network
pub struct PeerNode {
    pub id: String,
    pub addr: SocketAddr,
}

/// server node that serves IP requests
pub struct ServerNode {
    pub listening_socket: UdpSocket,        // socket that the server listens on
    peers: HashMap<String, PeerNode>,   // map of peers
}

/// minimum port number that server listens on
static MIN_PORT_NUMBER: u16 = 50_000;

/// implementations for ServerNode
impl ServerNode {
    /// inits a ServerNode and returns it
    /// `port`: the port number that the server will listen on
    pub fn build(port: u16) -> Result<ServerNode, NodeError> {
        if port < MIN_PORT_NUMBER {
            let err_msg = String::from("Select higher port number");
            return Err(NodeError::NodeCreationError(err_msg));
        }

        let listen_addr = SocketAddr::from(([127, 0, 0, 1], port));
        let socket = match UdpSocket::bind(listen_addr) {
            Ok(udp_sock) => udp_sock,
            Err(_) => {
                let err_msg = String::from("Error binding socket");
                return Err(NodeError::NodeCreationError(err_msg));
            }
        };

        Ok( ServerNode{ listening_socket: socket, peers: HashMap::new() } )
    }

    /// adds a peer into the index
    pub fn add_peer(&mut self, peer: PeerNode) {
        // this introduces overhead - maybe should just store UUID <-> IP map
        // .clone() may be inefficient
        self.peers.insert(peer.id.clone(), peer);
    }

    /// looks a PeerNode up based on ID
    pub fn lookup_id(&self, id: &str) -> Option<&PeerNode> {
        self.peers.get(id)
    }

    pub fn handle_request(&mut self, recv_bytes: &[u8], src_addr: SocketAddr) 
        -> Result<(), ()> {

        let recv_string = String::from_utf8(recv_bytes.to_vec()).unwrap();

        println!("===== \x1b[36mrequest from: {:?}\x1b[0m =====", src_addr);
        println!("\tRequest Size: {}B", recv_string.len());
        let json_req = match json::parse(&recv_string) {
            Ok(valid_json) => valid_json,
            Err(_) => {
                println!("\t\x1b[31mInvalid JSON\x1b[0m");
                return Err(())
            }
        };

        let req_type = &json_req["req_type"];
        println!("\thandling {} request. src = {}", req_type.to_string(), 
                 src_addr);
        if req_type.to_string() == "registration" {
            return self.handle_registration(src_addr, &json_req);
        } 
        else if req_type.to_string() == "query" {
            return self.handle_lookup(json_req, src_addr);
        }

        Ok(())
    }

    /// handles a lookup request, and sends a response
    ///
    /// `bytes`: the received request
    /// `src_addr`: the requester's IP
    pub fn handle_lookup(&self, json_req: json::JsonValue, 
                         src_addr: SocketAddr) -> Result<(), ()> {
        let queried_uuid = json_req["queried_uuid"].as_str();
        let response = match queried_uuid {
            Some(uuid) => {
                let mut data = json::JsonValue::new_object();
                data["address"] = match self.lookup_id(uuid) {
                    Some(val) => json::from(val.addr.to_string()),
                    None => json::from("nil")
                };
                data["uuid"] = json::from(uuid);
                data
            },
            None => {
                let mut data = json::JsonValue::new_object();
                data["address"] = json::from("nil");
                data["id"] = json::from("invalid_uuid");
                data
            }
        };
        // send response
        self.listening_socket.send_to(response.dump().as_bytes(), src_addr)
            .unwrap();
        Ok(())
    }

    /// handles a client registering with the server. Adds its address to peer
    /// Map
    ///
    /// `json_req`: a json request
    /// `src_addr`: the requesting addr
    pub fn handle_registration(&mut self, src_addr: SocketAddr, 
                               req: &json::JsonValue) -> Result<(), ()> {
        // init a new peer and insert it
        let new_uuid = Uuid::new_v4().to_string();

        // assumes that the client sends a valid address.
        let addr = req["addr"].to_string().parse::<SocketAddr>().unwrap();

        // I want to avoid the new_uuid.clone() here if possible
        let new_peer = PeerNode { addr, id: new_uuid.clone() }; 

        println!("peer added. UUID = {}, ADDR = {}", new_peer.id, 
                 new_peer.addr);
        self.add_peer(new_peer);


        let mut response = json::JsonValue::new_object();
        response["status"] = json::JsonValue::from("OK");
        response["uuid"] = json::JsonValue::from(new_uuid);

        // send response
        self.listening_socket.send_to(response.dump().as_bytes(), src_addr)
            .unwrap();
        Ok(())
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct T(());

/// TODO: make this implement Error class 
pub enum NodeError {
    NodeCreationError(String),
    JsonParseError(String),
}

