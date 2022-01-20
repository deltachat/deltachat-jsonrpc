import { RawClient, RPC } from "./src/lib";
import { eventIdToName } from "./src/events";
import { WebsocketClient } from "yerpc";

type DeltaEvent = { id: number; contextId: number; field1: any; field2: any };
var selectedAccount = 0;
window.addEventListener("DOMContentLoaded", (_event) => {
  (window as any).selectDeltaAccount = (id: string) => {
    selectedAccount = Number(id);
    window.dispatchEvent(new Event("account-changed"));
  };
  run().catch((err) => console.error("run failed", err));
});

async function run() {
  const $main = document.getElementById("main")!;
  const $side = document.getElementById("side")!;
  const $head = document.getElementById("header")!;

  const transport = new WebsocketClient("ws://localhost:20808/ws");
  const client = new RawClient(transport);

  transport.addEventListener("request", (event: Event) => {
    const request = (event as MessageEvent<RPC.Request>).data;
    const method = request.method;
    if (method === "event") {
      const params = request.params! as DeltaEvent;
      const name = eventIdToName(params.id);
      onIncomingEvent(params, name);
    }
  });

  window.addEventListener("account-changed", async (_event: Event) => {
    await client.selectAccount(selectedAccount);
    listChatsForSelectedAccount();
  });

  await Promise.all([loadAccountsInHeader(), listChatsForSelectedAccount()]);

  async function loadAccountsInHeader() {
    const accounts = await client.getAllAccounts();
    for (const account of accounts) {
      if (account.type === "Configured") {
        write(
          $head,
          `<a href="#" onclick="selectDeltaAccount(${account.id})">
          ${account.addr!}
          </a>&nbsp;`,
        );
      }
    }
  }

  async function listChatsForSelectedAccount() {
    clear($main);
    const selectedAccount = await client.getSelectedAccountId();
    if (!selectedAccount) return write($main, "No account selected");
    const info = await client.getAccountInfo(selectedAccount);
    if (info.type !== "Configured") {
      return write($main, "Account is not configured");
    }
    write($main, `<h2>${info.addr!}</h2>`);
    const chats = await client.scGetChatlistEntries(0, null, null);
    for (const [chatId, _messageId] of chats) {
      const chat = await client.scChatlistGetFullChatById(chatId);
      write($main, `<h3>${chat.name}</h3>`);
      const messageIds = await client.scMessageListGetMessageIds(chatId, 0);
      const messages = await client.scMessageGetMessages(messageIds);
      for (const [_messageId, message] of Object.entries(messages)) {
        write($main, `<p>${message.text}</p>`);
      }
    }
  }

  function onIncomingEvent(event: DeltaEvent, name: string) {
    write(
      $side,
      `
        <p class="message">
          [<strong>${name}</strong> on account ${event.contextId}]<br>
          <em>f1:</em> ${JSON.stringify(event.field1)}<br>
          <em>f2:</em> ${JSON.stringify(event.field2)}
        </p>`,
    );
  }
}

function write(el: HTMLElement, html: string) {
  el.innerHTML += html;
}
function clear(el: HTMLElement) {
  el.innerHTML = "";
}
