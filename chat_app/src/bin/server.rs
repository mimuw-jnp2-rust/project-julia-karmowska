use tokio::net::{TcpListener};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::broadcast;
use chat_app::types;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("localhost:8080").await.unwrap();

    let (tx, _rx) = broadcast::channel(10); //sending strings to max 10 ppl who can connect
    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();
        let tx = tx.clone();//klonowanie tx dla kazdego klienta
        let mut rx = tx.subscribe();//nowy rx dla każdego klienta


        tokio::spawn(async move { //spawnowanie taska obsługi klienta
            let (reader, mut writer) = socket.split(); //podział socketa na czytanie i pisanie

            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            loop {
                tokio::select! {
                    result = reader.read_line( & mut line) => { //pierwszy branch - otrzymanie wiadomosci do przeslania
                        if result.unwrap() == 0 {
                            break;
                        }
                        tx.send((line.clone(), addr)).unwrap();
                        line.clear();
                    }
                    result = rx.recv() => { //drugi branch - otrzymanie wiadomosci przez broadcast
                        let (msg, other_addr) = result.unwrap();
                        if addr != other_addr{
                        writer.write_all(msg.as_bytes()).await.unwrap();}
                    }
                }
            }
        });
    }
}
