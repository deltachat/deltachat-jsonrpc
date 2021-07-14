// use dc_cmd_derive::make_answer;

use async_std::path::PathBuf;
use deltachat::accounts::Accounts;
use tempfile::TempDir;

mod api;

use api::CommandApi;

#[async_std::main]
async fn main() {
    println!("Hello, world!");

    if let Err(err) = real_main().await {
        println!("Error: {:?}", err);
    }
}

async fn real_main() -> anyhow::Result<()> {
    println!("{}", "");
    let tmp_dir = TempDir::new().unwrap().path().into();

    println!("tmp_dir: {:?}", tmp_dir);

    // PathBuf::from("./accounts")
    let mut account_manager = Accounts::new("".to_string(), tmp_dir).await?;

    let mut cmd_api = CommandApi::new(&account_manager);

    let mut io = cmd_api.get_json_rpc_io();

    let request = r#"{"jsonrpc":"2.0","method":"add_account","id":1}"#;
    let response = r#"{"jsonrpc":"2.0","result":1,"id":1}"#;
    let result = io.handle_request_sync(request);

    println!("{:?}", result);
    assert_eq!(result, Some(response.to_owned()));

    let request = r#"{"jsonrpc":"2.0","method":"get_all_account_ids","id":1}"#;
    let response = r#"{"jsonrpc":"2.0","result":[1],"id":1}"#;
    let result = io.handle_request_sync(request);

    println!("{:?}", result);
    assert_eq!(result, Some(response.to_owned()));

    // let request =
    //     r#"{"jsonrpc": "2.0", "method": "say_hello", "params": { "name": "world" }, "id": 1}"#;
    // let response = r#"{"jsonrpc":"2.0", "result":"hello, world", "id":1 }"#;

    // assert_eq!(io.handle_request_sync(request), Some(response.to_owned()));

    println!("TS:\n{}", CommandApi::get_typescript());

    Ok(())
}
