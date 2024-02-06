# Basic P2P thing in Rust

Just messing around to learn some rust. ***Blazingly Fast***

## Progress:

Implemented central index server protocol that answers basic JSON request of two 
different types:

- Query: look for the address of a peer given a 128-bit UUID *(hex representation)*
- Register: registers senders IP in map and returns UUID

Check out `query_request.json` and `registration_request.json` for format.

This will only be used for the initial lookup of a peer's IP in the *(to be 
implemented)* client protocol.

## TODO:

- Maybe add some server persistence so that not everything lives in RAM and is
deleted after program exit.
- Make the server multithreaded. Heart tokio is good for that
- implement client protocol that only performs IP lookup for initial discovery
of a peer
- Maybe do something with DHT because cool

