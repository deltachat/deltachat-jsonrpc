use deltachat::contact::Contact;
use deltachat::context::Context;
use deltachat::message::Message;
use deltachat::message::MsgId;
use num_traits::cast::ToPrimitive;

use anyhow::{anyhow, Result};
use jsonrpc_core::serde_json::Value;
use serde::Serialize;

use crate::custom_return_type;

use super::contact::ContactObject;
use super::return_type::*;

#[derive(Serialize)]
pub struct MessageObject {
    id: u32,
    chat_id: u32,
    from_id: u32,
    quoted_text: Option<String>,
    quoted_message_id: Option<u32>,
    text: Option<String>,
    has_location: bool,
    has_html: bool,
    view_type: u32,
    state: u32,

    timestamp: i64,
    sort_timestamp: i64,
    received_timestamp: i64,
    has_deviating_timestamp: bool,

    // summary - use/create another function if you need it
    subject: String,
    show_padlock: bool,
    is_setupmessage: bool,
    is_info: bool,
    is_forwarded: bool,

    duration: i32,
    dimensions_height: i32,
    dimensions_width: i32,

    videochat_type: Option<u32>,
    videochat_url: Option<String>,

    override_sender_name: Option<String>,
    sender: ContactObject,

    setup_code_begin: Option<String>,

    file: Option<String>,
    file_mime: Option<String>,
    file_bytes: u64,
    file_name: Option<String>,
}

impl MessageObject {
    pub async fn from_message_id(message_id: u32, context: &Context) -> Result<Self> {
        let msg_id = MsgId::new(message_id);
        let message = Message::load_from_db(context, msg_id).await?;

        let quoted_message_id = message
            .quoted_message(context)
            .await?
            .map(|m| m.get_id().to_u32());

        let sender_contact = Contact::load_from_db(context, message.get_from_id()).await?;
        let sender = ContactObject::from_dc_contact(sender_contact, context).await?;
        let file_bytes = message.get_filebytes(context).await;
        let override_sender_name = message.get_override_sender_name();

        Ok(MessageObject {
            id: message_id,
            chat_id: message.get_chat_id().to_u32(),
            from_id: message.get_from_id(),
            quoted_text: message.quoted_text(),
            quoted_message_id,
            text: message.get_text(),
            has_location: message.has_location(),
            has_html: message.has_html(),
            view_type: message
                .get_viewtype()
                .to_u32()
                .ok_or_else(|| anyhow!("viewtype conversion to number failed"))?,
            state: message
                .get_state()
                .to_u32()
                .ok_or_else(|| anyhow!("state conversion to number failed"))?,

            timestamp: message.get_timestamp(),
            sort_timestamp: message.get_sort_timestamp(),
            received_timestamp: message.get_received_timestamp(),
            has_deviating_timestamp: message.has_deviating_timestamp(),

            subject: message.get_subject().to_owned(),
            show_padlock: message.get_showpadlock(),
            is_setupmessage: message.is_setupmessage(),
            is_info: message.is_info(),
            is_forwarded: message.is_forwarded(),

            duration: message.get_duration(),
            dimensions_height: message.get_height(),
            dimensions_width: message.get_width(),

            videochat_type: match message.get_videochat_type() {
                Some(vct) => Some(
                    vct.to_u32()
                        .ok_or_else(|| anyhow!("state conversion to number failed"))?,
                ),
                None => None,
            },
            videochat_url: message.get_videochat_url(),

            override_sender_name,
            sender,

            setup_code_begin: message.get_setupcodebegin(context).await,

            file: match message.get_file(context) {
                Some(path_buf) => path_buf.to_str().map(|s| s.to_owned()),
                None => None,
            }, //BLOBS
            file_mime: message.get_filemime(),
            file_bytes,
            file_name: message.get_filename(),
        })
    }
}

impl ReturnType for MessageObject {
    custom_return_type!("Message_Type".to_owned());

    fn get_typescript_type() -> String {
        r#"
        {
            id: number,
            chat_id: number,
            from_id: number,
            quoted_text: string | null,
            quoted_message_id: number | null,
            text: string,
            has_location: boolean,
            has_html: boolean,
            view_type: number,
            state: number,

            timestamp: number,
            sort_timestamp: number,
            received_timestamp: number,
            has_deviating_timestamp: boolean,
            
            subject: string | null,
            show_padlock: boolean,
            is_setupmessage: boolean,
            is_info: boolean,
            is_forwarded: boolean,
        
            duration: number,
            dimensions_height: number | null,
            dimensions_width: number | null,
        
            videochat_type: number | null,
            videochat_url: string | null,
            override_sender_name: string | null,
        
            sender: Contact_Type,
            setup_code_begin: string | null,
        
            file: string | null,
            file_mime: string | null,
            file_bytes: number | null,
            file_name: string | null,
        }
        "#
        .to_owned()
    }

    fn into_json_value(self) -> Value {
        jsonrpc_core::serde_json::to_value(self).unwrap() // todo: can we somehow get rid of that unwrap here?
    }
}
