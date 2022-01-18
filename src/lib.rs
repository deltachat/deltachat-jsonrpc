pub mod api;

pub use api::events;

// #[cfg(test)]
// mod tests {
//     use super::api::{AccountsWrapper, CommandApi};
//     use async_std::sync::{Arc, RwLock};
//     use deltachat::accounts::Accounts;
//     use tempfile::TempDir;

//     #[async_std::test]
//     async fn basic_json_rpc_functionality() -> anyhow::Result<()> {
//         // println!("{}", "");
//         let tmp_dir = TempDir::new().unwrap().path().into();

//         println!("tmp_dir: {:?}", tmp_dir);

//         // PathBuf::from("./accounts")
//         let account_manager = AccountsWrapper {
//             inner: Arc::new(RwLock::new(Accounts::new(tmp_dir).await?)),
//         };

//         let cmd_api = CommandApi::new(account_manager);

//         let io = cmd_api.get_json_rpc_io();

//         let request = r#"{"jsonrpc":"2.0","method":"add_account","id":1}"#;
//         let response = r#"{"jsonrpc":"2.0","result":1,"id":1}"#;
//         let result = io.handle_request_sync(request);

//         println!("{:?}", result);
//         assert_eq!(result, Some(response.to_owned()));

//         let request = r#"{"jsonrpc":"2.0","method":"get_all_account_ids","id":1}"#;
//         let response = r#"{"jsonrpc":"2.0","result":[1],"id":1}"#;
//         let result = io.handle_request_sync(request);

//         println!("{:?}", result);
//         assert_eq!(result, Some(response.to_owned()));

//         // let request =
//         //     r#"{"jsonrpc": "2.0", "method": "say_hello", "params": { "name": "world" }, "id": 1}"#;
//         // let response = r#"{"jsonrpc":"2.0", "result":"hello, world", "id":1 }"#;

//         // assert_eq!(io.handle_request_sync(request), Some(response.to_owned()));

//         Ok(())
//     }
// }
