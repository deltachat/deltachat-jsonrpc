import { strictEqual } from "assert";
import chai, { assert, expect } from "chai";

import { DeltaChat } from "..";

import {
  CMD_API_Server_Handle,
  CMD_API_SERVER_PORT,
  startCMD_API_Server,
} from "./test_base";

describe("basic tests", () => {
  let server_handle: CMD_API_Server_Handle;
  const dc = new DeltaChat(
    "ws://localhost:" + CMD_API_SERVER_PORT + "/api_ws",
    "silent",
  );

  before(async () => {
    server_handle = await startCMD_API_Server(CMD_API_SERVER_PORT);
    // make sure server is up by the time we continue
    await new Promise((res) => setTimeout(res, 100));
  });

  after(async () => {
    await server_handle.close();
  });

  it("connect", async () => {
    await dc.connect();
  });

  it("check email", async () => {
    const positive_test_cases = [
      "email@example.com",
      "36aa165ae3406424e0c61af17700f397cad3fe8ab83d682d0bddf3338a5dd52e@yggmail@yggmail",
    ];
    const negative_test_cases = ["email@", "example.com", "emai221"];

    expect(
      await Promise.all(
        positive_test_cases.map((email) =>
          dc.raw_api.check_email_validity(email)
        ),
      ),
    ).to.not.contain(false);

    expect(
      await Promise.all(
        negative_test_cases.map((email) =>
          dc.raw_api.check_email_validity(email)
        ),
      ),
    ).to.not.contain(true);
  });

  it("system info", async () => {
    const system_info = await dc.raw_api.get_system_info();
    expect(system_info).to.contain.keys([
      "arch",
      "num_cpus",
      "deltachat_core_version",
      "sqlite_version",
    ]);
  });

  describe("account managment", () => {
    it("should create account", async () => {
      await dc.raw_api.add_account();
      assert((await dc.raw_api.get_all_account_ids()).length === 1);
    });

    it("should remove the account again", async () => {
      await dc.raw_api.remove_account(
        (
          await dc.raw_api.get_all_account_ids()
        )[0],
      );
      assert((await dc.raw_api.get_all_account_ids()).length === 0);
    });

    it("should create multiple accounts", async () => {
      await dc.raw_api.add_account();
      await dc.raw_api.add_account();
      await dc.raw_api.add_account();
      await dc.raw_api.add_account();
      assert((await dc.raw_api.get_all_account_ids()).length === 4);
    });
  });

  describe("contact managment", function () {
    // could this also be done in an offline test?
    before(async () => {
      await dc.raw_api.select_account(await dc.raw_api.add_account());
    });
    it("block and unblock contact", async function () {
      const contactId = await dc.raw_api.sc_contacts_create_contact(
        "example@delta.chat",
        null,
      );
      expect((await dc.raw_api.sc_contacts_get_contact(contactId)).is_blocked)
        .to.be.false;
      await dc.raw_api.sc_contacts_block(contactId);
      expect((await dc.raw_api.sc_contacts_get_contact(contactId)).is_blocked)
        .to.be.true;
      expect(await dc.raw_api.sc_contacts_get_blocked()).to.have.length(1);
      await dc.raw_api.sc_contacts_unblock(contactId);
      expect((await dc.raw_api.sc_contacts_get_contact(contactId)).is_blocked)
        .to.be.false;
      expect(await dc.raw_api.sc_contacts_get_blocked()).to.have.length(0);
    });
  });
});
