mod api;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::bail;
use api::events::event_to_json_rpc_notification;
use api::{AccountsWrapper, CommandApi};

use async_std::path::PathBuf;
use async_std::sync::RwLock;
use async_std::{prelude::*, task};
use deltachat::accounts::Accounts;
use tide::Request;
use tide_websockets::{Message, WebSocket, WebSocketConnection};

use log::{debug, error, info, warn};

struct EventShare {
    pub(crate) connection_id: usize,
    pub(crate) event_subscribers: HashMap<usize, WebSocketConnection>,
}

impl EventShare {
    fn new() -> Self {
        EventShare {
            connection_id: 0,
            event_subscribers: HashMap::new(),
        }
    }

    pub(crate) fn subscribe_events(&mut self, stream: WebSocketConnection) -> usize {
        self.connection_id += 1;
        let connection_id = self.connection_id;
        self.event_subscribers.insert(connection_id, stream);
        connection_id
    }

    pub(crate) fn unsubscribe_events(&mut self, connection_id: usize) -> anyhow::Result<()> {
        if self.event_subscribers.remove(&connection_id).is_none() {
            warn!("removing of connection failed {}", connection_id);
            bail!("removing of connection failed {}", connection_id);
        }
        Ok(())
    }

    pub(crate) async fn send_event(&mut self, message: String) {
        for (connection_id, stream) in self.event_subscribers.iter() {
            if let Err(err) = stream.send_string(message.clone()).await {
                error!(
                    "could not send event to connection {}: {}",
                    connection_id, err
                );
            }
        }
    }
}

/// The shared application state.
#[derive(Clone)]
struct State {
    pub(crate) cmd_api: CommandApi,
    event_share: Arc<RwLock<EventShare>>,
}

impl State {
    pub(crate) async fn subscribe_events(&self, stream: WebSocketConnection) -> usize {
        self.event_share.write().await.subscribe_events(stream)
    }

    pub(crate) async fn unsubscribe_events(&self, connection_id: usize) -> anyhow::Result<()> {
        self.event_share
            .write()
            .await
            .unsubscribe_events(connection_id)
    }

    pub(crate) async fn init_event_share(&self, manager: AccountsWrapper) {
        let mut em = manager.read().await.get_event_emitter().await;
        let es = self.event_share.clone();
        task::spawn(async move {
            loop {
                let event = em.recv().await;
                match event {
                    Ok(event) => {
                        if let Some(event) = event {
                            // 1. translate event
                            let event_string = event_to_json_rpc_notification(event).to_string();
                            // 2.sendEvent to all listeners
                            es.write().await.send_event(event_string).await;
                        }
                    }
                    Err(err) => error!("Error receiving event: {} ", err),
                }
            }
        });
    }
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::init();
    info!("Starting");

    // Setup Account Manager / start it

    let account_manager = AccountsWrapper {
        inner: Arc::new(RwLock::new(
            Accounts::new("json_api".to_owned(), PathBuf::from("./accounts"))
                .await
                .unwrap(),
        )),
    };

    let state = State {
        cmd_api: CommandApi::new(account_manager.clone()),
        event_share: Arc::new(RwLock::new(EventShare::new())),
    };

    state.init_event_share(account_manager.clone()).await;

    let mut app = tide::with_state(state);

    app.at("/api_ws").get(WebSocket::new(
        |request: Request<State>, mut stream| async move {
            let io = request.state().cmd_api.get_json_rpc_io();

            debug!("connection opened");

            let mut open_tasks = Vec::new();

            let subscription: usize = request.state().subscribe_events(stream.clone()).await;

            while let Some(Ok(Message::Text(input))) = stream.next().await {
                // debug!("in: {}", input);
                let task = io.handle_request(&input);

                let stream_clone = stream.clone();
                open_tasks.push(task::spawn(async move {
                    if let Some(result) = task.await {
                        debug!("sending answer");

                        if let Err(err) = stream_clone.send_string(result).await {
                            error!("could not send answer, error: {}", err);
                        }
                        debug!("sending answer: done");
                    }
                }));
            }

            debug!("connection closed, awaiting open tasks");

            for open in open_tasks {
                open.await;
            }

            debug!("done with awaiting open tasks, droping connection");

            request.state().unsubscribe_events(subscription).await?;

            Ok(())
        },
    ));

    account_manager.read().await.start_io().await;

    app.listen("127.0.0.1:8080").await?;

    Ok(())
}
