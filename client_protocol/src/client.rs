/*
 * File: client.rs
 * Author: Ethan Graham
 * Date: 07 Feb. 2024
 *
 * Description: Client struct and implementations of protocol
 */
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, mpsc},
    net::SocketAddr,
    io::stdin,
};
use tokio::{
    net::UdpSocket,
    time::{self, Duration},
    sync::Mutex,
};
use json::{JsonValue, stringify, parse};
use uuid::Uuid;
use crate::message::{Message, MessageError};

// the host port. Change to IP addr in future.
static HOST_PORT: u16 = 50_000;
// client UUID is NULL upon creation. Changed upon registration.
static NULL_UUID_STR: &str = "00000000-0000-0000-0000-000000000000";

/// Client in the p2p network
#[derive(Clone)]
pub struct Client {
    pub listening_socket: Arc<Mutex<UdpSocket>>,
    pub peer_map: Arc<Mutex<HashMap<Uuid, SocketAddr>>>,
    pub recv_queue: Arc<Mutex<VecDeque<Message>>>,
    pub uuid: Uuid,
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
            uuid: NULL_UUID_STR.to_string().parse().unwrap()
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
        msg_json["dst_uuid"] = JsonValue::from(peer_uuid.to_string());
        msg_json["data"] = JsonValue::from(msg_data.to_string());
        msg_json["creation_time"] = JsonValue::from(0.to_string());

        // bind socket and send message to recipient. 
        let out_sock = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(socket) => socket,
            Err(err) => {
                let err_msg = format!("Could not bind UDP socket. {}", err);
                return Err(ClientError::UdpFailureError(err_msg))
            }
        };
        match out_sock.send_to(msg_json.dump().as_bytes(), addr).await {
            Ok(_) => Ok(()),
            Err(err) => { 
                let err_msg = format!("Could not send message to recipient: {}",
                                      err);
                Err(ClientError::UdpFailureError(err_msg))
            }
        }
    }

    /// Registers a client with the server. Done upon initialization.
    /// Sets UUID in client object, hence the &mut
    pub async fn register_with_server(&mut self) -> Result<Uuid, ClientError> {
        // bind arbitrary socket
        let out_socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        let mut request = JsonValue::new_object();


        request["req_type"] = JsonValue::from("registration".to_string());

        let socket_locked = self.listening_socket.lock().await;
        request["addr"] = JsonValue::from(socket_locked.local_addr().unwrap()
                                          .to_string());

        let host_addr = SocketAddr::from(([127, 0, 0, 1], HOST_PORT));

        match out_socket.send_to(request.dump().as_bytes(), host_addr).await {
            Ok(_) => {},
            Err(_) => 
                return Err(
                    ClientError::ServerUnavailableError("server unavailable"
                                                        .to_string()))
        }

        // wait for server response
        let mut buf = [0u8; 2014];
        let (size, _) = match out_socket.recv_from(&mut buf).await {
            Ok((len, addr)) => (len, addr),
            Err(err) => {
                let err_msg = format!(
                    "Error waiting for server response. {}", err);
                return Err(ClientError::ServerUnavailableError(err_msg));
            }
        };

        let server_resp = 
            String::from_utf8(buf[..size].to_vec()).unwrap();
        let server_resp = json::parse(&server_resp).unwrap();

        let client_uuid = &server_resp["uuid"].to_string();
        let status = &server_resp["status"].to_string();

        if status != "OK" {
            // TODO change this error type
            return Err(ClientError::ServerUnavailableError(
                    "Error registering with server".to_string()));
        }

        // assumes that the server sends a valid uuid
        let client_uuid = client_uuid.parse::<Uuid>().unwrap();
        
        // update UUID
        self.uuid = client_uuid;

        Ok(client_uuid)
    }

    /// queries the central index server for a uuid, and adds the new mapping
    /// to the caller's `peer_map`
    ///
    /// `peer_uuid`: queried uuid
    async fn server_lookup_uuid(&self, peer_uuid: &Uuid) 
        -> Result<SocketAddr, ClientError> {

        // prepare json request
        let mut query           = JsonValue::new_object();
        query["req_type"]       = JsonValue::from("query");
        query["queried_uuid"]   = JsonValue::from(peer_uuid.to_string());

        // send query to server
        let host_addr = SocketAddr::from(([127, 0, 0, 1], HOST_PORT));
        
        // socket for sending traffic to server
        let out_socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();

        match out_socket.send_to(query.dump()
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
        let (size, _) = match out_socket.recv_from(&mut buf).await {
            Ok((len, addr)) => (len, addr),
            Err(err) => {
                let err_msg = format!(
                    "Error waiting for server response. {}", err);
                return Err(ClientError::ServerUnavailableError(err_msg));
            }
        };

        // unwrap assumes that error sends valid response.
        // TODO: Consider error handling here. For the time being, I assume
        // that server functions correctly and following the protocol perfectly
        let server_resp = 
            String::from_utf8(buf[..size].to_vec()).unwrap();
        let server_resp = json::parse(&server_resp).unwrap();

        let recv_ip = &server_resp["address"].to_string();
        if recv_ip == "nil" {
            return Err(ClientError::PeerNotFoundError(
                    "No peer matching provided UUID".to_string()));
        }

        // return found socket address. Shouldn't fail at this point in time
        match recv_ip.to_string().parse::<SocketAddr>() {
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
    /// for peer discovery in the case that recipient is unknown.
    pub async fn outgoing_traff_loop(&mut self) {

        'main_loop: loop {

            let mut dst_uuid = String::new();
            let mut msg = String::new();

            println!("=======================================================");
            println!("Please enter a message >> ");
            let _ = stdin().read_line(&mut msg).unwrap();
            println!("");

            println!("Please enter a source uuid >> ");
            let _ = stdin().read_line(&mut dst_uuid).unwrap();
            println!("");

            // attempt to parse uuid. `len - 1` to remove trailiing '\n' from
            // pressing ENTER in cli
            let dst_uuid = dst_uuid.to_string()
                .replace(" ", "")
                .replace("\n", "");
            let peer_uuid: Uuid = match dst_uuid.parse() {
                Ok(valid_uuid) => valid_uuid,
                Err(_) => {
                    println!("\x1b[31mError parsing UUID. Try again.\x1b[0m");
                    continue 'main_loop;
                },
            };

            match self.send_message(&peer_uuid, &msg).await {
                Ok(_) => println!("\x1b[1mSent message successfully...\x1b[0m"),
                Err(err) => {
                    println!("Error sending message: {}", err);
                    continue 'main_loop;
                }
            }
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
                    let disp_line = 
                        "=================================================";
                    println!("{}", disp_line);
                    println!("Message from \x1b[36m{}\x1b[0m", msg.src_uuid);
                    println!("\x1b[1mSent:\x1b[0m[{}]", msg.creation_time);
                    println!("\x1b[1mContent:\x1b[0m");
                    println!("{}", msg.data);
                    println!("{}", disp_line);
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

