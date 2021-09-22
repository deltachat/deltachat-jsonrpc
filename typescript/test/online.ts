import { assert } from "chai";
import { DeltaChat } from "../dist/deltachat";
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
  });

  it.skip("send and recieve text message", async function () {
    this.timeout(6000);
    await dc.raw_api.select_account(1);

    // todo when we have functions to create contact and chat with that contact
  });

  it("assert(true)", async () => {
    assert(true);
  });
});
