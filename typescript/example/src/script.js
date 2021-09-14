//@ts-check

import { DeltaChat } from "../../dist/deltachat.js";
import { h } from "./util";

Promise.prototype["log"] = function () {
  this.then(console.log).catch(console.error);
  return this;
};

const dc = new DeltaChat("ws://localhost:20808/api_ws");

console.log({ dc });
window["dc"] = dc;

const resultDiv = document.getElementById("result");

function logEvent(logFn) {
  return (event) =>
    logFn(`[AC ${event.contextId}]`, event.field1, event.field2);
}

dc.on("INFO", logEvent(console.info));
dc.on("WARNING", logEvent(console.warn));
dc.on("ERROR", logEvent(console.error));
dc.on("ERROR_SELF_NOT_IN_GROUP", logEvent(console.error));
dc.on("CONNECTIVITY_CHANGED", console.info.bind(null, "Connectivity Changed"));
// possibly also log to webview in resultDiv

function addResultEntry(message) {
  const node = document.createElement("p");
  node.innerText = message;
  resultDiv.prepend(node);
}

document.getElementById("connect").onclick = async (ev) => {
  ev.target["disabled"] = true;
  try {
    await dc.connect();
    document.getElementById("connection_methods").style.display = "";
  } catch (error) {
    console.error("connection failed", error);
    addResultEntry(error);
    ev.target["disabled"] = false;
  }
};

dc.on("socket_connection_change", (isConnected) => {
  // console.info("socket_connection_change", {isConnected})
  document.getElementById("connect")["disabled"] = isConnected;
  if (!isConnected) {
    addResultEntry("🔌 connection to backend lost 🔌");
  } else {
    addResultEntry("🔌 connection to backend established 🔌");
  }
});

const ec_input = document.getElementById("email_check_input");
const ex_result = document.getElementById("email_check_result");
document.getElementById("email_check_button").onclick = async (ev) => {
  ex_result.innerText = "";
  try {
    const valid = await dc.raw_api.check_email_validity(ec_input["value"]);
    ex_result.innerText = valid ? "YES" : "NO";
  } catch (error) {
    addResultEntry(error);
  }
};

async function getAccounts() {
  const accounts_container = document.getElementById("accounts");

  const selectAccount = async (id) => {
    try {
      await dc.raw_api.select_account(id);
      await getAccounts();
    } catch (error) {
      addResultEntry(error);
    }
  };

  const removeAccount = async (id) => {
    try {
      if (!confirm(`Delete Account ${id}?`)) {
        return;
      }
      await dc.raw_api.remove_account(id);
      await getAccounts();
    } catch (error) {
      addResultEntry(error);
    }
  };

  let accounts = await dc.raw_api.get_all_accounts();
  let selected_account = await dc.raw_api.get_selected_account_id();
  const account_elements = accounts.map((account) => {
    let is_selected = account.id == selected_account;
    const selectAccountButton = h("button", "select");
    selectAccountButton.onclick = selectAccount.bind(null, account.id);
    const removeAccountButton = h("button", "remove");
    removeAccountButton.onclick = removeAccount.bind(null, account.id);
    if (account.type == "configured") {
      let avatar;
      if (account.display_name) {
        avatar = h("img", null, "avatar");
        avatar.src = account.profile_image;
      } else {
        const nameOrAddr = account.display_name || account.addr;
        const codepoint = nameOrAddr && nameOrAddr.codePointAt(0);
        avatar = h(
          "div",
          codepoint ? String.fromCodePoint(codepoint).toUpperCase() : "#",
          "avatar"
        );
        avatar.style["background"] = account.color;
      }

      return h(
        "div",
        [
          h("div", account.id.toString(), "id"),
          avatar,
          h("p", account.display_name, "title"),
          h("p", account.addr, "subtitle"),
          selectAccountButton,
          removeAccountButton,
        ],
        `account${is_selected ? " selected" : ""}`
      );
    } else {
      return h(
        "div",
        [
          h("div", account.id.toString(), "id"),
          h("div", account.id.toString(), "avatar"),
          h("p", "Unconfigured", "title"),
          selectAccountButton,
          removeAccountButton,
        ],
        `account${is_selected ? " selected" : ""}`
      );
    }
  });
  const deselectAccountsButton = h("button", "deselect account");
  deselectAccountsButton.onclick = selectAccount.bind(null, 0);
  const newAccountsButton = h("button", "new account");
  newAccountsButton.onclick = async () => {
    try {
      const id_of_new_account = await dc.raw_api.add_account();
      addResultEntry(`created new account with id: ${id_of_new_account}`);
      await getAccounts();
    } catch (error) {
      addResultEntry(error);
    }
  };

  accounts_container.innerText = "";
  accounts_container.append(
    h(
      "div",
      accounts.length > 0 ? account_elements : h("h4", "No accounts"),
      "accounts_box"
    )
  );
  if (accounts.length > 0) {
    accounts_container.append(deselectAccountsButton);
  }
  accounts_container.append(newAccountsButton);
}

document.getElementById("get_accounts_button").onclick = async () => {
  try {
    await getAccounts();
  } catch (error) {
    addResultEntry(error);
  }
};

// window.bench = async iterations => {
//   const unique = Number(Math.floor(Math.random() * 1000000)).toString(36);
//   const label = "bench" + unique;
//   const t1 = Date.now();
//   console.time(label);
//   for (let i = 0; i < iterations; i++) {
//     await dc.add(1, 4);
//   }
//   console.timeEnd(label);
//   const t2 = Date.now();
//   console.log((t2 - t1) / iterations);
// };

// window.pbench = async iterations => {
//   const unique = Number(Math.floor(Math.random() * 1000000)).toString(36);
//   const label = "start" + unique;
//   const label2 = "result" + unique;
//   const t1 = Date.now();
//   const promises = [];
//   console.time(label);
//   for (let i = 0; i < iterations; i++) {
//     promises.push(dc.add(1, 4));
//   }
//   console.timeEnd(label);
//   console.time(label2);
//   await Promise.all(promises);
//   console.timeEnd(label2);
//   const t2 = Date.now();
//   console.log((t2 - t1) / iterations);
// };

// document.getElementById("info").onclick = async () => {
//   let info = await dc.context.getInfo();
//   resultDiv.prepend(
//     h("table", [
//       h("thead", [h("tr", [h("td", "property"), h("td", "value")])]),
//       h(
//         "tbody",
//         Object.keys(info)
//           .sort()
//           .map(key =>
//             h("tr", [h("td", key.replace(/_/g, " ")), h("td", info[key])])
//           )
//       )
//     ])
//   );
// };

// document.getElementById("getChatList").onclick = async () => {
//   let ids = await dc.context.chatList.getChatListIds(0);
//   console.log({ ids });
//   resultDiv.prepend(
//     h("p", [h("b", "chatlistids:"), JSON.stringify(ids)])
//   );
//   const res = await dc.context.chatList.getChatListItemsByIds(ids);
//   console.log(res);
//   resultDiv.prepend(h("p", JSON.stringify({ res })));

//   resultDiv.prepend(
//     h(
//       "ul",
//       Object.keys(res)
//         .map(r => JSON.stringify({ ...r }))
//         .map(c => h("li", c))
//     )
//   );
// };
