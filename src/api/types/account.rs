use deltachat::config::Config;
use deltachat::constants::*;
use deltachat::contact::Contact;

use anyhow::Result;
use jsonrpc_core::serde_json::{json, Value};

use super::color_int_to_hex_string;
use super::return_type::*;
use crate::custom_return_type;

pub enum Account {
    Configured {
        id: u32,
        display_name: Option<String>,
        addr: Option<String>,
        // size: u32,
        profile_image: Option<String>, // TODO: This needs to be converted to work with blob http server.
        color: String,
    },
    Unconfigured {
        id: u32,
    },
}

impl ReturnType for Account {
    fn get_typescript_type() -> String {
        "{ id: number, type: \"unconfigured\" } | { id: number, type: \"configured\", display_name: string | null, addr: string | null, profile_image: string | null, color: string }".to_owned()
    }

    fn into_json_value(self) -> Value {
        match self {
            Account::Unconfigured { id } => json!({ "id": id, "type": "unconfigured" }),
            Account::Configured {
                id,
                display_name,
                addr,
                profile_image,
                color,
            } => json!({
               "id": id,
               "type": "configured",
               "display_name": display_name,
               "addr": addr,
               "profile_image": profile_image,
               "color": color
            }),
        }
    }

    custom_return_type!("Account_Type".to_owned());
}

impl Account {
    pub async fn from_context(id: u32, ctx: &deltachat::context::Context) -> Result<Self> {
        if ctx.is_configured().await? {
            let display_name = ctx.get_config(Config::Displayname).await?;
            let addr = ctx.get_config(Config::Addr).await?;
            let profile_image = ctx.get_config(Config::Selfavatar).await?;
            let color = color_int_to_hex_string(
                Contact::get_by_id(&ctx, DC_CONTACT_ID_SELF)
                    .await?
                    .get_color(),
            );
            Ok(Account::Configured {
                id,
                display_name,
                addr,
                profile_image,
                color,
            })
        } else {
            Ok(Account::Unconfigured { id })
        }
    }
}
