use anyhow::{anyhow, bail, Context, Result};
use async_std::sync::{Arc, RwLock};
use deltachat::message::MsgId;
use deltachat::{
    chat::{get_chat_msgs, ChatId},
    chatlist::Chatlist,
    config::Config,
    constants::*,
    contact::{may_be_valid_addr, Contact},
    context::get_info,
    message::Message,
    provider::get_provider_info,
};
use std::collections::BTreeMap;
use std::{collections::HashMap, str::FromStr};
use yerpc::rpc;

pub use deltachat::accounts::Accounts;

pub mod events;
pub mod types;

use crate::api::types::chat_list::{ChatListItemFetchResult, _get_chat_list_items_by_id};

use types::account::Account;
use types::chat::FullChat;
use types::chat_list::ChatListEntry;
use types::contact::ContactObject;
use types::message::MessageObject;
use types::provider_info::ProviderInfo;

#[derive(Clone, Debug)]
pub struct CommandApi {
    pub(crate) accounts: Arc<RwLock<Accounts>>,
}

impl CommandApi {
    pub fn new(accounts: Accounts) -> Self {
        CommandApi {
            accounts: Arc::new(RwLock::new(accounts)),
        }
    }

    async fn get_context(&self, id: u32) -> Result<deltachat::context::Context> {
        let sc = self
            .accounts
            .read()
            .await
            .get_account(id)
            .await
            .ok_or_else(|| anyhow!("account with id {} not found", id))?;
        Ok(sc)
    }
}

#[rpc(all_positional, ts_outdir = "typescript/generated")]
impl CommandApi {
    // ---------------------------------------------
    //
    //       Misc context independent methods
    //
    // ---------------------------------------------
    async fn check_email_validity(&self, email: String) -> bool {
        may_be_valid_addr(&email)
    }

    /// get general info, even if no context is selected
    async fn get_system_info(&self) -> BTreeMap<&'static str, String> {
        get_info()
    }

    async fn get_provider_info(
        &self,
        account_id: u32,
        email: String,
    ) -> Result<Option<ProviderInfo>> {
        let ctx = self.get_context(account_id).await?;

        let socks5_enabled = ctx
            .get_config_bool(deltachat::config::Config::Socks5Enabled)
            .await?;

        let provider_info =
            get_provider_info(&ctx, email.split('@').last().unwrap_or(""), socks5_enabled).await;
        Ok(ProviderInfo::from_dc_type(provider_info))
    }

    // ---------------------------------------------
    //
    //              Account Management
    //
    // ---------------------------------------------

    async fn add_account(&self) -> Result<u32> {
        self.accounts.write().await.add_account().await
    }

    async fn remove_account(&self, account_id: u32) -> Result<()> {
        self.accounts.write().await.remove_account(account_id).await
    }

    async fn get_all_account_ids(&self) -> Vec<u32> {
        self.accounts.read().await.get_all().await
    }

    async fn get_account_info(&self, account_id: u32) -> Result<Account> {
        let context_option = self.accounts.read().await.get_account(account_id).await;
        if let Some(ctx) = context_option {
            Ok(Account::from_context(&ctx, account_id).await?)
        } else {
            Err(anyhow!(
                "account with id {} doesn't exist anymore",
                account_id
            ))
        }
    }

    async fn get_all_accounts(&self) -> Result<Vec<Account>> {
        let mut accounts = Vec::new();
        for id in self.accounts.read().await.get_all().await {
            let context_option = self.accounts.read().await.get_account(id).await;
            if let Some(ctx) = context_option {
                accounts.push(Account::from_context(&ctx, id).await?)
            } else {
                println!("account with id {} doesn't exist anymore", id);
            }
        }
        Ok(accounts)
    }

    async fn select_account(&self, id: u32) -> Result<()> {
        self.accounts.write().await.select_account(id).await
    }

    async fn get_selected_account_id(&self) -> Option<u32> {
        self.accounts.read().await.get_selected_account_id().await
    }

    // ---------------------------------------------
    //
    //     Functions that need an Account
    //
    // ---------------------------------------------

    async fn is_configured(&self, account_id: u32) -> Result<bool> {
        let ctx = self.get_context(account_id).await?;
        Ok(ctx.is_configured().await?)
    }

    async fn get_info(&self, account_id: u32) -> Result<BTreeMap<&'static str, String>> {
        let ctx = self.get_context(account_id).await?;
        Ok(ctx.get_info().await?)
    }

    async fn set_config(&self, account_id: u32, key: String, value: Option<String>) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        set_config(&ctx, &key, value.as_deref()).await
    }

    async fn batch_set_config(
        &self,
        account_id: u32,
        config: HashMap<String, Option<String>>,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        for (key, value) in config.into_iter() {
            set_config(&ctx, &key, value.as_deref())
                .await
                .with_context(|| format!("Can't set {} to {:?}", key, value))?;
        }
        Ok(())
    }

    async fn get_config(&self, account_id: u32, key: String) -> Result<Option<String>> {
        let ctx = self.get_context(account_id).await?;
        get_config(&ctx, &key).await
    }

    async fn batch_get_config(
        &self,
        account_id: u32,
        keys: Vec<String>,
    ) -> Result<HashMap<String, Option<String>>> {
        let ctx = self.get_context(account_id).await?;
        let mut result: HashMap<String, Option<String>> = HashMap::new();
        for key in keys {
            result.insert(key.clone(), get_config(&ctx, &key).await?);
        }
        Ok(result)
    }

    /// set config for the credentials before calling this
    async fn configure(&self, account_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        ctx.stop_io().await;
        ctx.configure().await?;
        ctx.start_io().await;
        Ok(())
    }

    /// Signal an ongoing process to stop.
    async fn stop_ongoing_process(&self, account_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        ctx.stop_ongoing().await;
        Ok(())
    }

    // ---------------------------------------------
    //                  autocrypt
    // ---------------------------------------------

    async fn autocrypt_initiate_key_transfer(&self, account_id: u32) -> Result<String> {
        let ctx = self.get_context(account_id).await?;
        deltachat::imex::initiate_key_transfer(&ctx).await
    }

    async fn autocrypt_continue_key_transfer(
        &self,
        account_id: u32,
        message_id: u32,
        setup_code: String,
    ) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        deltachat::imex::continue_key_transfer(&ctx, MsgId::new(message_id), &setup_code).await
    }

    // ---------------------------------------------
    //                  Chat List
    // ---------------------------------------------

    async fn get_chatlist_entries(
        &self,
        account_id: u32,
        list_flags: u32,
        query_string: Option<String>,
        query_contact_id: Option<u32>,
    ) -> Result<Vec<ChatListEntry>> {
        let ctx = self.get_context(account_id).await?;
        let list = Chatlist::try_load(
            &ctx,
            list_flags as usize,
            query_string.as_deref(),
            query_contact_id,
        )
        .await?;
        let mut l: Vec<ChatListEntry> = Vec::new();
        for i in 0..list.len() {
            l.push(ChatListEntry(
                list.get_chat_id(i)?.to_u32(),
                list.get_msg_id(i)?.unwrap_or_default().to_u32(),
            ));
        }
        Ok(l)
    }

    async fn get_chatlist_items_by_entries(
        &self,
        account_id: u32,
        entries: Vec<ChatListEntry>,
    ) -> Result<HashMap<u32, ChatListItemFetchResult>> {
        // todo custom json deserializer for ChatListEntry?
        let ctx = self.get_context(account_id).await?;
        let mut result: HashMap<u32, ChatListItemFetchResult> = HashMap::new();
        for (_i, entry) in entries.iter().enumerate() {
            result.insert(
                entry.0,
                match _get_chat_list_items_by_id(&ctx, entry).await {
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

    async fn chatlist_get_full_chat_by_id(
        &self,
        account_id: u32,
        chat_id: u32,
    ) -> Result<FullChat> {
        let ctx = self.get_context(account_id).await?;
        FullChat::from_dc_chat_id(&ctx, chat_id).await
    }

    async fn accept_chat(&self, account_id: u32, chat_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        ChatId::new(chat_id).accept(&ctx).await
    }

    async fn block_chat(&self, account_id: u32, chat_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        ChatId::new(chat_id).block(&ctx).await
    }

    // ---------------------------------------------
    //                Message List
    // ---------------------------------------------

    async fn message_list_get_message_ids(
        &self,
        account_id: u32,
        chat_id: u32,
        flags: u32,
    ) -> Result<Vec<u32>> {
        let ctx = self.get_context(account_id).await?;
        let msg = get_chat_msgs(&ctx, ChatId::new(chat_id), flags, None).await?;
        Ok(msg
            .iter()
            .filter_map(|chat_item| match chat_item {
                deltachat::chat::ChatItem::Message { msg_id } => Some(msg_id.to_u32()),
                _ => None,
            })
            .collect())
    }

    async fn message_get_message(&self, account_id: u32, message_id: u32) -> Result<MessageObject> {
        let ctx = self.get_context(account_id).await?;
        MessageObject::from_message_id(message_id, &ctx).await
    }

    async fn message_get_messages(
        &self,
        account_id: u32,
        message_ids: Vec<u32>,
    ) -> Result<HashMap<u32, MessageObject>> {
        let ctx = self.get_context(account_id).await?;
        let mut messages: HashMap<u32, MessageObject> = HashMap::new();
        for message_id in message_ids {
            messages.insert(
                message_id,
                MessageObject::from_message_id(message_id, &ctx).await?,
            );
        }
        Ok(messages)
    }

    // ---------------------------------------------
    //                   Contact
    // ---------------------------------------------

    async fn contacts_get_contact(
        &self,
        account_id: u32,
        contact_id: u32,
    ) -> Result<ContactObject> {
        let ctx = self.get_context(account_id).await?;

        ContactObject::from_dc_contact(
            &ctx,
            deltachat::contact::Contact::get_by_id(&ctx, contact_id).await?,
        )
        .await
    }

    /// Add a single contact as a result of an explicit user action.
    ///
    /// Returns contact id of the created or existing contact
    async fn contacts_create_contact(
        &self,
        account_id: u32,
        email: String,
        name: Option<String>,
    ) -> Result<u32> {
        let ctx = self.get_context(account_id).await?;
        if !may_be_valid_addr(&email) {
            bail!(anyhow!(
                "provided email address is not a valid email address"
            ))
        }
        Contact::create(&ctx, &name.unwrap_or_default(), &email).await
    }

    /// Returns contact id of the created or existing DM chat with that contact
    async fn contacts_create_chat_by_contact_id(
        &self,
        account_id: u32,
        contact_id: u32,
    ) -> Result<u32> {
        let ctx = self.get_context(account_id).await?;
        let contact = Contact::get_by_id(&ctx, contact_id).await?;
        ChatId::create_for_contact(&ctx, contact.id)
            .await
            .map(|id| id.to_u32())
    }

    async fn contacts_block(&self, account_id: u32, contact_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        Contact::block(&ctx, contact_id).await
    }

    async fn contacts_unblock(&self, account_id: u32, contact_id: u32) -> Result<()> {
        let ctx = self.get_context(account_id).await?;
        Contact::unblock(&ctx, contact_id).await
    }

    async fn contacts_get_blocked(&self, account_id: u32) -> Result<Vec<ContactObject>> {
        let ctx = self.get_context(account_id).await?;
        let blocked_ids = Contact::get_all_blocked(&ctx).await?;
        let mut contacts: Vec<ContactObject> = Vec::with_capacity(blocked_ids.len());
        for id in blocked_ids {
            contacts.push(
                ContactObject::from_dc_contact(
                    &ctx,
                    deltachat::contact::Contact::get_by_id(&ctx, id).await?,
                )
                .await?,
            );
        }
        Ok(contacts)
    }

    async fn contacts_get_contact_ids(
        &self,
        account_id: u32,
        list_flags: u32,
        query: Option<String>,
    ) -> Result<Vec<u32>> {
        let ctx = self.get_context(account_id).await?;
        Contact::get_all(&ctx, list_flags, query).await
    }

    // formerly called getContacts2 in desktop
    async fn contacts_get_contacts(
        &self,
        account_id: u32,
        list_flags: u32,
        query: Option<String>,
    ) -> Result<Vec<ContactObject>> {
        let ctx = self.get_context(account_id).await?;
        let contact_ids = Contact::get_all(&ctx, list_flags, query).await?;
        let mut contacts: Vec<ContactObject> = Vec::with_capacity(contact_ids.len());
        for id in contact_ids {
            contacts.push(
                ContactObject::from_dc_contact(
                    &ctx,
                    deltachat::contact::Contact::get_by_id(&ctx, id).await?,
                )
                .await?,
            );
        }
        Ok(contacts)
    }

    async fn contacts_get_contacts_by_ids(
        &self,
        account_id: u32,
        ids: Vec<u32>,
    ) -> Result<HashMap<u32, ContactObject>> {
        let ctx = self.get_context(account_id).await?;

        let mut contacts = HashMap::with_capacity(ids.len());
        for id in ids {
            contacts.insert(
                id,
                ContactObject::from_dc_contact(
                    &ctx,
                    deltachat::contact::Contact::get_by_id(&ctx, id).await?,
                )
                .await?,
            );
        }
        Ok(contacts)
    }

    // ---------------------------------------------
    //           misc prototyping functions
    //       that might get removed later again
    // ---------------------------------------------

    /// Returns the messageid of the sent message
    async fn misc_send_text_message(
        &self,
        account_id: u32,
        text: String,
        chat_id: u32,
    ) -> Result<u32> {
        let ctx = self.get_context(account_id).await?;

        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some(text));

        let message_id = deltachat::chat::send_msg(&ctx, ChatId::new(chat_id), &mut msg).await?;
        Ok(message_id.to_u32())
    }
}

// Helper functions (to prevent code duplication)
async fn set_config(
    ctx: &deltachat::context::Context,
    key: &str,
    value: Option<&str>,
) -> Result<(), anyhow::Error> {
    if key.starts_with("ui.") {
        ctx.set_ui_config(key, value).await
    } else {
        ctx.set_config(Config::from_str(key).context("unknown key")?, value)
            .await
    }
}

async fn get_config(
    ctx: &deltachat::context::Context,
    key: &str,
) -> Result<Option<String>, anyhow::Error> {
    if key.starts_with("ui.") {
        ctx.get_ui_config(key).await
    } else {
        ctx.get_config(Config::from_str(key).context("unknown key")?)
            .await
    }
}
