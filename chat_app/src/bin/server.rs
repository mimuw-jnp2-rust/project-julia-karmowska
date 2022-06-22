use std::borrow::BorrowMut;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, };
use tokio::net::{TcpListener};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Mutex};

use dashmap::DashMap;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc::{Receiver, Sender};

use log::{info, warn};
use simple_logger::SimpleLogger;
use chat_app::types::Message;


const LOCAL: &str = "localhost:8080";
const MAX_CLIENT_NUM: usize = 20;
const NAME_SIZE: usize = 30;
const MESSAGE_SIZE: usize = 256;
const ERROR_MSG: &str = "**************ERROR_MSG*************\n";
const CHANNEL_COUNT: usize = 10;

async fn write_buf<'a>(writer: &mut OwnedWriteHalf, message: &str) {
    writer.write(message.as_bytes()).await.expect("Failed to write msg");
    writer.flush().await.expect("Failed to flush after write");
}

//czeka na username
async fn request_username(reader: &mut BufReader<OwnedReadHalf>,
                          writer: &mut OwnedWriteHalf,
                          clients: &mut Arc<DashMap<String, OwnedWriteHalf>>,
) -> Result<String, String> {
    let mut buffer = String::with_capacity(NAME_SIZE);
    loop {
        buffer.clear();
        match reader.read_line(&mut buffer).await {
            Ok(_) => {
                let mut name = String::from(buffer.trim()); //ucinanie '\n'
                name.truncate(NAME_SIZE); //skracanie username
                if clients.contains_key(name.as_str()) {
                    write_buf(writer, "Name already taken, please try another!").await;
                } else {
                    return Ok(name); //ok - poprawny username
                }
            }
            Err(_) => { return Err(String::from(ERROR_MSG)); }
        }
    }
}


async fn receive_msg(reader: &mut BufReader<OwnedReadHalf>, user: &str) -> Result<String, String> {
    let mut buffer = String::with_capacity(MESSAGE_SIZE);
    match reader.read_line(&mut buffer).await {
        Ok(_) => {
            if buffer.is_empty() {
                return Err(String::from(ERROR_MSG));
            }

            let mut msg = format!("{} : {}", user, buffer.as_str());
            if msg.len() > MESSAGE_SIZE {
                msg.truncate(MESSAGE_SIZE - 1);
                msg.push('\n');
            }

            Ok(msg)
        }
        Err(_) => { Err(String::from(ERROR_MSG)) }
    }
}

async fn broadcast(clients: &mut Arc<DashMap<String, OwnedWriteHalf>>, message: Message) {
    for mut entry in clients.iter_mut() {
        //if entry.key() != sender {
        //todo pisanie wiadomosci!!
        write_buf(entry.value_mut(), "ff").await;
        //}
    }
}

async fn handle_connection(reader: &mut BufReader<OwnedReadHalf>, name: &str, sender: Sender<(String, String)>)
{
    loop {
        let msg = receive_msg(reader, name).await;
        match msg {
            Ok(_) => {
                if sender.send((msg.clone().unwrap(), name.parse().unwrap())).await.is_ok() {} else {
                    warn!("Message from {} could not be send to broadcast", &name);
                }
            }
            Err(_) => {
                let err_msg = format!("{} has left the chat", &name);
                if sender.send((err_msg, name.parse().unwrap())).await.is_ok() {} else {
                    warn!("Exit message from {} could not be send to broadcast", &name);
                }
                break;
            }
        }
    }
}

struct Channel {
    sender: Arc<Mutex<Sender<Message>>>,
    receiver: Arc<Mutex<Receiver<Message>>>,
    users: Arc<DashMap<String, OwnedWriteHalf>>,
}

impl Channel {
    pub fn new(sender: Arc<Mutex<Sender<Message>>>,
               receiver: Arc<Mutex<Receiver<Message>>>,
               users: Arc<DashMap<String, OwnedWriteHalf>>) -> Self {
        Channel {
            sender,
            receiver,
            users,
        }
    }
}


async fn manage_client(reader: OwnedReadHalf, mut writer:OwnedWriteHalf, channels: Arc<Mutex<Vec<Channel>>>) {
    let mut reader = BufReader::new(reader); //buffer czyta z socketa tcp od klienta
    //let mut line = String::new();
    //write_buf(&mut writer, "Enter username \n").await;
    let hello = Message::Hello { username: "".to_string(), channel: 0 };
    /*if let Ok(hello) = request_username(&mut reader, &mut writer, &mut clients_mut).await
    {
        clients_mut.insert(name.clone(), writer);
        //todo wydzielic funkcje
        //todo rozważac msg ChangeChannel - wtedy wysyłac mu historie i innym ze dołączył i reszcie że wyszedł
        info!("Client {} has joined", name); //debug msg

        let arrival_msg = format!("User {} has joined. \n", name);
        if sender.send((arrival_msg, name.clone())).is_ok() {} else {
            info!("DEBUG: message from {} could not be broadcast", name);
        }

        handle_connection(&mut reader, &name, sender).await;
        //koniec połączenia
        clients_mut.remove(&name);
        info!("Client {} has left the chat", name);
    }*/
}


async fn manage_communication(channels_cp: Arc<Mutex<Vec<Channel>>>, i: usize){
    loop {
        let mut guard  = channels_cp.lock().await;
        let mut channel = guard.deref_mut().get_mut(i).unwrap();
        let mut receiver_ptr = Arc::clone(&channel.receiver);
        let mut receiver_guard = receiver_ptr.lock().await;
        let receiver = receiver_guard.deref_mut();
        if let Some(msg) = receiver.recv().await {
            info!("Broadcasting message : {}","XD");
            broadcast(&mut channel.users, msg).await;
        }
        drop(guard);
    }
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().init().unwrap();

    let listener = TcpListener::bind(LOCAL).await.unwrap();

    let mut channels: Arc<Mutex<Vec<Channel>>> = Arc::new(Mutex::new(Vec::with_capacity(CHANNEL_COUNT)));

    for _i in 0..CHANNEL_COUNT {
        let (sender, mut receiver) = mpsc::channel::<Message>(MAX_CLIENT_NUM);
        let users: Arc<DashMap<String, OwnedWriteHalf>> = Arc::new(DashMap::with_capacity(MAX_CLIENT_NUM));//klienci
        let sender = Arc::new(Mutex::new(sender));
        let receiver = Arc::new(Mutex::new(receiver));
        let channel = Channel { sender, receiver, users };
        let mut guard = channels.lock().await;
        guard.push(channel);

    }


    for i in 0..CHANNEL_COUNT {
        let mut channels_cp = Arc::clone(&channels);

        tokio::spawn(async move {
            manage_communication(channels_cp, i)
        });
    }


    //nowy klient
    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        info!("Incoming connection from {}", addr);
        let (reader, mut writer) = socket.into_split();

        let mut channels_cp = Arc::clone(&channels);


        info!("Initializing new client. Waiting for username and channel number");

        tokio::spawn(async move {

            manage_client(reader, writer, channels_cp);

        });
        // };
    }/**/
}
