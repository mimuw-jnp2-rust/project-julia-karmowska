use std::io::stdin;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpStream};
use tokio::sync::mpsc;
use log::{error, info, warn};
use simple_logger::SimpleLogger;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc::{Receiver, Sender};
use chat_app::types;
use types::Message;


use std::string::String;


const SERVER: &str = "localhost:8080";
const QUIT: &str = "/quit\n";

async fn read_msg(mut reader: BufReader<OwnedReadHalf>, mut receiver: Receiver<Message>) {
    let mut line = String::new();
    loop {
        line.clear();

        let received = reader.read_line(&mut line).await;
        info!("received msg from server");
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
                let _mes: Message = serde_json::from_str(input.as_str()).unwrap();
                if input == *QUIT {//user wpisał quit
                    let _quit_msg = input.clone();
                    match sender.send(Message::Quit).await { //wysyłanie wiadomości o zakończeniu
                        Ok(_) => {}
                        Err(_) => {
                            warn!("Error on sending exit");
                        }
                    }
                    break;
                }
                let mut serialized = serde_json::to_string(&Message::ClientMessage {message: input.trim().parse().unwrap() }).unwrap();
                serialized.push('\n');
                let res = writer.write(serialized.as_bytes()).await; //wysyłanie do servera wiadomości
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
    let _user_size = stdin().read_line(&mut username);
    let mut channel_str = String::new();
    println!("Enter channel number(1 - 10)");
    let _channel_size = stdin().read_line(&mut channel_str);
    let channel: usize = channel_str.trim().parse().unwrap();
    let username = username.trim();
    Message::Hello { username: username.to_string(), channel }
}


async fn send_data(writer: &mut BufWriter<OwnedWriteHalf>, reader: &mut BufReader<OwnedReadHalf>) {
    loop {
        let hello = get_initial_data();
        let mut serialized: String = serde_json::to_string(&hello).unwrap();
        serialized.push('\n');
        let _sent = writer.write(serialized.as_bytes()).await;
        let _ = writer.flush().await;
        let mut line = String::new();
        info!("sent message to server");

        match reader.read_line(&mut line).await {
            Ok(_mess) => {
                let message = serde_json::from_str(line.as_str());
                match message {
                    Ok(Message::UsernameTaken) => {
                        println!("Username  taken. Enter data again");
                    }
                    Ok(Message::Ok { .. }) => {
                        info!("received ok message from server");
                        break;
                    }
                    Ok(Message::ChatFull) => {
                        println!("Chat is full. try again later!");
                    }
                    _ => error!("Unexpected message from server (message): {} !", serde_json::to_string(&message.unwrap()).unwrap()),
                }
            }
            Err(_) => { error!("Error on receiving from server(ERR)") }
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
    let (sender, receiver) = mpsc::channel::<Message>(10);//10 wiadomości

    send_data(&mut writer, &mut reader).await;
    info!("sent username and channel no to server");

    tokio::spawn(async move {
        read_msg(reader, receiver).await;
    });

    tokio::spawn(async move {
        write_msg(writer, sender).await;
    });
}