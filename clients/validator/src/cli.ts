import ValidatorClient from "./index";
import inquirer from "inquirer";
// This script runs a CLI to consume the Validator and provide mixnet information to the user

const VALIDATOR_URLS: string[] = [
  "https://testnet-milhon-validator1.nymtech.net",
  "https://testnet-milhon-validator2.nymtech.net",
];
const DENOM = "punk";
const MOCK_MNEMONIC =
  "vault risk throw flat garlic pretty clay senior birth correct panic floor around pen horror mail entry arrest zoo devote message evoke street total";
type AccountType = {
  addr: string;
  client: any;
  mnemonic: string;
};
function validatorCli() {
  // define funcs to be used in CLI switch-case

  let state: AccountType = {
    addr: "",
    client: null,
    mnemonic: "",
  };

  function restartApp() {
    setTimeout(() => {
      validatorCli();
    }, 300);
  }

  function generateNewAccount() {
    const mnemonic = ValidatorClient.randomMnemonic();
    ValidatorClient.mnemonicToAddress(mnemonic, "punk")
      .then((address) => {
        console.log("Your address is: ", address);
        console.log("Your mnemonic is: ", mnemonic);
        return address;
      })
      .catch((err) => {
        console.log("err", err);
      });
    restartApp();
  }

  function startTransactionMenu() {
    inquirer
      .prompt([
        {
          type: "list",
          name: "task",
          message: "What now?",
          choices: ["send_funds"],
        },
      ])
      .then(({ task }) => {
        switch (task) {
          case "send_funds":
            console.log("sending funds from ", state.addr);
            break;
          default:
            return null;
        }
      });
  }
  function connectAccount() {
    inquirer
      .prompt([
        {
          name: "user_mnemonic",
          type: "input",
          message: "please enter your mnemonic:",
        },
      ])
      .then(async ({ user_mnemonic }) => {
        console.log("Connecting...");
        const addr = await ValidatorClient.mnemonicToAddress(
          MOCK_MNEMONIC,
          // user_mnemonic,
          "punk"
        );

        console.log("Decryped address:", addr);
        const client = await ValidatorClient.connect(
          addr,
          MOCK_MNEMONIC,
          VALIDATOR_URLS,
          DENOM
        );

        state = {
          addr,
          mnemonic: MOCK_MNEMONIC,
          client,
        };

        const balance = await client.getBalance(addr);
        console.log(`connected to validator, our address is ${client.address}`);
        console.log("connected to validator", client.urls[0]);
        console.log("📫 Your address:", addr);
        console.log("💰 Your balance:", balance);

        startTransactionMenu();
      })
      .catch((err) => {
        console.log("error: ", err);
      });
  }

  // app provides a list of possible tasks
  inquirer
    .prompt([
      {
        type: "list",
        name: "task",
        message: "So...What would you like to do today?",
        choices: [
          "create_account",
          "connect_account",
          "get_mixnodes",
          "send_funds",
        ],
      },
    ])
    .then(({ task }) => {
      switch (task) {
        case "create_account":
          generateNewAccount();
          break;
        case "connect_account":
          connectAccount();
          break;
        default:
          return null;
      }
    });
}

validatorCli();
// if it's get mixnodes, return all mixnodes
// if it's create an account
// if it's send funds from one address to another
