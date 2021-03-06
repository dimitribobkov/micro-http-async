use std::collections::HashMap;
use std::future::Future;
use crate::Request;
use tokio::io::AsyncReadExt;
use chunked_transfer::Encoder;
use std::io::Write;


/// # DataType
/// 
/// This returns the data type of the response, wrapping the response as well
/// 
/// Used mostly for returning static images as bytes
/// 
/// for example, if you're requesting for a static image from say `/static/img.png`,
/// you would want `Bytes(content)` instead of `Text(content)`. The API already handles
/// this for you, but it is worth keeping in mind how it works behind the scenes
pub enum DataType{
    Text(String),
    Bytes(Vec<u8>)
}

/// # Routes
/// 
/// This struct defines the routes. It uses a hashmap to do this.
/// 
/// `HashMap<Route, Content>` where content is the return content (ie, html or json).
pub struct Routes{
    routes: HashMap::<String, std::pin::Pin<Box<dyn Fn(Request) -> std::pin::Pin<Box<dyn Future<Output = Result<String, String>> + Send>>>>>
}

impl Routes{
    /// # New
    /// 
    /// Create a new `Route` struct
    pub async fn new() -> Self{
        Self{
            routes: HashMap::<String, std::pin::Pin<Box<dyn Fn(Request) -> std::pin::Pin<Box<dyn Future<Output = Result<String, String>> + Send>>>>>::new()
        }
    }

    /// # Add Route
    /// 
    /// Adds a new route to the routes hashmap. If the route already exists,
    /// its value is updated
    pub async fn add_route(&mut self, route: String, content: std::pin::Pin<Box<dyn Fn(Request) -> std::pin::Pin<Box<dyn Future<Output = Result<String, String>> + Send>>>>){
        self.routes.insert(route, content);
    }

    /// # Get Route
    /// 
    /// This function takes in the response string from the `TcpStream` and searches the hashmap
    /// for the callback function associated with the route. It then checks that the route is valid,
    /// and runs it asynchrynously (using the request so that the callback can make use of the request data)
    /// 
    /// This function only runs the callback - handling POST and GET requests is up to the callback.
    /// 
    /// If this function detects a request for static content - which it can only detect if the data is stored in
    /// `/static/`, then it will return early with the static content, and not run any functions.
    pub async fn get_route(&self, request: String, user_addr: std::net::SocketAddr) -> Result<DataType, &str>{
        let request = Request::new(request, user_addr);

        // Handle static files
        if request.uri.contains("static"){
            let file_path = format!(".{}", request.uri);
            return match tokio::fs::File::open(file_path).await{
                Ok(mut file_handle) => {
                    let mut contents = vec![];
                    file_handle.read_to_end(&mut contents).await.unwrap();
                    let result = String::from("HTTP/1.1 {} {}\r\nContent-type: image/jpeg;\r\nTransfer-Encoding: chunked\r\n\r\n");
                    let mut result = result.into_bytes();
                    let mut encoded = Vec::new();
                    {
                        let mut encoder = Encoder::with_chunks_size(&mut encoded, 8);
                        encoder.write_all(&contents).unwrap();
                    }
                    result.extend(&encoded);
                    match String::from_utf8(result.clone()){
                        Ok(_) => {
                            let result = String::from("HTTP/1.1 {} {}\r\nContent-type: text/css;\r\nTransfer-Encoding: chunked\r\n\r\n");
                            let mut result = result.into_bytes();
                            result.extend(&encoded);
                            let v = String::from_utf8(result).expect("This should work");
                            return Ok(DataType::Text(v))
                        }
                        Err(_) => {
                            return Ok(DataType::Bytes(result))
                        }
                    }
                }
                Err(e) => {
                    println!("Error loading static content: {}", e);
                    Ok(DataType::Text(String::from("ERROR - CONTENT NOT AVAILABLE")))
                }
            };
        }

        // If not static, handle the request
        let func = match self.routes.get(&request.uri){
            Some(v) => v,
            None => {
                println!("Error - user requested '{}', which does not exist on this server.", request.uri);
                self.routes.get(&"err".to_string()).unwrap()// we assume we've got an error handler
            } 
        };
           
        // Check that our function returned an Ok result, and unwrap it after it executes
        let result = if let Ok(v) = func(request).await{
            return Ok(DataType::Text(v));
        }else{
            DataType::Text(String::new()) // Err returned, just return nothing
        };

        Ok(result)
    }
}