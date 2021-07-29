mod api;

use std::sync::Arc;

use api::CommandApi;

use async_std::{path::PathBuf};
use async_std::{prelude::*, task};
use deltachat::accounts::Accounts;
use jsonrpc_core::request;
use tide::Request;
use tide_websockets::{Message, WebSocket};

use log::{debug, error, info};

/// The shared application state.
#[derive(Clone)]
struct State {
    pub(crate) cmd_api: CommandApi,
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::init();

    // Setup Account Manager / start it

    let account_manager = Accounts::new("json_api".to_owned(), PathBuf::from("./accounts"))
        .await
        .unwrap();

    let state = State {
        cmd_api: CommandApi::new(&account_manager),
    };

    let mut app = tide::with_state(state);
    // let handler_arc: Arc<RwLock<jsonrpc_core::IoHandler>> = Arc::new(RwLock::new(cmd_api.get_json_rpc_io()));

    // let handler_arc_r: &'static Arc<RwLock<jsonrpc_core::IoHandler>> = &handler_arc;

    app.at("/events")
        .get(WebSocket::new(|_request, mut stream| async move {
            // TODO
            while let Some(Ok(Message::Text(input))) = stream.next().await {
                let output: String = input.chars().rev().collect();

                stream
                    .send_string(format!("{} | {}", &input, &output))
                    .await?;
            }

            Ok(())
        }));

    app.at("/api_ws").get(WebSocket::new(
        |request: Request<State>, mut stream| async move {
            let io = request.state().cmd_api.get_json_rpc_io();

            debug!("connection openened");

            let mut open_tasks = Vec::new();

            while let Some(Ok(Message::Text(input))) = stream.next().await  {
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

            Ok(())
        },
    ));

    app.listen("127.0.0.1:8080").await?;

    Ok(())
}
