/*
 * File: server_node.rs
 * Author: Ethan Graham
 * Date: 06 Feb. 2024
 *
 * Description: implementation of central server protocol
 */
use server::ServerNode;
pub mod server;

static DEFAULT_PORT: u16 = 50_000;

/// main routine
fn main() {

    // init server
    let mut server = match ServerNode::build(DEFAULT_PORT){
        Ok(server_node) => server_node,
        Err(_) => panic!("oops"),
    };

    println!("=====Initializing server=====");
    println!("listening at: {}", server.listening_socket.local_addr().unwrap());
    println!();

    let mut recv_buf = [0; 1024];
    loop {
        match server.listening_socket.recv_from(&mut recv_buf) {
            Ok((n_bytes, src_addr)) => {
                let recv_data = &recv_buf[..n_bytes];
                match server.handle_request(recv_data, src_addr) {
                    Ok(_) => {},
                    Err(_) => {}
                }
            } 
            Err(_) => {}, // don't do anything on error
        }
    }
}

