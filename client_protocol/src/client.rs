/*
 * File: client.rs
 * Author: Ethan Graham
 * Date: 07 Feb. 2024
 *
 * Description: Client struct and implementations of protocol
 */
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, mpsc};
use std::net::SocketAddr;
use tokio::{
    net::UdpSocket,
    time::{self, Duration},
    sync::Mutex,
};
use json::JsonValue;
use uuid::Uuid;
use crate::message::{Message, MessageError};

/// Client in the p2p network
#[derive(Clone)]
pub struct Client {
    listening_socket: Arc<Mutex<UdpSocket>>,
    peer_map: Arc<Mutex<HashMap<Uuid, SocketAddr>>>,
    recv_queue: Arc<Mutex<VecDeque<Message>>>,
    uuid: Uuid,
}

/// protocol implementations
impl Client {
    /// build a new Client
    ///
    /// `port`: the port that the client will listen on
    pub async fn build(port: u16) -> Result<Client, ClientError> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        
        // attempt to bind UDP socket
        let listening_socket = match UdpSocket::bind(addr).await {
            Ok(socket) => socket,
            Err(err) => {
                let err_msg = format!(
                    "problem creating client: {err}");
                return Err(ClientError::ClientCreationError(err_msg));
            }
        };

        let peer_map: HashMap<Uuid, SocketAddr> = HashMap::new();
        let recv_queue: Arc<Mutex<VecDeque<Message>>> =
            Arc::new(Mutex::new(VecDeque::new()));
        Ok(Client{ 
            listening_socket: Arc::new(Mutex::new(listening_socket)), 
            peer_map: Arc::new(Mutex::new(peer_map)), 
            recv_queue,
            uuid: Uuid::new_v4() // randomly generated
        })
    }

    /// sends a message to recipient with known UUID.
    ///
    /// `self`'s `peer_map` can be modified in the event that a server lookup
    /// takes place for an unknown `(peer_uid, addr)` mapping.
    ///
    /// `peer_uuid`: the uuid of the recipient
    /// `msg`: the message data
    async fn send_message(&mut self, peer_uuid: &Uuid, msg_data: &str) 
        -> Result<(), ClientError> {
        
        let mut peer_map = self.peer_map.lock().await;

        // check if the (uuid <-> addr) is cached. Otherwise retrieve from CIS 
        if !peer_map.contains_key(peer_uuid) {
            match self.server_lookup_uuid(peer_uuid).await {
                Ok(socket_addr) => { 
                    peer_map.insert(*peer_uuid, socket_addr); 
                },
                Err(err) => 
                    return Err(err),
            }
        }

        // address will be cached now
        let addr = peer_map.get(peer_uuid).unwrap();

        // format msg as JSON
        let mut msg_json = JsonValue::new_object();
        msg_json["src_uuid"] = JsonValue::from(self.uuid.to_string());
        msg_json["src_uuid"] = JsonValue::from(peer_uuid.to_string());
        msg_json["data"] = JsonValue::from(msg_data.to_string());
        msg_json["creation_time"] = JsonValue::from(0.to_string());

        // bind socket and send message to recipient
        let out_sock = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        match out_sock.send_to(msg_json.dump().as_bytes(), addr).await {
            Ok(_) => Ok(()),
            Err(err) => 
                Err(ClientError::UdpFailureError(err.to_string())),
        }
    }

    /// queries the central index server for a uuid, and adds the new mapping
    /// to the caller's `peer_map`
    ///
    /// `peer_uuid`: queried uuid
    async fn server_lookup_uuid(&self, peer_uuid: &Uuid) 
        -> Result<SocketAddr, ClientError> {

        // prepare json request
        let mut query       = JsonValue::new_object();
        query["req_type"]   = JsonValue::from("query");
        query["uuid"]       = JsonValue::from(peer_uuid.to_string());

        // send query to server
        let host_addr = SocketAddr::from(([127, 0, 0, 1], 50_000));
        match self.listening_socket.lock().await.send_to(query.dump()
                .as_bytes(), host_addr).await {
            Ok(n_bytes) => {
                println!("send {} bytes to central index server", n_bytes);
            }
            Err(err) => {
                let err_msg = format!(
                    "Unable to reach server. {}", err);
                return Err(ClientError::ServerUnavailableError(err_msg));
            }
        }

        // buffer for server response
        let mut buf = [0; 1024];
        let (size, _) = match self.listening_socket.lock().await
            .recv_from(&mut buf).await {
            Ok((len, addr)) => (len, addr),
            Err(err) => {
                let err_msg = format!(
                    "Error waiting for server response. {}", err);
                return Err(ClientError::ServerUnavailableError(err_msg));
            }
        };

        // unwrap assumes that error sends valid response.
        let server_resp = json::JsonValue::from(
            String::from_utf8(buf[..size].to_vec()).unwrap());

        let recv_ip = &server_resp["ip"].to_string();

        if recv_ip == "nil" {
            return Err(ClientError::PeerNotFoundError(
                    "No peer matching provided UUID".to_string()));
        }

        // return found socket address. Shouldn't fail at this point in time
        match recv_ip.parse::<SocketAddr>() {
            Ok(addr) => Ok(addr),
            Err(_) => Err(ClientError::ServerUnavailableError("Fuck"
                                                              .to_string())),
        }
    }

    /// listens for incoming traffic, posts the messages in the recv_queue
    /// for display by display_loop()
    pub async fn incoming_traff_loop(&mut self){
        let mut recv_buf: [u8; 1024] = [0; 1024];
        let arc_ref = Arc::clone(&self.recv_queue);

        // loop and ask client for message to send
        'main_loop: loop {
            let (recv_len, _) = self.listening_socket
                .lock()
                .await
                .recv_from(&mut recv_buf)
                .await.unwrap();
            println!("Received {}B", recv_len);

            // "functional code is so readable"
            let json_data = json::parse(&String::from_utf8(recv_buf[..recv_len]
                .to_vec()).unwrap()).unwrap();

            let msg = match Message::from_json(json_data) {
                Ok(msg) => msg,
                Err(err) => {
                    println!("{}", err);
                    continue 'main_loop;
                }
            };
            
            // different scope so that lock can be dropped before sleep
            {
                arc_ref.lock().await.push_back(msg);
            }

            // sleep for 0.2 seconds. Dunno seemed like a reasonable time
            let _ = time::sleep(Duration::from_millis(200)).await;
        }
    }

    /// sends outgoing traffic. `self` is mutable since a server lookup happens
    /// for peer discovery
    async fn outgoing_traff_loop(&mut self) {
        loop {
        }
    }

    /// checks out the recv_queue to see if there are incoming messages that 
    /// need to be displayed, and displays them
    pub async fn display_loop(&mut self) {
        let arc_ref = Arc::clone(&(self.recv_queue));
        loop {
            {
                // inner scope so that the thread doesn't sleep with the lock.
                let mut locked_queue = arc_ref.lock().await;
                // display all messages if there are any available
                while !locked_queue.is_empty() {
                    let msg = locked_queue.pop_back().unwrap();
                    println!("=================================================");
                    println!("Message from \x1b[36m{}\x1b[0m", msg.src_uuid);
                    println!("\x1b[1mSent:\x1b[0m[{}]", msg.creation_time);
                    println!("\x1b[1mContent:\x1b[0m");
                    println!("{}", msg.data);
                    println!("=================================================");
                }
            }
            // TODO: implement .to_string() for `Message`
            let _ = time::sleep(Duration::from_millis(1000)).await;
        }
    }
}

use std::error;
use std::fmt;

/// errors relating to client activity.
#[derive(Debug, Clone)]
pub enum ClientError {
    ServerUnavailableError(String),
    PeerNotFoundError(String),
    UdpFailureError(String),
    ClientCreationError(String),
}

impl error::Error for ClientError {}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::ServerUnavailableError(msg) => 
                write!(f, "ServerUnavailableError: {}", msg),
            ClientError::PeerNotFoundError(msg) => 
                write!(f, "PeerNotFoundError: {}", msg),
            ClientError::UdpFailureError(msg) => 
                write!(f, "UdpFailureError: {}", msg),
            ClientError::ClientCreationError(msg) => 
                write!(f, "ClientCreationError: {}", msg),
        }
    }
}

