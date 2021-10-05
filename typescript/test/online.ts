import { assert, expect } from "chai";
import { DeltaChat } from "../dist/deltachat";
import { Event_TypeID, get_event_name_from_id } from "../dist/events";
import {
  CMD_API_Server_Handle,
  CMD_API_SERVER_PORT,
  createTempUser,
  startCMD_API_Server,
} from "./test_base";

describe("online tests", function () {
  let server_handle: CMD_API_Server_Handle;
  const dc = new DeltaChat(
    "ws://localhost:" + CMD_API_SERVER_PORT + "/api_ws",
    "silent"
  );
  let account = null as any as { email: string; password: string };
  let account2 = null as any as { email: string; password: string };

  before(async function () {
    server_handle = await startCMD_API_Server(CMD_API_SERVER_PORT);

    if (!process.env.DCC_NEW_TMP_EMAIL) {
      console.log(
        "Missing DCC_NEW_TMP_EMAIL environment variable!, skip intergration tests"
      );
      this.skip();
    }

    account = await createTempUser(process.env.DCC_NEW_TMP_EMAIL);
    if (!account || !account.email || !account.password) {
      console.log(
        "We didn't got back an account from the api, skip intergration tests"
      );
      this.skip();
    }

    account2 = await createTempUser(process.env.DCC_NEW_TMP_EMAIL);
    if (!account2 || !account2.email || !account2.password) {
      console.log(
        "We didn't got back an account2 from the api, skip intergration tests"
      );
      this.skip();
    }
  });

  after(async () => {
    await server_handle.close();
  });

  it("should connect", async () => {
    await dc.connect();
  });

  let are_configured = false;

  it("configure test accounts", async function () {
    this.timeout(6000);
    await dc.raw_api.select_account(await dc.raw_api.add_account());

    await dc.raw_api.sc_set_config("addr", account.email);
    await dc.raw_api.sc_set_config("mail_pw", account.password);
    let configure_promise = dc.raw_api.sc_configure();

    await dc.raw_api.select_account(await dc.raw_api.add_account());

    await dc.raw_api.sc_set_config("addr", account2.email);
    await dc.raw_api.sc_set_config("mail_pw", account2.password);
    await Promise.all([configure_promise, dc.raw_api.sc_configure()]);

    are_configured = true;
  });

  it("send and recieve text message", async function () {
    if (!are_configured) {
      this.skip();
    }
    this.timeout(5000);

    await dc.raw_api.select_account(1);
    const contactId = await dc.raw_api.sc_contacts_create_contact(
      account2.email,
      null
    );
    const chatId = await dc.raw_api.sc_contacts_create_chat_by_contact_id(
      contactId
    );
    dc.raw_api.sc_misc_send_text_message("Hello", chatId);

    const { field1: chatIdOnAccountB } = await waitForEvent(
      dc,
      "INCOMING_MSG",
      2
    );

    await dc.raw_api.select_account(2);
    await dc.raw_api.sc_accept_chat(chatIdOnAccountB);
    const messageList = await dc.raw_api.sc_message_list_get_message_ids(
      chatIdOnAccountB,
      0
    );

    expect(messageList).have.length(1);
    const message = await dc.raw_api.sc_message_get_message(messageList[0]);
    expect(message.text).equal("Hello");
  });

  it("send and recieve text message roundtrip, encrypted on answer onwards", async function () {
    if (!are_configured) {
      this.skip();
    }
    this.timeout(5000);

    // send message from A to B
    await dc.raw_api.select_account(1);
    const contactId = await dc.raw_api.sc_contacts_create_contact(
      account2.email,
      null
    );
    const chatId = await dc.raw_api.sc_contacts_create_chat_by_contact_id(
      contactId
    );
    dc.raw_api.sc_misc_send_text_message("Hello2", chatId);
    // wait for message from A
    const event = await waitForEvent(dc, "INCOMING_MSG", 2);
    const { field1: chatIdOnAccountB } = event;

    await dc.raw_api.select_account(2);
    await dc.raw_api.sc_accept_chat(chatIdOnAccountB);
    const messageList = await dc.raw_api.sc_message_list_get_message_ids(
      chatIdOnAccountB,
      0
    );
    const message = await dc.raw_api.sc_message_get_message(
      messageList.reverse()[0]
    );
    expect(message.text).equal("Hello2");
    // Send message back from B to A
    dc.raw_api.sc_misc_send_text_message("super secret message", chatId);
    // Check if answer arives at A and if it is encrypted
    await waitForEvent(dc, "INCOMING_MSG", 1);
    await dc.raw_api.select_account(1);
    const messageId = (
      await dc.raw_api.sc_message_list_get_message_ids(chatId, 0)
    ).reverse()[0];
    const message2 = await dc.raw_api.sc_message_get_message(messageId);
    expect(message2.text).equal("super secret message");
    expect(message2.show_padlock).equal(true);
  });

});

type event_data = {
  contextId: number;
  id: Event_TypeID;
  [key: string]: any;
};
async function waitForEvent(
  dc: DeltaChat,
  event: ReturnType<typeof get_event_name_from_id>,
  accountId: number
): Promise<event_data> {
  return new Promise((res, rej) => {
    const callback = (ev: event_data) => {
      if (ev.contextId == accountId) {
        dc.removeListener(event, callback);
        res(ev);
      }
    };

    dc.on(event, callback);
  });
}
