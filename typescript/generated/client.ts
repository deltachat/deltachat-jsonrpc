import * as T from "./types.js";
import * as RPC from "./jsonrpc.js";

type RequestMethod = (method: string, params?: RPC.Params) => Promise<unknown>;
type NotificationMethod = (method: string, params?: RPC.Params) => void;
interface Transport {
  request: RequestMethod;
  notification: NotificationMethod;
}

export class RawClient {
  private _request: RequestMethod;
  private _notification: NotificationMethod;

  constructor(transport: Transport) {
    this._request = transport.request.bind(transport);
    this._notification = transport.notification.bind(transport);
  }

  public checkEmailValidity(email: string): Promise<boolean> {
    return (this._request(
      "check_email_validity",
      [email] as RPC.Params,
    )) as Promise<boolean>;
  }
  public getSystemInfo(): Promise<Record<string, string>> {
    return (this._request("get_system_info", [] as RPC.Params)) as Promise<
      Record<string, string>
    >;
  }
  public scGetProviderInfo(email: string): Promise<(T.ProviderInfo | null)> {
    return (this._request(
      "sc_get_provider_info",
      [email] as RPC.Params,
    )) as Promise<(T.ProviderInfo | null)>;
  }
  public addAccount(): Promise<T.U32> {
    return (this._request("add_account", [] as RPC.Params)) as Promise<T.U32>;
  }
  public removeAccount(accountId: T.U32): Promise<null> {
    return (this._request(
      "remove_account",
      [accountId] as RPC.Params,
    )) as Promise<null>;
  }
  public getAllAccountIds(): Promise<(T.U32)[]> {
    return (this._request("get_all_account_ids", [] as RPC.Params)) as Promise<
      (T.U32)[]
    >;
  }
  public getAccountInfo(accountId: T.U32): Promise<T.Account> {
    return (this._request(
      "get_account_info",
      [accountId] as RPC.Params,
    )) as Promise<T.Account>;
  }
  public getAllAccounts(): Promise<(T.Account)[]> {
    return (this._request("get_all_accounts", [] as RPC.Params)) as Promise<
      (T.Account)[]
    >;
  }
  public selectAccount(id: T.U32): Promise<null> {
    return (this._request("select_account", [id] as RPC.Params)) as Promise<
      null
    >;
  }
  public getSelectedAccountId(): Promise<(T.U32 | null)> {
    return (this._request(
      "get_selected_account_id",
      [] as RPC.Params,
    )) as Promise<(T.U32 | null)>;
  }
  public scIsConfigured(): Promise<boolean> {
    return (this._request("sc_is_configured", [] as RPC.Params)) as Promise<
      boolean
    >;
  }
  public scGetInfo(): Promise<Record<string, string>> {
    return (this._request("sc_get_info", [] as RPC.Params)) as Promise<
      Record<string, string>
    >;
  }
  public scSetConfig(key: string, value: (string | null)): Promise<null> {
    return (this._request(
      "sc_set_config",
      [key, value] as RPC.Params,
    )) as Promise<null>;
  }
  public scGetConfig(key: string): Promise<(string | null)> {
    return (this._request("sc_get_config", [key] as RPC.Params)) as Promise<
      (string | null)
    >;
  }
  public scBatchGetConfig(
    keys: (string)[],
  ): Promise<Record<string, (string | null)>> {
    return (this._request(
      "sc_batch_get_config",
      [keys] as RPC.Params,
    )) as Promise<Record<string, (string | null)>>;
  }
  public scConfigure(): Promise<null> {
    return (this._request("sc_configure", [] as RPC.Params)) as Promise<null>;
  }
  public scStopOngoingProcess(): Promise<null> {
    return (this._request(
      "sc_stop_ongoing_process",
      [] as RPC.Params,
    )) as Promise<null>;
  }
  public scAutocryptInitiateKeyTransfer(): Promise<string> {
    return (this._request(
      "sc_autocrypt_initiate_key_transfer",
      [] as RPC.Params,
    )) as Promise<string>;
  }
  public scAutocryptContinueKeyTransfer(
    messageId: T.U32,
    setupCode: string,
  ): Promise<null> {
    return (this._request(
      "sc_autocrypt_continue_key_transfer",
      [messageId, setupCode] as RPC.Params,
    )) as Promise<null>;
  }
  public scGetChatlistEntries(
    listFlags: T.U32,
    queryString: (string | null),
    queryContactId: (T.U32 | null),
  ): Promise<(T.ChatListEntry)[]> {
    return (this._request(
      "sc_get_chatlist_entries",
      [listFlags, queryString, queryContactId] as RPC.Params,
    )) as Promise<(T.ChatListEntry)[]>;
  }
  public scGetChatlistItemsByEntries(
    entries: (T.ChatListEntry)[],
  ): Promise<Record<T.U32, T.ChatListItemFetchResult>> {
    return (this._request(
      "sc_get_chatlist_items_by_entries",
      [entries] as RPC.Params,
    )) as Promise<Record<T.U32, T.ChatListItemFetchResult>>;
  }
  public scChatlistGetFullChatById(chatId: T.U32): Promise<T.FullChat> {
    return (this._request(
      "sc_chatlist_get_full_chat_by_id",
      [chatId] as RPC.Params,
    )) as Promise<T.FullChat>;
  }
  public scAcceptChat(chatId: T.U32): Promise<null> {
    return (this._request("sc_accept_chat", [chatId] as RPC.Params)) as Promise<
      null
    >;
  }
  public scBlockChat(chatId: T.U32): Promise<null> {
    return (this._request("sc_block_chat", [chatId] as RPC.Params)) as Promise<
      null
    >;
  }
  public scMessageListGetMessageIds(
    chatId: T.U32,
    flags: T.U32,
  ): Promise<(T.U32)[]> {
    return (this._request(
      "sc_message_list_get_message_ids",
      [chatId, flags] as RPC.Params,
    )) as Promise<(T.U32)[]>;
  }
  public scMessageGetMessage(messageId: T.U32): Promise<T.Message> {
    return (this._request(
      "sc_message_get_message",
      [messageId] as RPC.Params,
    )) as Promise<T.Message>;
  }
  public scMessageGetMessages(
    messageIds: (T.U32)[],
  ): Promise<Record<T.U32, T.Message>> {
    return (this._request(
      "sc_message_get_messages",
      [messageIds] as RPC.Params,
    )) as Promise<Record<T.U32, T.Message>>;
  }
  public scContactsGetContact(contactId: T.U32): Promise<T.Contact> {
    return (this._request(
      "sc_contacts_get_contact",
      [contactId] as RPC.Params,
    )) as Promise<T.Contact>;
  }
  public scContactsCreateContact(
    email: string,
    name: (string | null),
  ): Promise<T.U32> {
    return (this._request(
      "sc_contacts_create_contact",
      [email, name] as RPC.Params,
    )) as Promise<T.U32>;
  }
  public scContactsCreateChatByContactId(contactId: T.U32): Promise<T.U32> {
    return (this._request(
      "sc_contacts_create_chat_by_contact_id",
      [contactId] as RPC.Params,
    )) as Promise<T.U32>;
  }
  public scContactsBlock(contactId: T.U32): Promise<null> {
    return (this._request(
      "sc_contacts_block",
      [contactId] as RPC.Params,
    )) as Promise<null>;
  }
  public scContactsUnblock(contactId: T.U32): Promise<null> {
    return (this._request(
      "sc_contacts_unblock",
      [contactId] as RPC.Params,
    )) as Promise<null>;
  }
  public scContactsGetBlocked(): Promise<(T.Contact)[]> {
    return (this._request(
      "sc_contacts_get_blocked",
      [] as RPC.Params,
    )) as Promise<(T.Contact)[]>;
  }
  public scContactsGetContactIds(
    listFlags: T.U32,
    query: (string | null),
  ): Promise<(T.U32)[]> {
    return (this._request(
      "sc_contacts_get_contact_ids",
      [listFlags, query] as RPC.Params,
    )) as Promise<(T.U32)[]>;
  }
  public scContactsGetContacts(
    listFlags: T.U32,
    query: (string | null),
  ): Promise<(T.Contact)[]> {
    return (this._request(
      "sc_contacts_get_contacts",
      [listFlags, query] as RPC.Params,
    )) as Promise<(T.Contact)[]>;
  }
  public scContactsGetContactsByIds(
    ids: (T.U32)[],
  ): Promise<Record<T.U32, T.Contact>> {
    return (this._request(
      "sc_contacts_get_contacts_by_ids",
      [ids] as RPC.Params,
    )) as Promise<Record<T.U32, T.Contact>>;
  }
  public scMiscSendTextMessage(text: string, chatId: T.U32): Promise<T.U32> {
    return (this._request(
      "sc_misc_send_text_message",
      [text, chatId] as RPC.Params,
    )) as Promise<T.U32>;
  }
}
