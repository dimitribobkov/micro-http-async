use tokio::net::{TcpListener, TcpStream}; // Async versions of the stdlib implementation
use tokio::io; // :D

use crate::Connection;
use crate::Routes;

/// # HTTP Server
/// 
/// This struct stores the listener, which listens for incoming connections and handles them
/// 
/// **Example**:
/// 
/// ```
/// let http_server = HttpServer::new("127.0.0.1", "8080").await.unwrap(); // Create a new http listener
/// ```
pub struct HttpServer{
    listener: TcpListener,
    pub routes: Routes,
}


impl HttpServer{

    /// # New
    /// 
    /// Create a new server, with a given IP and port
    /// 
    /// **Example**
    /// ```
    /// let http_server = HttpServer::new("127.0.0.1", "8080").await.unwrap();
    /// ```
    pub async fn new(ip: &str, port: &str) -> io::Result<Self>{
        let address = format!("{}:{}", ip, port);
        Ok(Self{
            listener: TcpListener::bind(&address).await?,
            routes: Routes::new().await,
        })
    }

    /// # Listen
    /// 
    /// Listen for new connections. 
    /// 
    /// Run `handle_connection` upon connection.
    pub async fn listen(&mut self){
        loop{
            let (socket, addr) = self.listener.accept().await.unwrap(); // Accept an incoming connection
            self.handle_connection(socket, addr).await.unwrap(); // Handle it
        }
    }

    /// # Handle Connection
    /// 
    /// This function takes a `TcpStream`, and runs all the necessary functions to read the request,
    /// handle the response and write it back to the user.
    /// 
    /// This function should only be called by the `HttpServer`, as it should only be run upon accepting
    /// a new connection
    /// 
    /// We define the content to return using the `Routes` struct in `HttpServer`
    /// 
    /// It returns a Result for better error handling if something goes wrong at any point during I/O operations
    async fn handle_connection(&mut self, stream: TcpStream, addr: std::net::SocketAddr) -> Result<(), &str>{
        
        let mut connection = Connection::new(stream); // Create our connection handler

        let request_str = connection.read_to_string().await; // get a string value from the recieved data

        // only needs the request and address as it constructs a `Request` to get the route and more info
        let ret_str = self.routes.get_route(request_str, addr).await.unwrap();

        match ret_str{
            crate::DataType::Text(text) => {
                connection.write_string(text).await.unwrap();
            }
            crate::DataType::Bytes(bytes) => {
                connection.write_bytes(bytes).await.unwrap();
            }
        }

        Ok(()) // Return the future
    }
}