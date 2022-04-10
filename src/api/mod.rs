use async_std::sync::{Arc, RwLock};
use deltachat::message::MsgId;
use std::collections::BTreeMap;
use std::ops::Deref;
use std::{collections::HashMap, str::FromStr};

use dc_cmd_derive::gen_command_api;
use deltachat::{
    accounts::Accounts,
    chat::{get_chat_msgs, ChatId},
    chatlist::Chatlist,
    config::Config,
    constants::*,
    contact::{may_be_valid_addr, Contact},
    context::get_info,
    message::Message,
    provider::get_provider_info,
};

use anyhow::{anyhow, bail, Result};
use serde::Deserialize;

pub mod events;
pub mod types;

use crate::api::types::chat_list::{ChatListItemFetchResult, _get_chat_list_items_by_id};

use types::account::Account;
use types::chat::FullChat;
use types::chat_list::ChatListEntry;
use types::contact::ContactObject;
use types::message::MessageObject;
use types::provider_info::ProviderInfo;
use types::return_type::*;

#[derive(Clone, Debug)]
pub struct AccountsWrapper {
    pub inner: Arc<RwLock<Accounts>>,
}

impl Deref for AccountsWrapper {
    type Target = RwLock<Accounts>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Clone, Debug)]
pub struct CommandApi {
    manager: AccountsWrapper,
}

impl CommandApi {
    pub fn new(am: AccountsWrapper) -> Self {
        CommandApi { manager: am }
    }

    async fn selected_context(&self) -> Result<deltachat::context::Context> {
        let sc = self
            .manager
            .read()
            .await
            .get_selected_account()
            .await
            .ok_or_else(|| {
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
        may_be_valid_addr(&email)
    }

    /// get general info, even if no context is selected
    async fn get_system_info(&self) -> BTreeMap<&'static str, String> {
        get_info()
    }

    async fn get_provider_info(&self, email: String) -> Option<ProviderInfo> {
        ProviderInfo::from_dc_type(get_provider_info(&email, false).await)
    }

    // ---------------------------------------------
    //
    //              Account Management
    //
    // ---------------------------------------------

    async fn add_account(&self) -> Result<u32> {
        self.manager.write().await.add_account().await
    }

    async fn remove_account(&self, account_id: u32) -> Result<()> {
        self.manager.write().await.remove_account(account_id).await
    }

    async fn get_all_account_ids(&self) -> Vec<u32> {
        self.manager.read().await.get_all().await
    }

    async fn get_account_info(&self, account_id: u32) -> Result<Account> {
        let context_option = self.manager.read().await.get_account(account_id).await;
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
        for id in self.manager.read().await.get_all().await {
            let context_option = self.manager.read().await.get_account(id).await;
            if let Some(ctx) = context_option {
                accounts.push(Account::from_context(id, &ctx).await?)
            } else {
                println!("account with id {} doesn't exist anymore", id);
            }
        }
        Ok(accounts)
    }

    async fn select_account(&self, id: u32) -> Result<()> {
        self.manager.write().await.select_account(id).await
    }

    async fn get_selected_account_id(&self) -> Option<u32> {
        self.manager.read().await.get_selected_account_id().await
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
        sc.stop_io().await;
        sc.configure().await?;
        sc.start_io().await;
        Ok(())
    }

    /// Signal an ongoing process to stop.
    async fn sc_stop_ongoing_process(&self) -> Result<()> {
        let sc = self.selected_context().await?;
        sc.stop_ongoing().await;
        Ok(())
    }

    // ---------------------------------------------
    //                  autocrypt
    // ---------------------------------------------

    async fn sc_autocrypt_initiate_key_transfer(&self) -> Result<String> {
        let sc = self.selected_context().await?;
        deltachat::imex::initiate_key_transfer(&sc).await
    }

    async fn sc_autocrypt_continue_key_transfer(
        &self,
        message_id: u32,
        setup_code: String,
    ) -> Result<()> {
        let sc = self.selected_context().await?;
        deltachat::imex::continue_key_transfer(&sc, MsgId::new(message_id), &setup_code).await
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
        FullChat::from_dc_chat_id(&sc, chat_id).await
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
        MessageObject::from_message_id(&sc, message_id).await
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
                MessageObject::from_message_id(&sc, message_id).await?,
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
            &sc,
            deltachat::contact::Contact::get_by_id(&sc, contact_id).await?,
        )
        .await
    }

    /// Add a single contact as a result of an explicit user action.
    ///
    /// Returns contact id of the created or existing contact
    async fn sc_contacts_create_contact(&self, email: String, name: Option<String>) -> Result<u32> {
        let sc = self.selected_context().await?;
        if !may_be_valid_addr(&email) {
            bail!(anyhow!(
                "provided email address is not a valid email address"
            ))
        }
        Contact::create(&sc, &name.unwrap_or_default(), &email).await
    }

    /// Returns contact id of the created or existing DM chat with that contact
    async fn sc_contacts_create_chat_by_contact_id(&self, contact_id: u32) -> Result<u32> {
        let sc = self.selected_context().await?;
        let contact = Contact::get_by_id(&sc, contact_id).await?;
        ChatId::create_for_contact(&sc, contact.id)
            .await
            .map(|id| id.to_u32())
    }

    async fn sc_contacts_block(&self, contact_id: u32) -> Result<()> {
        let sc = self.selected_context().await?;
        Contact::block(&sc, contact_id).await
    }

    async fn sc_contacts_unblock(&self, contact_id: u32) -> Result<()> {
        let sc = self.selected_context().await?;
        Contact::unblock(&sc, contact_id).await
    }

    async fn sc_contacts_get_blocked(&self) -> Result<Vec<ContactObject>> {
        let sc = self.selected_context().await?;
        let blocked_ids = Contact::get_all_blocked(&sc).await?;
        let mut contacts: Vec<ContactObject> = Vec::with_capacity(blocked_ids.len());
        for id in blocked_ids {
            contacts.push(
                ContactObject::from_dc_contact(
                    &sc,
                    deltachat::contact::Contact::get_by_id(&sc, id).await?,
                )
                .await?,
            );
        }
        Ok(contacts)
    }

    async fn sc_contacts_get_contact_ids(
        &self,
        list_flags: u32,
        query: Option<String>,
    ) -> Result<Vec<u32>> {
        let sc = self.selected_context().await?;
        Contact::get_all(&sc, list_flags, query).await
    }

    // formerly called getContacts2 in desktop
    async fn sc_contacts_get_contacts(
        &self,
        list_flags: u32,
        query: Option<String>,
    ) -> Result<Vec<ContactObject>> {
        let sc = self.selected_context().await?;
        let contact_ids = Contact::get_all(&sc, list_flags, query).await?;
        let mut contacts: Vec<ContactObject> = Vec::with_capacity(contact_ids.len());
        for id in contact_ids {
            contacts.push(
                ContactObject::from_dc_contact(
                    &sc,
                    deltachat::contact::Contact::get_by_id(&sc, id).await?,
                )
                .await?,
            );
        }
        Ok(contacts)
    }

    async fn sc_contacts_get_contacts_by_ids(
        &self,
        ids: Vec<u32>,
    ) -> Result<HashMap<u32, ContactObject>> {
        let sc = self.selected_context().await?;

        let mut contacts = HashMap::with_capacity(ids.len());
        for id in ids {
            contacts.insert(
                id,
                ContactObject::from_dc_contact(
                    &sc,
                    deltachat::contact::Contact::get_by_id(&sc, id).await?,
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
    async fn sc_misc_send_text_message(&self, text: String, chat_id: u32) -> Result<u32> {
        let sc = self.selected_context().await?;

        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some(text));

        let message_id = deltachat::chat::send_msg(&sc, ChatId::new(chat_id), &mut msg).await?;
        Ok(message_id.to_u32())
    }
}
