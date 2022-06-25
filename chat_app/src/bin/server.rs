use std::ops::{DerefMut};
use std::sync::{Arc};
use tokio::net::{TcpListener};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Mutex};
use anyhow::{anyhow, Result};
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
const CHANNEL_COUNT: usize = 10;

async fn write_buf<'a>(writer: &mut OwnedWriteHalf, message: &str) -> Result<()> {
    writer.write(message.as_bytes()).await?;
    writer.flush().await?;
    info!("flushed message");
    Ok(())
}


async fn receive_msg(reader: &mut BufReader<OwnedReadHalf>, _user: &str) -> Result<String> {
    let mut buffer = String::with_capacity(MESSAGE_SIZE);
    match reader.read_line(&mut buffer).await {
        Ok(_) => {
            if buffer.is_empty() {
                return Err(anyhow!("Buffer empty"));
            }
            info!("received message from client");
            Ok(buffer)
        }
        Err(_) => {
            Err(anyhow!("Error reading message"))
        }
    }
}

async fn broadcast(clients: &mut Arc<DashMap<String, OwnedWriteHalf>>, message: Message) -> Result<()> {
    for mut entry in clients.iter_mut() {
        write_buf(entry.value_mut(), serde_json::to_string(&message).unwrap().as_str()).await?;
    }
    Ok(())
}

async fn handle_connection(reader: &mut BufReader<OwnedReadHalf>, name: &str, channel_num: usize, channels: Arc<Mutex<Vec<Channel>>>) -> Result<()>
{
    loop {
        let msg = receive_msg(reader, name).await?;

        match serde_json::from_str(msg.as_str())? {
            Message::ClientMessage { message } => {
                let mut guard = channels.lock().await;
                let channel = guard.get_mut(channel_num).unwrap();
                let sender_guard = channel.sender.lock().await;
                let message = Message::BroadcastMessage { message, user: name.to_string() };
                sender_guard.send(message).await?;
                info!("Broadcast client message");
                drop(sender_guard);
                drop(guard);
            }
            Message::Quit => {
                let mut guard = channels.lock().await;
                let channel = guard.get_mut(channel_num).unwrap();
                let sender_guard = channel.sender.lock().await;
                let message = Message::UserQuit { user: name.to_string() };
                sender_guard.send(message).await?;
                drop(sender_guard);
                drop(guard);
                break;
            }
            _ => {
                break;
            }
        }
    }
    Ok(())
}

struct Channel {
    sender: Arc<Mutex<Sender<Message>>>,
    receiver: Arc<Mutex<Receiver<Message>>>,
    users: Arc<DashMap<String, OwnedWriteHalf>>,
}


async fn read_username_and_channel(reader: &mut BufReader<OwnedReadHalf>) -> Result<Message> {
    let mut buffer = String::with_capacity(NAME_SIZE);
    loop {
        buffer.clear();
        reader.read_line(&mut buffer).await.expect("TODO: panic message");

        let message: Message = serde_json::from_str(buffer.as_str())?;

        if let Message::Hello {
            username: _,
            channel
        } = message {
            if channel >= CHANNEL_COUNT
            { return Err(anyhow!("Channel number too big")); }
        } else { return Err(anyhow!("wrong message from user")); };
        return Ok(message);
    }
}

async fn manage_client(reader: OwnedReadHalf, writer: OwnedWriteHalf, channels: Arc<Mutex<Vec<Channel>>>) -> Result<()> {
    let mut reader = BufReader::new(reader); //buffer czyta z socketa tcp od klienta
    if let Ok(Message::Hello { username, channel }) = read_username_and_channel(&mut reader).await
    {
        let mut channels_lock = channels.lock().await;
        let channel_r = channels_lock.deref_mut().get_mut(channel).unwrap();
        channel_r.users.insert(username.clone(), writer);
        info!("Client {} has joined", username);
        let mut serialized = serde_json::to_string(&Message::Ok).unwrap();
        serialized.push('\n');
        channel_r.users.get_mut(username.as_str()).unwrap().value_mut().write(serialized.as_bytes()).await?;
        let _ = channel_r.users.get_mut(username.as_str()).unwrap().value_mut().flush().await;
        broadcast(&mut channel_r.users, Message::UserJoined { user: username.clone() }).await?;
        drop(channels_lock);

        info!("Broadcast user joined");
        handle_connection(&mut reader, &username, channel, Arc::clone(&channels)).await?;

        //koniec połączenia
        //channel_r.users.remove(username.as_str());
        info!("Client {} has left the chat", username.as_str());
    } else { warn!("Wrong message from client") }/**/
    Ok(())
}


async fn manage_communication(channels_cp: Arc<Mutex<Vec<Channel>>>, i: usize) -> Result<()> {
    loop {
        let mut guard = channels_cp.lock().await;
        let receiver_ptr = Arc::clone(&guard.deref_mut().get_mut(i).unwrap().receiver);
        let mut receiver_guard = receiver_ptr.lock().await;
        let receiver = receiver_guard.deref_mut();
        drop(guard);
        if let Some(msg) = receiver.recv().await {
            info!("Broadcasting message to channel {}",i);
            broadcast(&mut channels_cp.lock().await.deref_mut().get_mut(i).unwrap().users, msg).await?;
        }
        drop(receiver_guard);
    }
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().init().unwrap();

    let listener = TcpListener::bind(LOCAL).await.unwrap();

    let channels: Arc<Mutex<Vec<Channel>>> = Arc::new(Mutex::new(Vec::with_capacity(CHANNEL_COUNT)));

    for _i in 0..CHANNEL_COUNT {
        let (sender, receiver) = mpsc::channel::<Message>(MAX_CLIENT_NUM);
        let users: Arc<DashMap<String, OwnedWriteHalf>> = Arc::new(DashMap::with_capacity(MAX_CLIENT_NUM));
        let sender = Arc::new(Mutex::new(sender));
        let receiver = Arc::new(Mutex::new(receiver));
        let channel = Channel { sender, receiver, users };
        let mut guard = channels.lock().await;
        guard.push(channel);
        drop(guard);
    }


    for i in 0..CHANNEL_COUNT {
        let channels_cp = Arc::clone(&channels);

        tokio::spawn(async move {
            manage_communication(channels_cp, i).await
        });
    }


    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        info!("Incoming connection from {}", addr);
        let (reader, writer) = socket.into_split();

        let channels_cp = Arc::clone(&channels);


        info!("Initializing new client. Waiting for username and channel number");

        tokio::spawn(async move {
            manage_client(reader, writer, channels_cp).await.unwrap();
        });
    }
}
