use crate::error::{ClientError, ConnectError};
use crate::MqttOptions;
use crossbeam_channel;
use futures::{sync::mpsc, Future, Sink};
use mqtt311::{PacketIdentifier, Publish, QoS, Subscribe, Unsubscribe, SubscribeTopic};
use std::sync::Arc;

pub mod connection;
pub mod mqttstate;
pub mod network;
pub mod prepend;

#[derive(Debug)]
pub enum Notification {
    Publish(Publish),
    PubAck(PacketIdentifier),
    PubRec(PacketIdentifier),
    PubRel(PacketIdentifier),
    PubComp(PacketIdentifier),
    SubAck(PacketIdentifier),
    None,
}

/// Requests to network event loop
#[derive(Debug)]
pub enum Request {
    Publish(Publish),
    Subscribe(Subscribe),
    Unsubscribe(Unsubscribe),
    PubAck(PacketIdentifier),
    PubRec(PacketIdentifier),
    PubRel(PacketIdentifier),
    PubComp(PacketIdentifier),
    Ping,
    Reconnect(MqttOptions),
    Disconnect,
    None,
}

#[derive(Debug)]
pub enum Command {
    Pause,
    Resume,
}

pub struct UserHandle {
    request_tx: mpsc::Sender<Request>,
    command_tx: mpsc::Sender<Command>,
    notification_rx: crossbeam_channel::Receiver<Notification>,
}

#[derive(Clone)]
pub struct MqttClient {
    request_tx: mpsc::Sender<Request>,
    command_tx: mpsc::Sender<Command>,
    max_packet_size: usize,
}

impl MqttClient {
    pub fn start(opts: MqttOptions) -> Result<(Self, crossbeam_channel::Receiver<Notification>), ConnectError> {
        let max_packet_size = opts.max_packet_size();
        let UserHandle {
            request_tx,
            command_tx,
            notification_rx,
        } = connection::Connection::run(opts)?;

        let client = MqttClient {
            request_tx,
            command_tx,
            max_packet_size,
        };

        Ok((client, notification_rx))
    }

    //    pub fn proxy_start(opts: MqttOptions, ) -> Result<(Self, crossbeam_channel::Receiver<Notification>), ConnectError> {
    //
    //    }

    pub fn publish<S, V>(&mut self, topic: S, qos: QoS, payload: V) -> Result<(), ClientError>
    where
        S: Into<String>,
        V: Into<Vec<u8>>,
    {
        let payload = payload.into();
        if payload.len() > self.max_packet_size {
            return Err(ClientError::PacketSizeLimitExceeded);
        }

        let publish = Publish {
            dup: false,
            qos,
            retain: false,
            topic_name: topic.into(),
            pkid: None,
            payload: Arc::new(payload),
        };

        let tx = &mut self.request_tx;
        tx.send(Request::Publish(publish)).wait()?;
        Ok(())
    }

    pub fn subscribe<S>(&mut self, topic: S, qos: QoS) -> Result<(), ClientError>
    where
        S: Into<String>,
    {
        let topic = SubscribeTopic {
            topic_path: topic.into(),
            qos,
        };
        let subscribe = Subscribe {
            pkid: PacketIdentifier::zero(),
            topics: vec![topic],
        };

        let tx = &mut self.request_tx;
        tx.send(Request::Subscribe(subscribe)).wait()?;
        Ok(())
    }

    pub fn unsubscribe<S>(&mut self, topic: S) -> Result<(), ClientError>
        where
            S: Into<String>,
    {
        let unsubscribe = Unsubscribe {
            pkid: PacketIdentifier::zero(),
            topics: vec![topic.into()],
        };

        let tx = &mut self.request_tx;
        tx.send(Request::Unsubscribe(unsubscribe)).wait()?;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), ClientError> {
        let tx = &mut self.command_tx;
        tx.send(Command::Pause).wait()?;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), ClientError> {
        let tx = &mut self.command_tx;
        tx.send(Command::Resume).wait()?;
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), ClientError> {
        let tx = &mut self.request_tx;
        tx.send(Request::Disconnect).wait()?;
        Ok(())
    }
}

// use std::fmt;

// impl fmt::Debug for Request {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         match self {
//             Request::Publish(p) => write!(f,
//                 "Publish \
//                  topic = {}, \
//                  pkid = {:?}, \
//                  payload size = {:?} bytes",
//                 p.topic_name,
//                 p.pkid,
//                 p.payload.len()
//             ),

//             _ => write!(f, "{:?}", self),
//         }
//     }
// }
