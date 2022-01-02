use deltachat::contact::VerifiedStatus;
use deltachat::context::Context;

use anyhow::Result;
use jsonrpc_core::serde_json::Value;

use super::{color_int_to_hex_string, return_type::*};
use serde::Serialize;
use ts_rs::TS;

#[derive(Serialize, TS)]
#[serde(rename = "Contact")]
pub struct ContactObject {
    address: String,
    color: String,
    auth_name: String,
    status: String,
    display_name: String,
    id: u32,
    name: String,
    profile_image: Option<String>, // BLOBS
    name_and_addr: String,
    is_blocked: bool,
    is_verified: bool,
}

impl ReturnType for ContactObject {
    crate::ts_rs_return_type!();
}

impl ContactObject {
    pub async fn from_dc_contact(
        contact: deltachat::contact::Contact,
        context: &Context,
    ) -> Result<Self> {
        let profile_image = match contact.get_profile_image(context).await? {
            Some(path_buf) => path_buf.to_str().map(|s| s.to_owned()),
            None => None,
        };
        let is_verified = contact.is_verified(context).await? == VerifiedStatus::BidirectVerified;

        Ok(ContactObject {
            address: contact.get_addr().to_owned(),
            color: color_int_to_hex_string(contact.get_color()),
            auth_name: contact.get_authname().to_owned(),
            status: contact.get_status().to_owned(),
            display_name: contact.get_display_name().to_owned(),
            id: contact.id,
            name: contact.get_name().to_owned(),
            profile_image, //BLOBS
            name_and_addr: contact.get_name_n_addr().to_owned(),
            is_blocked: contact.is_blocked(),
            is_verified,
        })
    }
}
