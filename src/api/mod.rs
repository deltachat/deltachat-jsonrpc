use std::collections::BTreeMap;
use std::{collections::HashMap, str::FromStr};

use dc_cmd_derive::gen_command_api;
use deltachat::chat::get_chat_msgs;
use deltachat::config::Config;
use deltachat::constants::*;
use deltachat::contact::Contact;
use deltachat::contact::{may_be_valid_addr, VerifiedStatus};
use deltachat::context::{get_info, Context};
use deltachat::message::Message;
use deltachat::provider::get_provider_info;
use deltachat::{
    accounts::Accounts,
    chat::{Chat, ChatId},
    message::MsgId,
};
use deltachat::{
    chat::{get_chat_contacts, ChatVisibility},
    chatlist::Chatlist,
};

use num_traits::cast::ToPrimitive;

use anyhow::{anyhow, Result};
use jsonrpc_core::serde_json::{json, Value};
use serde::{Deserialize, Serialize};

pub(crate) mod return_type;
use return_type::*;

pub mod events;

use crate::custom_return_type;

enum Account {
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
    async fn from_context(id: u32, ctx: &deltachat::context::Context) -> Result<Self> {
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

#[derive(Serialize)]
struct ContactObject {
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
    async fn from_dc_contact(
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
            is_verified: contact.is_verified(context).await == VerifiedStatus::BidirectVerified,
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

#[derive(Serialize)]
struct FullChat {
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
    async fn from_dc_chat_id(chat_id: u32, context: &Context) -> Result<Self> {
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

        Ok(FullChat {
            id: chat_id,
            name: chat.name.clone(),
            is_protected: chat.is_protected(),
            profile_image: match chat.get_profile_image(context).await? {
                Some(path_buf) => path_buf.to_str().map(|s| s.to_owned()),
                None => None,
            }, //BLOBS ?
            archived: chat.get_visibility() == deltachat::chat::ChatVisibility::Archived,
            chat_type: chat.get_type().to_u32().unwrap(), // TODO get rid of this unwrap?
            is_unpromoted: chat.is_unpromoted(),
            is_self_talk: chat.is_self_talk(),
            contacts,
            contact_ids: contact_ids.clone(),
            color: color_int_to_hex_string(chat.get_color(context).await?),
            fresh_message_counter: rust_chat_id.get_fresh_msg_cnt(context).await?,
            is_contact_request: chat.is_contact_request(),
            is_device_chat: chat.is_device_talk(),
            self_in_group: contact_ids.contains(&DC_CONTACT_ID_SELF),
            is_muted: chat.is_muted(),
            ephemeral_timer: rust_chat_id.get_ephemeral_timer(context).await?.to_u32(),
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

#[derive(Serialize)]
struct MessageObject {
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
    async fn from_message_id(message_id: u32, context: &Context) -> Result<Self> {
        let msg_id = MsgId::new(message_id);
        let message = Message::load_from_db(context, msg_id).await?;

        let quoted_message_id = message
            .quoted_message(context)
            .await?
            .map(|m| m.get_id().to_u32());

        let sender_contact = Contact::load_from_db(context, message.get_from_id()).await?;
        let sender = ContactObject::from_dc_contact(sender_contact, context).await?;

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
                .ok_or(anyhow!("viewtype conversion to number failed"))?,
            state: message
                .get_state()
                .to_u32()
                .ok_or(anyhow!("state conversion to number failed"))?,

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
                        .ok_or(anyhow!("state conversion to number failed"))?,
                ),
                None => None,
            },
            videochat_url: message.get_videochat_url(),

            override_sender_name: message.get_override_sender_name(),
            sender,

            setup_code_begin: message.get_setupcodebegin(context).await,

            file: match message.get_file(context) {
                Some(path_buf) => path_buf.to_str().map(|s| s.to_owned()),
                None => None,
            }, //BLOBS
            file_mime: message.get_filemime(),
            file_bytes: message.get_filebytes(context).await,
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

struct ProviderInfo {
    before_login_hint: String,
    overview_page: String,
    status: u32, // in reality this is an enum, but for simlicity and because it gets converted into a number anyway, we use an u32 here.
}

impl ReturnType for ProviderInfo {
    fn get_typescript_type() -> String {
        "{ before_login_hint: string, overview_page: string, status: 1 | 2 | 3 }".to_owned()
    }

    fn into_json_value(self) -> Value {
        json!({
            "before_login_hint": self.before_login_hint,
            "overview_page": self.overview_page,
            "status": self.status
        })
    }

    custom_return_type!("ProviderInfo_Type".to_owned());
}

#[derive(Deserialize)]
struct ChatListEntry(u32, u32);
impl ReturnType for ChatListEntry {
    fn get_typescript_type() -> String {
        "[number, number]".to_owned()
    }

    fn into_json_value(self) -> Value {
        Value::Array(vec![self.0.into_json_value(), self.1.into_json_value()])
    }

    custom_return_type!("ChatListEntry_Type".to_owned());
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ChatListItemFetchResult {
    #[serde(rename_all = "camelCase")]
    ChatListItem {
        id: u32,
        name: String,
        avatar_path: Option<String>,
        color: String,
        last_updated: Option<i64>,
        summary_text1: String,
        summary_text2: String,
        summary_status: u32,
        is_protected: bool,
        is_group: bool,
        fresh_message_counter: usize,
        is_self_talk: bool,
        is_device_talk: bool,
        is_sending_location: bool,
        is_self_in_group: bool,
        is_archived: bool,
        is_pinned: bool,
        is_muted: bool,
        is_contact_request: bool,
    },
    ArchiveLink,
    #[serde(rename_all = "camelCase")]
    Error {
        id: u32,
        error: String,
    },
}

impl ReturnType for ChatListItemFetchResult {
    fn get_typescript_type() -> String {
        "\n | { \
            type: \"ChatListItem\"; \
            id: number; \
            name: string; \
            avatarPath: null | string; \
            color: string; \
            lastUpdated: number; \
            freshMessageCounter: number; \
            summaryStatus: number; \
            summaryText1: string; \
            summaryText2: string; \
            isArchived: boolean; \
            isDeviceTalk: boolean; \
            isGroup: boolean; \
            isMuted: boolean; \
            isPinned: boolean; \
            isSelfInGroup: boolean; \
            isSelfTalk: boolean; \
            isSendingLocation: boolean; \
            isProtected: boolean; \
            isContactRequest: boolean; \
          } \
        | { type: \"ArchiveLink\" } \
        | { \
            type: \"Error\"; \
            id: number; \
            error: string; \
          }"
        .to_owned()
    }

    fn into_json_value(self) -> Value {
        jsonrpc_core::serde_json::to_value(self).unwrap() // todo: can we somehow get rid of that unwrap here?
    }

    custom_return_type!("ChatListItemFetchResult_Type".to_owned());
}

async fn _get_chat_list_items_by_id(
    ctx: &deltachat::context::Context,
    entry: &ChatListEntry,
) -> Result<ChatListItemFetchResult> {
    let chat_id = ChatId::new(entry.0);
    let last_msgid = match entry.1 {
        0 => None,
        _ => Some(MsgId::new(entry.1)),
    };

    if chat_id.is_archived_link() {
        return Ok(ChatListItemFetchResult::ArchiveLink);
    }

    let chat = Chat::load_from_db(&ctx, chat_id).await?;
    let summary = Chatlist::get_summary2(&ctx, chat_id, last_msgid, Some(&chat)).await?;

    let summary_text1 = summary.get_text1().unwrap_or("").to_owned();
    let summary_text2 = summary.get_text2().unwrap_or("").to_owned();

    let visibility = chat.get_visibility();

    let avatar_path = match chat.get_profile_image(ctx).await? {
        Some(path) => Some(path.to_str().unwrap_or("invalid/path").to_owned()),
        None => None,
    };

    let last_updated = match last_msgid {
        Some(id) => {
            let last_message = deltachat::message::Message::load_from_db(&ctx, id).await?;
            Some(last_message.get_timestamp() * 1000)
        }
        None => None,
    };

    let self_in_group = get_chat_contacts(&ctx, chat_id)
        .await?
        .contains(&DC_CONTACT_ID_SELF);

    let fresh_message_counter = chat_id.get_fresh_msg_cnt(&ctx).await?;
    let color = color_int_to_hex_string(chat.get_color(&ctx).await?);

    Ok(ChatListItemFetchResult::ChatListItem {
        id: chat_id.to_u32(),
        name: chat.get_name().to_owned(),
        avatar_path,
        color,
        last_updated,
        summary_text1,
        summary_text2,
        summary_status: summary.get_state().to_u32().expect("impossible"), // idea and a function to transform the constant to strings? or return string enum
        is_protected: chat.is_protected(),
        is_group: chat.get_type() == Chattype::Group,
        fresh_message_counter,
        is_self_talk: chat.is_self_talk(),
        is_device_talk: chat.is_device_talk(),
        is_self_in_group: self_in_group,
        is_sending_location: chat.is_sending_locations(),
        is_archived: visibility == ChatVisibility::Archived,
        is_pinned: visibility == ChatVisibility::Pinned,
        is_muted: chat.is_muted(),
        is_contact_request: chat.is_contact_request(),
    })
}

fn color_int_to_hex_string(color: u32) -> String {
    format!("{:#08x}", color).replace("0x", "#")
}

#[derive(Clone, Debug)]
pub struct CommandApi {
    manager: Accounts,
}

impl CommandApi {
    pub fn new(am: &Accounts) -> Self {
        CommandApi {
            manager: am.clone(),
        }
    }

    async fn selected_context(&self) -> Result<deltachat::context::Context> {
        let sc = self.manager.get_selected_account().await.ok_or_else(|| {
            anyhow!("no account/context selected, select one with select_account")
        })?;
        Ok(sc)
    }
}

#[gen_command_api]
impl CommandApi {
    // ---------------------------------------------
    //
    //       Misc context independent methods
    //
    // ---------------------------------------------
    async fn check_email_validity(&self, email: String) -> bool {
        return may_be_valid_addr(&email);
    }

    /// get general info, even if no context is selected
    async fn get_system_info(&self) -> BTreeMap<&'static str, String> {
        get_info()
    }

    async fn get_provider_info(&self, email: String) -> Option<ProviderInfo> {
        let provider = get_provider_info(&email).await;
        provider.map(|p| ProviderInfo {
            before_login_hint: p.before_login_hint.to_owned(),
            overview_page: p.overview_page.to_owned(),
            status: p.status.to_u32().unwrap(),
        })
    }

    // ---------------------------------------------
    //
    //              Account Management
    //
    // ---------------------------------------------

    async fn add_account(&self) -> Result<u32> {
        self.manager.add_account().await
    }

    async fn remove_account(&self, account_id: u32) -> Result<()> {
        self.manager.remove_account(account_id).await
    }

    async fn get_all_account_ids(&self) -> Vec<u32> {
        self.manager.get_all().await
    }

    async fn get_account_info(&self, account_id: u32) -> Result<Account> {
        let context_option = self.manager.get_account(account_id).await;
        if let Some(ctx) = context_option {
            Ok(Account::from_context(account_id, &ctx).await?)
        } else {
            Err(anyhow!(
                "account with id {} doesn't exist anymore",
                account_id
            ))
        }
    }

    async fn get_all_accounts(&self) -> Result<Vec<Account>> {
        let mut accounts = Vec::new();
        for id in self.manager.get_all().await {
            let context_option = self.manager.get_account(id).await;
            if let Some(ctx) = context_option {
                accounts.push(Account::from_context(id, &ctx).await?)
            } else {
                println!("account with id {} doesn't exist anymore", id);
            }
        }
        return Ok(accounts);
    }

    async fn select_account(&self, id: u32) -> Result<()> {
        self.manager.select_account(id).await
    }

    async fn get_selected_account_id(&self) -> Option<u32> {
        // TODO use the simpler api when availible: https://github.com/deltachat/deltachat-core-rust/pull/2570
        match self.manager.get_selected_account().await {
            Some(ctx) => Some(ctx.get_id()),
            None => None,
        }
    }

    // ---------------------------------------------
    //
    //     Functions for the selected Account
    //
    // ---------------------------------------------

    // TODO add a function where an parameter is a custom struct / object

    // TODO fn sc_send_message () -> {}

    async fn sc_is_configured(&self) -> Result<bool> {
        let sc = self.selected_context().await?;
        Ok(sc.is_configured().await?)
    }

    async fn sc_get_info(&self) -> Result<BTreeMap<&'static str, String>> {
        let sc = self.selected_context().await?;
        Ok(sc.get_info().await?)
    }

    async fn sc_set_config(&self, key: String, value: Option<String>) -> Result<()> {
        let sc = self.selected_context().await?;
        let value = value.as_ref().map(String::as_ref);
        Ok(sc.set_config(Config::from_str(&key)?, value).await?)
    }

    async fn sc_get_config(&self, key: String) -> Result<Option<String>> {
        let sc = self.selected_context().await?;
        Ok(sc.get_config(Config::from_str(&key)?).await?)
    }

    async fn sc_batch_get_config(
        &self,
        keys: Vec<String>,
    ) -> Result<HashMap<String, Option<String>>> {
        let sc = self.selected_context().await?;
        let mut result: HashMap<String, Option<String>> = HashMap::new();
        for key in keys {
            result.insert(key.clone(), sc.get_config(Config::from_str(&key)?).await?);
        }
        Ok(result)
    }

    /// set config for the credentials before calling this
    async fn sc_configure(&self) -> Result<()> {
        let sc = self.selected_context().await?;
        self.manager.stop_io().await;
        sc.configure().await?;
        self.manager.start_io().await;
        Ok(())
    }

    /// Signal an ongoing process to stop.
    async fn sc_stop_ongoing_process(&self) -> Result<()> {
        let sc = self.selected_context().await?;
        sc.stop_ongoing().await;
        Ok(())
    }

    // ---------------------------------------------
    //                  Chat List
    // ---------------------------------------------

    async fn sc_get_chatlist_entries(
        &self,
        list_flags: u32,
        query_string: Option<String>,
        query_contact_id: Option<u32>,
    ) -> Result<Vec<ChatListEntry>> {
        let sc = self.selected_context().await?;
        let list = Chatlist::try_load(
            &sc,
            list_flags as usize,
            query_string.as_deref(),
            query_contact_id,
        )
        .await?;
        let mut l: Vec<ChatListEntry> = Vec::new();
        for i in 0..list.len() {
            l.push(ChatListEntry(
                list.get_chat_id(i).to_u32(),
                list.get_msg_id(i)?.unwrap_or_default().to_u32(),
            ));
        }
        Ok(l)
    }

    async fn sc_get_chatlist_items_by_entries(
        &self,
        entries: Vec<ChatListEntry>,
    ) -> Result<HashMap<u32, ChatListItemFetchResult>> {
        // todo custom json deserializer for ChatListEntry?
        let sc = self.selected_context().await?;
        let mut result: HashMap<u32, ChatListItemFetchResult> = HashMap::new();
        for (_i, entry) in entries.iter().enumerate() {
            result.insert(
                entry.0,
                match _get_chat_list_items_by_id(&sc, entry).await {
                    Ok(res) => res,
                    Err(err) => ChatListItemFetchResult::Error {
                        id: entry.0,
                        error: format!("{:?}", err),
                    },
                },
            );
        }
        Ok(result)
    }

    // ---------------------------------------------
    //                    Chat
    // ---------------------------------------------

    async fn sc_chatlist_get_full_chat_by_id(&self, chat_id: u32) -> Result<FullChat> {
        let sc = self.selected_context().await?;
        FullChat::from_dc_chat_id(chat_id, &sc).await
    }

    async fn sc_accept_chat(&self, chat_id: u32) -> Result<()> {
        let sc = self.selected_context().await?;
        ChatId::new(chat_id).accept(&sc).await
    }

    async fn sc_block_chat(&self, chat_id: u32) -> Result<()> {
        let sc = self.selected_context().await?;
        ChatId::new(chat_id).block(&sc).await
    }

    // ---------------------------------------------
    //                Message List
    // ---------------------------------------------

    async fn sc_message_list_get_message_ids(&self, chat_id: u32, flags: u32) -> Result<Vec<u32>> {
        let sc = self.selected_context().await?;
        let msg = get_chat_msgs(&sc, ChatId::new(chat_id), flags, None).await?;
        Ok(msg
            .iter()
            .filter_map(|chat_item| match chat_item {
                deltachat::chat::ChatItem::Message { msg_id } => Some(msg_id.to_u32()),
                _ => None,
            })
            .collect())
    }

    async fn sc_message_get_message(&self, message_id: u32) -> Result<MessageObject> {
        let sc = self.selected_context().await?;
        MessageObject::from_message_id(message_id, &sc).await
    }

    async fn sc_message_get_messages(
        &self,
        message_ids: Vec<u32>,
    ) -> Result<HashMap<u32, MessageObject>> {
        let sc = self.selected_context().await?;
        let mut messages: HashMap<u32, MessageObject> = HashMap::new();
        for message_id in message_ids {
            messages.insert(
                message_id,
                MessageObject::from_message_id(message_id, &sc).await?,
            );
        }
        Ok(messages)
    }

    // ---------------------------------------------
    //                   Contact
    // ---------------------------------------------

    async fn sc_contacts_get_contact(&self, contact_id: u32) -> Result<ContactObject> {
        let sc = self.selected_context().await?;

        ContactObject::from_dc_contact(
            deltachat::contact::Contact::get_by_id(&sc, contact_id).await?,
            &sc,
        )
        .await
    }

    // ---------------------------------------------
    //           misc prototyping functions
    //       that might get removed later again
    // ---------------------------------------------

    /// Returns the messageid of the sent message
    async fn sc_misc_send_text_message(&self, text: String, chat_id: u32) -> Result<u32> {
        let sc = self.selected_context().await?;

        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some(text));

        let message_id = deltachat::chat::send_msg(&sc, ChatId::new(chat_id), &mut msg).await?;
        Ok(message_id.to_u32())
    }
}
