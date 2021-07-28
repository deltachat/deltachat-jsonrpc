#[allow(dead_code)]
mod api;

use api::CommandApi;

#[async_std::main]
async fn main() {
    println!("{}", CommandApi::get_typescript());
}
