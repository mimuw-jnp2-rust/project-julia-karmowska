use std::io::stdin;
use std::net::SocketAddr;
use std::sync::mpsc::Receiver;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpStream, tcp::ReadHalf};
use tokio::net::tcp::WriteHalf;
use tracing::{error, info};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;

const SERVER: &str = "localhost:8080";
const QUIT: &str = "/quit\n";


//async fn receive(mut reader: BufReader<ReadHalf>, receiver: Receiver<(String, SocketAddr)>) {}

#[tokio::main]
async fn main() {
    let mut stream = TcpStream::connect(SERVER).await.unwrap();
    let (read, mut write) = stream.into_split();
    println!("Connection with server established at {}, enter {} to exit chat", &mut read.peer_addr().unwrap(), QUIT.trim());

    println!("Wait until you will be asked to enter your username");

    let mut reader = BufReader::new(read);
    let mut writer = BufWriter::new(write);
    let (sender, mut receiver) = mpsc::channel::<String>(10);//10 wiadomości

    let mut line = String::new();

    //reader
    tokio::spawn(async move {
        loop {
            line.clear();

            let received = reader.read_line(&mut line).await;
            if let Ok(_received_msg) = receiver.try_recv() { //wiadomość od drugiego taska o zakończeniu
                println!("You have left the chat.");
                break;
            }
            match received {
                Ok(len) =>
                    {
                        if len == 0 {
                            break;
                        }
                        println!("{}", line);
                    }
                Err(_) => {
                    println!("Connection lost! type {} to quit", QUIT.trim());
                }
            }
        }
    });

    //writer
    let mut input = String::new();
    loop {
        input.clear();
        let msg_send = stdin().read_line(&mut input); //user wpisuje wiadomość
        match msg_send {
            Ok(_) => {
                if input == String::from(QUIT) {//user wpisał quit
                    let quit_msg = input.clone();
                    match sender.send(quit_msg).await { //wysyłanie waidomości o zakończeniu
                        Ok(_) => {}
                        Err(_) => {
                            println!("Exiting ");
                        }
                    }
                    break;
                } else {
                    let res = writer.write(input.as_bytes()).await; //wysyłanie do servera wiadomości
                    match res {
                        Ok(_) => { writer.flush().await.expect("Failed to flush buffer") }
                        Err(_) => {
                            println!("Connection lost! type {} to quit", QUIT.trim());
                            break;
                        }
                    }
                }
            }
            Err(_) => {
                println!("Error while reading input. ");
            }
        }
    }
}