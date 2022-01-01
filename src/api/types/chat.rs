use deltachat::chat::get_chat_contacts;
use deltachat::chat::{Chat, ChatId};
use deltachat::constants::*;
use deltachat::contact::Contact;
use deltachat::context::Context;

use num_traits::cast::ToPrimitive;

use anyhow::{anyhow, Result};
use jsonrpc_core::serde_json::Value;
use serde::Serialize;

use super::color_int_to_hex_string;
use super::contact::ContactObject;
use super::return_type::*;
use crate::custom_return_type;

#[derive(Serialize)]
pub struct FullChat {
    id: u32,
    name: String,
    is_protected: bool,
    profile_image: Option<String>, //BLOBS ?
    archived: bool,
    // subtitle  - will be moved to frontend because it uses translation functions
    chat_type: u32,
    is_unpromoted: bool,
    is_self_talk: bool,
    contacts: Vec<ContactObject>,
    contact_ids: Vec<u32>,
    color: String,
    fresh_message_counter: usize,
    // is_group - please check over chat.type in frontend instead
    is_contact_request: bool,
    is_device_chat: bool,
    self_in_group: bool,
    is_muted: bool,
    ephemeral_timer: u32, //TODO look if there are more important properties in newer core versions
}

impl FullChat {
    pub async fn from_dc_chat_id(chat_id: u32, context: &Context) -> Result<Self> {
        let rust_chat_id = ChatId::new(chat_id);
        let chat = Chat::load_from_db(&context, rust_chat_id).await?;

        let contact_ids = get_chat_contacts(context, rust_chat_id).await?;

        let mut contacts = Vec::new();

        for contact_id in &contact_ids {
            contacts.push(
                ContactObject::from_dc_contact(
                    Contact::load_from_db(context, *contact_id).await?,
                    context,
                )
                .await?,
            )
        }

        let profile_image = match chat.get_profile_image(context).await? {
            Some(path_buf) => path_buf.to_str().map(|s| s.to_owned()),
            None => None,
        };

        let color = color_int_to_hex_string(chat.get_color(context).await?);
        let fresh_message_counter = rust_chat_id.get_fresh_msg_cnt(context).await?;
        let ephemeral_timer = rust_chat_id.get_ephemeral_timer(context).await?.to_u32();

        Ok(FullChat {
            id: chat_id,
            name: chat.name.clone(),
            is_protected: chat.is_protected(),
            profile_image, //BLOBS ?
            archived: chat.get_visibility() == deltachat::chat::ChatVisibility::Archived,
            chat_type: chat
                .get_type()
                .to_u32()
                .ok_or_else(|| anyhow!("unknown chat type id"))?, // TODO get rid of this unwrap?
            is_unpromoted: chat.is_unpromoted(),
            is_self_talk: chat.is_self_talk(),
            contacts,
            contact_ids: contact_ids.clone(),
            color,
            fresh_message_counter,
            is_contact_request: chat.is_contact_request(),
            is_device_chat: chat.is_device_talk(),
            self_in_group: contact_ids.contains(&DC_CONTACT_ID_SELF),
            is_muted: chat.is_muted(),
            ephemeral_timer,
        })
    }
}

impl ReturnType for FullChat {
    custom_return_type!("FullChat_Type".to_owned());

    fn get_typescript_type() -> String {
        r#"
        {
            id: number,
            name: string,
            is_protected: boolean,
            profile_image: string,
            archived: boolean,
            chat_type: number,
            is_unpromoted: boolean,
            is_self_talk: boolean,
            contacts: Contact_Type[],
            contact_ids: number[],
            color: string,
            fresh_message_counter: number,
            is_group: boolean,
            is_contact_request: boolean,
            is_device_chat: boolean,
            self_in_group: boolean,
            is_muted: boolean,
            ephemeral_timer: number, 
        }
        "#
        .to_owned()
    }

    fn into_json_value(self) -> Value {
        jsonrpc_core::serde_json::to_value(self).unwrap() // todo: can we somehow get rid of that unwrap here?
    }
}
