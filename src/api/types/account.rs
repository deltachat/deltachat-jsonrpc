use deltachat::config::Config;
use deltachat::constants::*;
use deltachat::contact::Contact;

use anyhow::Result;
use jsonrpc_core::serde_json::Value;

use super::color_int_to_hex_string;
use super::return_type::*;

use serde::Serialize;
use ts_rs::TS;

#[derive(Serialize, TS)]
#[serde(tag = "type")]
pub enum Account {
    //#[serde(rename_all = "camelCase")]
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
    crate::ts_rs_return_type!();
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
