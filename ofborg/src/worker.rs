use amqp::Basic;
use amqp::{Consumer, Channel};
use amqp::protocol::basic::{Deliver,BasicProperties};
use std::marker::Send;
use serde::Serialize;
use serde_json;

pub struct Worker<T: SimpleWorker<M>>
    where M: Serialize {
    internal: T
}

pub struct Response {
}

pub type Actions<T> = Vec<Action<T>>;

#[derive(Debug)]
pub enum Action<M: ?Sized>
    where M: Serialize {
    Ack,
    NackRequeue,
    NackDump,
    PublishJSON(QueueMsgJSON<M>),
    Publish(QueueMsg),
}


pub struct QueueMsgJSON<M: ?Sized>
    where M: Serialize {
    pub exchange: Option<String>,
    pub routing_key: Option<String>,
    pub content: M,
}

#[derive(Debug)]
pub struct QueueMsg {
    pub exchange: Option<String>,
    pub routing_key: Option<String>,
    pub mandatory: bool,
    pub immediate: bool,
    pub properties: Option<BasicProperties>,
    pub content: Vec<u8>,
}

pub fn publish_serde_action<T: ?Sized>(exchange: Option<String>, routing_key: Option<String>, msg: &T) -> Action<T>
    where
     T: Serialize {
    let props = BasicProperties {
        content_type: Some("application/json".to_owned()),
        ..Default::default()
    };

    return Action::Publish(QueueMsg{
        exchange: exchange,
        routing_key: routing_key,
        mandatory: true,
        immediate: false,
        properties: Some(props),
        content: serde_json::to_string(&msg).unwrap().into_bytes()
    });
}

pub trait SimpleWorker<M> {
    type J;

    fn consumer(&self, job: &Self::J) -> Actions<M>;

    fn msg_to_job(&self, method: &Deliver, headers: &BasicProperties,
                  body: &Vec<u8>) -> Result<Self::J, String>;
}

pub fn new<T: SimpleWorker<Sized>>(worker: T) -> Worker<T> {
    return Worker{
        internal: worker,
    };
}



impl <T: SimpleWorker<Sized> + Send> Consumer for Worker<T> {
    fn handle_delivery(&mut self,
                       channel: &mut Channel,
                       method: Deliver,
                       headers: BasicProperties,
                       body: Vec<u8>) {



        let job = self.internal.msg_to_job(&method, &headers, &body).unwrap();
        for action in self.internal.consumer(&job) {
            match action {
                Action::Ack => {
                    channel.basic_ack(method.delivery_tag, false).unwrap();
                }
                Action::NackRequeue => {
                    channel.basic_nack(method.delivery_tag, false, true).unwrap();
                }
                Action::NackDump => {
                    channel.basic_nack(method.delivery_tag, false, false).unwrap();
                }
                Action::Publish(msg) => {
                    let exch = msg.exchange.clone().unwrap_or("".to_owned());
                    let key = msg.routing_key.clone().unwrap_or("".to_owned());

                    let props = msg.properties.unwrap_or(BasicProperties{ ..Default::default()});
                    channel.basic_publish(
                        exch,
                        key,
                        msg.mandatory,
                        msg.immediate,
                        props,
                        msg.content
                    ).unwrap();
                }
            }
        }
    }
}
