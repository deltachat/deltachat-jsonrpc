use std::convert::TryInto;
use std::{collections::HashMap, str::FromStr};

use dc_cmd_derive::gen_command_api;
use deltachat::config::Config;
use deltachat::constants::*;
use deltachat::contact::may_be_valid_addr;
use deltachat::contact::Contact;
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
        "{ id: number } | { id: number, display_name: string | null, addr: string | null, profile_image: string | null, color: string }".to_owned()
    }

    fn into_json_value(self) -> Value {
        match self {
            Account::Unconfigured { id } => json!({ "id": id }),
            Account::Configured {
                id,
                display_name,
                addr,
                profile_image,
                color,
            } => json!({
               "id": id,
               "display_name": display_name,
               "addr": addr,
               "profile_image": profile_image,
               "color": color
            }),
        }
    }

    custom_return_type!("Account_Type".to_owned());
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
    },
    #[serde(rename_all = "camelCase")]
    DeadDrop {
        last_updated: i64,
        sender_name: String,
        sender_address: String,
        sender_contact_id: u32,
        message_id: u32,
        summary_text1: String,
        summary_text2: String,
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
            type: \"DeadDrop\"; \
            lastUpdated: number; \
            messageId: number; \
            senderAddress: string; \
            senderContactId: number; \
            senderName: string; \
            summaryText1: string; \
            summaryText2: string; \
          } \
        | { \
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

    if chat_id.is_deaddrop() {
        let last_message_id =
            last_msgid.ok_or_else(|| anyhow!("couldn't fetch last chat message on deadrop"))?;
        let last_message = deltachat::message::Message::load_from_db(&ctx, last_message_id).await?;

        let contact =
            deltachat::contact::Contact::load_from_db(&ctx, last_message.get_from_id()).await?;

        return Ok(ChatListItemFetchResult::DeadDrop {
            last_updated: last_message.get_timestamp() * 1000,
            sender_name: contact.get_display_name().to_owned(),
            sender_address: contact.get_addr().to_owned(),
            sender_contact_id: contact.get_id(),
            message_id: last_message_id.to_u32(),
            summary_text1,
            summary_text2,
        });
    }

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
    //       Misc context independent methods
    // ---------------------------------------------
    async fn check_email_validity(&self, email: String) -> bool {
        return may_be_valid_addr(&email);
    }

    // ---------------------------------------------
    //              Account Management
    // ---------------------------------------------

    async fn add_account(&self) -> Result<u32> {
        self.manager.add_account().await
    }

    async fn get_all_account_ids(&self) -> Vec<u32> {
        self.manager.get_all().await
    }

    async fn get_all_accounts(&self) -> Result<Vec<Account>> {
        let mut accounts = Vec::new();
        for id in self.manager.get_all().await {
            let context_option = self.manager.get_account(id).await;
            if let Some(ctx) = context_option {
                if ctx.is_configured().await? {
                    let display_name = ctx.get_config(Config::Displayname).await?;
                    let addr = ctx.get_config(Config::Addr).await?;
                    let profile_image = ctx.get_config(Config::Selfavatar).await?;
                    let color = color_int_to_hex_string(
                        Contact::get_by_id(&ctx, DC_CONTACT_ID_SELF)
                            .await?
                            .get_color(),
                    );
                    accounts.push(Account::Configured {
                        id,
                        display_name,
                        addr,
                        profile_image,
                        color,
                    });
                } else {
                    accounts.push(Account::Unconfigured { id });
                }
            } else {
                println!("account with id {} doesn't exist anymore", id);
            }
        }
        return Ok(accounts);
    }

    async fn select_account(&self, id: u32) -> Result<()> {
        self.manager.select_account(id).await
    }

    // ---------------------------------------------
    // Functions for the selected Account / Context
    // ---------------------------------------------

    // TODO add a function where an parameter is a custom struct / object

    // TODO fn sc_send_message () -> {}

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
        for (i, entry) in entries.iter().enumerate() {
            result.insert(
                i.try_into().unwrap(),
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
}
