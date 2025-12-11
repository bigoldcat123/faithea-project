//! Example HTTP server demonstrating the usage of the http-server library.
//!
//! This example creates a simple HTTP server with:
//! - Multiple route handlers for different URL patterns
//! - Guard middleware for request validation and logging
//! - File serving capabilities
//!
//! # Running the Example
//!
//! ```sh
//! cargo run --bin server
//! ```
//!
//! Then visit:
//! - http://127.0.0.1:8899/hello/world - Path parameter example
//! - http://127.0.0.1:8899/file - File serving example
//! - http://127.0.0.1:8899/ - Root path example
//! - http://127.0.0.1:8899/other - Will trigger guard logging

use http_server::{guard::GuardTire, handler::HandlerTire, request::HttpRequest, response::HttpResponse, server::HttpServer};
use tokio::fs::File;

/// Example handler that returns a simple text response.
///
/// This handler demonstrates path parameter usage where `{abc}` in the route
/// captures a segment from the URL. The captured value could be accessed
/// from the request if needed.
///
/// # Response
/// - Status: 200 OK
/// - Content-Type: implicit
/// - Body: "12345"
async fn handle_path_param(_req: HttpRequest) -> HttpResponse {
    let mut res = HttpResponse::new();
    res.add_header(("content-length", "5"));
    res.set_body(http_server::response::ResponseBody::Simple("12345".into()));
    res
}

/// Example handler that serves a file from the filesystem.
///
/// This handler demonstrates file streaming capabilities. It opens the
/// current source file and streams it back to the client with the correct
/// Content-Length header.
///
/// # Response
/// - Status: 200 OK
/// - Content-Length: file size
/// - Body: File content (this source file)
async fn handle_file(_req: HttpRequest) -> HttpResponse {
    let mut res = HttpResponse::new();
    
    // Open the current source file for demonstration
    let file_path = "/Users/dadigua/Desktop/graduation/http-server/src/bin/server.rs";
    let f = File::open(file_path).await.unwrap();
    let len = f.metadata().await.unwrap().len();
    
    res.add_header(("content-length", len.to_string().as_str()));
    // Note: You could add Content-Type header for better browser compatibility
    // res.add_header(("Content-Type", "text/plain; charset=utf-8"));
    
    res.set_body(http_server::response::ResponseBody::File(f));
    res
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Set up the handler routing trie
    let mut handler = HandlerTire::default();
    
    // Register handlers for different route patterns
    
    // Path parameter route: captures any segment after /hello/
    handler.add("/hello/{abc}", handle_path_param);
    
    // Exact match route: only matches /file exactly
    handler.add("/file", handle_file);
    
    // Root path handler: serves the same file as /file
    handler.add("/", handle_file);
    
    // Set up the guard middleware trie
    let mut guards = GuardTire::default();
    
    // Guard for all paths under /hello/ with a single segment wildcard
    // This guard will execute for paths like /hello/world, /hello/123, etc.
    guards.add("/hello/*", async |req| {
        println!("[Guard 1] Processing request under /hello/* path");
        // Example guard logic: could validate authentication, log metrics, etc.
        // Returning Ok(req) allows the request to proceed to the handler
        // Returning Err(HttpResponse) would stop processing and send error response
        Ok(req)
    });
    
    // Guard for a specific exact path
    guards.add("/hello/asdasd", async |req| {
        println!("[Guard 2] Processing request for exact path /hello/asdasd");
        // This guard demonstrates that more specific paths are matched first
        Ok(req)
    });
    
    // Catch-all guard for all requests (lowest priority)
    guards.add("/**", async |req| {
        println!("[Guard 3] Processing any request (catch-all)");
        // This guard runs for ALL requests due to multi-segment wildcard pattern
        // Useful for logging, CORS headers, rate limiting, etc.
        Ok(req)
    });
    
    // Create the HTTP server instance
    // Bind to localhost port 8899 with the configured handlers and guards
    let server = HttpServer::new("127.0.0.1:8899", handler, guards);
    
    println!("HTTP server starting on http://127.0.0.1:8899");
    println!("Press Ctrl+C to stop the server");
    
    // Start the server (this runs indefinitely)
    server.start().await;
}