/*
 * File: client.rs
 * Author: Ethan Graham
 * Date: 07 Feb. 2024
 *
 * Description: Client struct and implementations of protocol
 */
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use json;
use uuid::Uuid;

/// Client in the p2p network
pub struct Client {
    listening_socket: UdpSocket,
    peer_map: HashMap<Uuid, SocketAddr>
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

        let map: HashMap<Uuid, SocketAddr> = HashMap::new();
        Ok(Client{ listening_socket, peer_map: map })

    }
    /// sends a message to recipient with known UUID
    ///
    /// `peer_uuid`: the uuid of the recipient
    /// `msg`: the message data
    async fn send_message(&self, peer_uuid: Uuid, msg: String) 
        -> Result<(), ClientError> {

        let peer_addr = match self.peer_map.get(&peer_uuid) {
            Some(socket_addr) => {
                socket_addr.to_owned()
            },
            None => match self.server_lookup_uuid(&peer_uuid).await {
                Ok(val) => val,
                Err(err) => return Err(err)
            }
        };

        Ok(())
    }

    /// queries the central index server for a uuid
    ///
    /// `peer_uuid`: queried uuid
    async fn server_lookup_uuid(&self, peer_uuid: &Uuid) 
        -> Result<SocketAddr, ClientError> {

        // prepare json request
        let mut query = json::JsonValue::new_object();
        query["req_type"] = json::JsonValue::from("query");
        query["uuid"] = json::JsonValue::from(peer_uuid.to_string());

        // send query to server
        let host_addr = SocketAddr::from(([127, 0, 0, 1], 50_000));
        match self.listening_socket.send_to(query.dump().as_bytes(), host_addr)
            .await {
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
        let (size, _) = match self.listening_socket.recv_from(&mut buf)
            .await {
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
            Err(_) => Err(ClientError::ServerUnavailableError("Fuck".
                                                              to_string())),
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
                write!(f, "ServerUnavailableError: {}", msg),
            ClientError::UdpFailureError(msg) => 
                write!(f, "ServerUnavailableError: {}", msg),
            ClientError::ClientCreationError(msg) => 
                write!(f, "ServerUnavailableError: {}", msg),
        }
    }
}

