use std::net::SocketAddr;
use std::{env, io::Error};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::{self, Sender, Receiver};

type MessageInfo = (String, SocketAddr);

const DEFAULT_SERVER_ADDRESS: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Error> {
    let server_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_SERVER_ADDRESS.to_string());

    let socket = TcpListener::bind(&server_addr).await;
    let listener = socket.expect("Failed to bind");

    let (tx, _rx) = broadcast::channel::<MessageInfo>(10);

    while let Ok((socket, addr)) = listener.accept().await {
        let tx = tx.clone();
        let rx = tx.subscribe();
        tokio::spawn(async move { accept_connection(socket, tx, rx, addr).await });
    }

    Ok(())
}

async fn accept_connection(mut stream: TcpStream, tx: Sender<MessageInfo>, mut rx: Receiver<MessageInfo>, addr: SocketAddr) {
    println!("Accepting from: {}", addr);

    let (reader, mut writer) = stream.split();

    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();
    loop {
        // 最初に値を返した方を適用する
        // futureを与えるため，アーム部にawaitはいらない
        tokio::select! {
            result = buf_reader.read_line(&mut line) => {
                match result {
                    Ok(bytes) => {
                        if bytes == 0 {
                            println!("Connection closed");
                            break;
                        }
                    }
                    Err(error) => {
                        println!("Error: {}", error);
                    }
                }
                tx.send((line.clone(), addr)).unwrap();
                line.clear();
            }

            result = rx.recv() => {
                let (msg, other_addr) = result.unwrap();
                if addr != other_addr {
                    writer.write_all(msg.as_bytes()).await.unwrap();
                }
            }
        }
    }
}