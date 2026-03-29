# http-server

A multithreaded HTTP server built from scratch in Rust with no frameworks. Raw TCP, manual HTTP parsing, and a thread pool implemented using `std::sync` primitives.

## What it does

- Accepts and parses raw HTTP/1.1 requests over TCP
- Routes GET requests to static files across `static/`, `styles/`, and `scripts/` directories
- Serves correct `Content-Type` headers for `.html`, `.css`, and `.js` files
- Handles concurrent connections
- Path traversal protection 

## Architecture

The server is built around three components:

**TCP Listener + Channel**
The main thread binds a `TcpListener` and sends each incoming `TcpStream` through an `mpsc` channel to the worker pool. 

**Thread Pool**
Four worker threads are spawned at startup. Each holds a clone of an `Arc<Mutex<Receiver>>`, using it to handle the connection appropriately.

**Request Handling**
Each connection is parsed manually: the request line is split to extract the method and route, the file extension determines the `DocType`, and the response is formatted and written directly to the stream.

## Running it

```bash
git clone https://github.com/yourusername/http-server
cd http-server
cargo run
```

Then open `http://localhost:8080` in your browser.

## File structure

```
http-server/
  static/       # HTML pages
  styles/       # CSS stylesheets
  scripts/      # JavaScript files
  src/
    main.rs
```

To experiment, add any `.html` file to `static/` and it will be accessible at `/<filename>` automatically. Same applies to `.css` in `styles/` and `.js` in `scripts/`.

## What I learned

Building this made the abstractions behind frameworks like Flask and Express concrete. A route decorator is just a match arm. The magic became mechanical once I reasoned with the problem myself.

Thread pool design with Arc<Mutex<>>, and path traversal attacks and how to prevent them.
