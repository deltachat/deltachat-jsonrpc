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
use yerpc_tide::yerpc_handler;

use log::{debug, error, info, warn};

/// The shared application state.
#[derive(Clone)]
struct State {
    pub(crate) cmd_api: CommandApi,
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::init();
    info!("Starting");

    // Setup Account Manager / start it

    let account_manager = AccountsWrapper {
        inner: Arc::new(RwLock::new(
            Accounts::new(PathBuf::from("./accounts")).await.unwrap(),
        )),
    };

    let state = State {
        cmd_api: CommandApi::new(account_manager.clone()),
    };

    // state.init_event_share(account_manager.clone()).await;

    let mut app = tide::with_state(state);

    app.at("/api_ws")
        .get(yerpc_handler(|request: Request<State>, rpc| {
            let state = request.state();
            let events = state.cmd_api.manager.read().await.get_event_emitter().await;
            task::spawn({
                async move {
                    while let Some(event) = events.next().await {
                        let event = event_to_json_rpc_notification(event);
                        rpc.notify("onevent", event).await?;
                    }
                    Ok(())
                }
            });
            state.cmd_api
        }));
    account_manager.read().await.start_io().await;

    app.listen("127.0.0.1:20808").await?;

    Ok(())
}
