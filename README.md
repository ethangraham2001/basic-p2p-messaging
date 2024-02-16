# Basic P2P thing in Rust

Just messing around to learn some rust. ***Blazingly Fast***

## Progress:

### Central Index Server Protocol

Implemented central index server protocol that answers basic JSON request of two 
different types:

- Query: look for the address of a peer given a 128-bit UUID *(hex representation)*
- Register: registers senders IP in map and returns UUID

Check out `query_request.json` and `registration_request.json` for format.

This will only be used for the initial lookup of a peer's IP in the *(to be 
implemented)* client protocol.

### Client Protocol

Upon program launch, the client registers with central index server, which will
respond with its UUID. From there on out, every time the client sends a message
to another peer:

1. If the peer's `(uuid, IP)` is known, the message is sent directly
2. If the mapping is unknown, *(only UUID is known)* the client queries the 
server for it. Upon server response, it will cache the mapping and go to step 1.

#### Implementation

I implement a `struct Client` that consists of

- A thread safe listening socket
- A thread safe `(uuid, IP:port)` map
- A thread safe queue for received messages
- A UUID

Which are acted on by three threads running concurrently, one handling sending
messages from user input, one handling message reception, and one handling
message display.

Messages are represented in `struct Message` which just wraps a few things
such as destination UUID and source UUID, as well as defining a few things such
as `from_json()` as all messages are JSON-formatted.

## Bugs

- There seems to be some blocking happening, as sometimes the program will stall
for a while without receiving any new messages and then receive them all at
once. I suspect that this is related to one of the loops holding a lock on
`recv_queue` for longer than it should, halting the progress of the other thread
*(only two threads operate on `recv_queue` at a given time)*.
- Need to fix parsing of creation time from JSON. Shouldn't be too difficult, 
it's just not a priority right now.

## TODO:

- Everything is running on `localhost` right now. Ideally this should work for
any arbitrary IP addresses. I've heard this can be quite a challenge, but will
look into this.
- Maybe add some server persistence so that not everything lives in RAM and is
deleted after program exit.
- Make the server multithreaded. Heart tokio is good for that
- Maybe do something with DHT because cool

