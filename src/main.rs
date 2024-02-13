use serde_json::{Map, Value};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    #[arg(short, long, default_value = "0.0.0.0")]
    binding_ip: String,

    #[arg(short, long, default_value_t = 80)]
    port: u32,
    
    #[arg(short, long, default_value_t = 512)]
    max_request_size: usize,
}

#[tokio::main]
async fn main() {
    let config = Args::parse();

    let server = TcpListener::bind(format!("{}:{}", config.binding_ip, config.port)).await.unwrap();

    println!("server running on port {}:{}", config.binding_ip, config.port);

    loop {
        let (mut stream, _addr) = server.accept().await.unwrap();

        tokio::spawn(async move {
            let mut buffer = vec![0; config.max_request_size];
            let len = stream.read(&mut buffer).await.unwrap();
            buffer.truncate(len);

            let stringified = String::from_utf8(buffer).unwrap();
            let mut lines: Vec<&str> = stringified.split("\r\n").collect();

            let initial = lines.remove(0);

            //bad data
            if initial.len() == 0 {
                return;
            }

            let (method, right) = initial.split_once(" ").unwrap();
            let (path, version) = right.rsplit_once(" ").unwrap();
            
            let mut headers: Map<String, Value> = Map::new();
            for line in lines {
                match line.split_once(":") {
                    Some((key, value)) => {
                        headers.insert(key.into(), value.trim_start().into());
                    }
                    None => {
                        headers.insert(line.into(), "".into());
                    }
                }
            }

            let mut response_map: Map<String, Value> = Map::new();
            response_map.insert("method".into(), method.into());
            response_map.insert("path".into(), path.into());
            response_map.insert("version".into(), version.into());
            response_map.insert("headers".into(), headers.into());

            let json: Vec<u8> = serde_json::to_vec(&Value::Object(response_map)).unwrap();

            let mut response: Vec<u8> = vec![];
            response.extend(b"HTTP/1.1 200 OK\r\n");
            response.extend(format!("content-length: {}\r\n", json.len()).as_bytes());
            response.extend(b"content-type: application/json\r\n");
            response.extend(b"\r\n");
            response.extend(&json);

            stream.write(&response).await.unwrap();
        });
    }
}
