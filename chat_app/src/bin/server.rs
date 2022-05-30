use std::sync::Arc;
use tokio::net::{TcpListener};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, };
use tokio::sync::broadcast;

use dashmap::DashMap;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::broadcast::Sender;


const LOCAL: &str = "localhost:8080";
const MAX_CLIENT_NUM: usize = 20;
const NAME_SIZE: usize = 30;
const MESSAGE_SIZE: usize = 256;
const ERROR_MSG: &str = "**************ERROR_MSG*************\n";


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

async fn broadcast(clients: &mut Arc<DashMap<String, OwnedWriteHalf>>, message: &str, sender:&str) {
    for mut entry in clients.iter_mut() {
        if entry.key()!=sender{
            write_buf(entry.value_mut(), message).await;
        }
    }
}

async fn handle_connection(reader: &mut BufReader<OwnedReadHalf>, name: &str, sender: Sender<(String, String)>)
{
    loop {
        let msg = receive_msg(reader, name).await;
        match msg {
            Ok(_) => {
                if sender.send((msg.clone().unwrap(), name.parse().unwrap())).is_ok() {} else {
                    println!("DEBUG: Message from {} could not be send to broadcast", &name);
                }
            }
            Err(_) => {
                let err_msg = format!("{} has left the chat", &name);
                if sender.send((err_msg, name.parse().unwrap())).is_ok() {} else {
                    println!("DEBUG: Exit message from {} could not be send to broadcast", &name);
                }
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind(LOCAL).await.unwrap();

    let clients: Arc<DashMap<String, OwnedWriteHalf>> = Arc::new(DashMap::with_capacity(MAX_CLIENT_NUM));//klienci

    let (sender, mut receiver) = broadcast::channel::<(String,String) >(10); //sending strings to max 10 ppl who can connect
    let mut clients_cp = Arc::clone(&clients);

    //task do którego inne wątki wysyłają wiadomości od swoich klientów -
    //robi broadcast do wszystkich klientów
    tokio::spawn(async move {
        loop {
            if let Ok(msg) = receiver.recv().await {
                println!("Broadcasting message : {}", msg.0);
                broadcast(&mut clients_cp, &msg.0, &msg.1).await;
            }
        }
    });


    //nowy klient
    loop {
        let ( socket, addr) = listener.accept().await.unwrap();
        let sender = sender.clone();//klonowanie tx dla kazdego klienta
        //let mut receiver = sender.subscribe();//nowy rx dla każdego klienta
        println!("Incoming connection from {}", addr);
        let (reader, mut writer) = socket.into_split(); //podział socketa na czytanie i pisanie

        let mut clients_mut = clients.clone();
        let len = clients_mut.len();
        if len >= MAX_CLIENT_NUM {
            println!("Refusing the connection. Chat is full");
            let refuse_msg = format!("Chat is full. ({}/{}). Try again later!", MAX_CLIENT_NUM, MAX_CLIENT_NUM);
            writer.write(refuse_msg.as_bytes()).await.expect("Error on write to client!");
        } else {
            println!("Initializing client no. {}. Requesting username", len);

            tokio::spawn(async move { //spawnowanie taska obsługi klienta

                let mut reader = BufReader::new(reader); //buffer czyta z socketa tcp od klienta
                //let mut line = String::new();
                write_buf(&mut writer, "Enter username \n").await;
                if let  Ok(name) = request_username(&mut reader, &mut writer, &mut clients_mut).await
                {
                    clients_mut.insert(name.clone(), writer);
                    println!("Client {} has joined", name); //debug msg

                    let arrival_msg = format!("User {} has joined. \n", name);
                    if sender.send((arrival_msg, name.clone())).is_ok(){}
                    else{
                        println!("DEBUG: message from {} could not be broadcast", name);
                    }

                    handle_connection(&mut reader, &name, sender).await;
                    //koniec połączenia
                    clients_mut.remove(&name);
                    println!("Client {} has left the chat", name);

                }
            });
        };
    }
}
