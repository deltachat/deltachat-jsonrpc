use deltachat::contact::VerifiedStatus;
use deltachat::context::Context;

use anyhow::Result;
use jsonrpc_core::serde_json::Value;

use serde::Serialize;

use crate::custom_return_type;

use super::{color_int_to_hex_string, return_type::*};

#[derive(Serialize)]
pub struct ContactObject {
    address: String,
    color: String,
    auth_name: String,
    status: String,
    display_name: String,
    id: u32,
    name: String,
    profile_image: Option<String>, //BLOBS
    name_and_addr: String,
    is_blocked: bool,
    is_verified: bool,
}

impl ContactObject {
    pub async fn from_dc_contact(
        contact: deltachat::contact::Contact,
        context: &Context,
    ) -> Result<Self> {
        Ok(ContactObject {
            address: contact.get_addr().to_owned(),
            color: color_int_to_hex_string(contact.get_color()),
            auth_name: contact.get_authname().to_owned(),
            status: contact.get_status().to_owned(),
            display_name: contact.get_display_name().to_owned(),
            id: contact.id,
            name: contact.get_name().to_owned(),
            profile_image: match contact.get_profile_image(context).await? {
                Some(path_buf) => path_buf.to_str().map(|s| s.to_owned()),
                None => None,
            }, //BLOBS
            name_and_addr: contact.get_name_n_addr().to_owned(),
            is_blocked: contact.is_blocked(),
            is_verified: contact.is_verified(context).await? == VerifiedStatus::BidirectVerified,
        })
    }
}

impl ReturnType for ContactObject {
    custom_return_type!("Contact_Type".to_owned());

    fn get_typescript_type() -> String {
        r#"
        {
            address: string,
            color: string,
            auth_name: string,
            status: string,
            display_name: string,
            id: number,
            name: string,
            profile_image: string,
            name_and_addr: string,
            is_blocked: boolean,
            is_verified: boolean,
        }
        "#
        .to_owned()
    }

    fn into_json_value(self) -> Value {
        jsonrpc_core::serde_json::to_value(self).unwrap() // todo: can we somehow get rid of that unwrap here?
    }
}
