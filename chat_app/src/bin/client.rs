use std::io::stdin;
use std::string;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpStream};
use tokio::sync::mpsc;
use log::{error, info, warn};
use simple_logger::SimpleLogger;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc::{Receiver, Sender};
use chat_app::types;
use types::Message;
use serde::{Serialize, Deserialize};
use serde_json;
use std::string::String;

const SERVER: &str = "localhost:8080";
const QUIT: &str = "/quit\n";

async fn read_msg(mut reader: BufReader<OwnedReadHalf>, mut receiver: Receiver<Message>) {
    let mut line = String::new();
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
                    print!("{}", line); //wypisywanie otrzymanej wiadomości
                }
            Err(_) => {
                info!("Connection lost! type {} to quit", QUIT.trim());
            }
        }
    }
}

async fn write_msg(mut writer: BufWriter<OwnedWriteHalf>, sender: Sender<Message>) {
    let mut input = String::new();
    loop {
        input.clear();
        let msg_send = stdin().read_line(&mut input); //user wpisuje wiadomość
        match msg_send {
            Ok(_) => {
                let mes: Message = serde_json::from_str(input.as_str()).unwrap();
                if input == *QUIT {//user wpisał quit
                    let quit_msg = input.clone();
                    match sender.send(Message::Quit {}).await { //wysyłanie wiadomości o zakończeniu
                        Ok(_) => {}
                        Err(_) => {
                            warn!("Error on sending exit");
                        }
                    }
                    break;
                }
                let res = writer.write(input.as_bytes()).await; //wysyłanie do servera wiadomości
                match res {
                    Ok(_) => { writer.flush().await.expect("Failed to flush buffer") }
                    Err(_) => {
                        warn!("Connection lost! type {} to quit", QUIT.trim());
                        break;
                    }
                }
            }
            Err(_) => {
                warn!("Error while reading input. ");
            }
        }
    }
}

fn get_initial_data() -> Message {
    println!("Enter your username");

    let mut username = String::new();
    let user_size = stdin().read_line(&mut username);
    let mut channel_str = String::new();
    println!("Enter channel number(1 - 10)");
    let channel_size = stdin().read_line(&mut channel_str);
    let channel: usize = channel_str.trim().parse().unwrap();
    Message::Hello { username, channel }
}


async fn send_data(writer: &mut BufWriter<OwnedWriteHalf>, reader: &mut BufReader<OwnedReadHalf>) {
    loop {
        let hello = get_initial_data();
        let sent = writer.write(serde_json::to_string(&hello).unwrap().as_bytes()).await;
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(mess) => {
                let message = serde_json::from_str(line.as_str());
                match message {
                    Ok(Message::UsernameTaken { .. }) => {
                        println!("Username  taken. Enter data again");
                    }
                    Ok(Message::Ok { .. }) => { break; }
                    _ => error!("Unexpected message from server!"),
                }
            }
            Err(_) => { error!("Error on receiving from server") }
        }
    }
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().init().unwrap();

    let stream = TcpStream::connect(SERVER).await.unwrap();
    let (read, write) = stream.into_split();
    info!("Connection with server established at {}, enter {} to exit chat", &mut read.peer_addr().unwrap(), QUIT.trim());


    let mut reader = BufReader::new(read);
    let mut writer = BufWriter::new(write);
    let (sender, mut receiver) = mpsc::channel::<Message>(10);//10 wiadomości

    send_data(&mut writer, &mut reader).await;
    let mut line = String::new();
    match reader.read_line(&mut line).await {
        Ok(_) => {
            match serde_json::from_str(line.as_str()) {
                Ok(Message::Ok {}) => {}
                Ok(Message::ChatFull {}) => {
                    println!("Chat is full. try again later!");
                    return;
                }
            }
        }
        Err(_) => {
            error!("error on reading from server");
            return;
        }
    };

    tokio::spawn(async move {
        read_msg(reader, receiver).await;
    });

    tokio::spawn(async move {
        write_msg(writer, sender).await;
    });
}