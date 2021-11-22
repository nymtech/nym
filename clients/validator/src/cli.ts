import ValidatorClient from "./index";
import inquirer from "inquirer";
// This script runs a CLI to consume the Validator and provide mixnet information to the user

const VALIDATOR_URLS: string[] = [
  "https://testnet-milhon-validator1.nymtech.net",
  // "https://testnet-milhon-validator2.nymtech.net", // <-- val 2 doesnt work apparently.
];
const DENOM = "punk";
const MOCK_MNEMONIC =
  "vault risk throw flat garlic pretty clay senior birth correct panic floor around pen horror mail entry arrest zoo devote message evoke street total";
// ^^ addr: punk10dxwmqjy72s9nkm9x9pluyn6pyx0gkptjhs4k9
// curr balance: 899999747

// const MOCK_MNEMONIC =
//   "oil once motion cute crawl patch happy wave donkey zoo retreat matrix emerge adult very universe aware error snap credit actress couple upset engine";
// ^^ addr: punk1yzr7gtmtlfd0s7s9wpexhteeu05y4xlcvh65eh
// curr balance: 5045 UPUNK

// const MOCK_MNEMONIC =
//   "sample menu edit midnight guard review call record horn antenna stairs awkward fringe document during amazing twelve wise wide escape matter betray staff someone";
// ^^ addr: punk1wn8lwxe5hvdtx60c6p7ekskmu75agwfrslf0qs
// curr balance:

type AccountType = {
  addr: string;
  client: any;
  mnemonic?: string;
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

  function sendFundsMenu() {
    inquirer
      .prompt([
        {
          name: "recipient",
          type: "input",
          message: "please enter the receipient:",
        },
        {
          name: "amount",
          type: "input",
          message: "please enter the amount (UPUNK):",
        },
      ])
      .then(async ({ recipient, amount }) => {
        const { addr, client } = state;
        console.log(
          `🔥 Hold Tight - Sending ${amount}UPUNK to ${recipient} 🚀`
        );

        const res = await client.send(addr, recipient, [
          {
            denom: "upunk",
            amount: amount,
          },
        ]);
        console.log("Funds Transfer Response:", res);
        restartApp();
      });
  }

  async function delegateGateway() {
    console.log(
      "unfortunately - gateway delegation is switched off at the moment."
    );
    startTransactionMenu();
    // const id = "punk1yzr7gtmtlfd0s7s9wpexhteeu05y4xlcvh65eh";
    // const gatewayID = "EQhjPpUuy4i1u87nfQMW21WiBT5mJk4dcq4ju7Vct7cB";
    // const coin = {
    //   denom: "upunk",
    //   amount: "101",
    // };
    // const res = await state.client.delegateToMixnode(gatewayID, coin);
    // console.log("delegateMixnode ==> ", res);
  }

  async function delegateMixnode() {
    const mixNodeID = "2cFpCe7yP79CcuRpf6JBRdJaSp7JF5YcA5SHi8JVm1d2";
    // const mixNodeID = "2Vrr7s2peGiWsPh6xY3ZFEMDRmMNv8xLBUtV5XMyQLSB";
    const coin = {
      denom: "upunk",
      amount: "1001",
    };
    const res = await state.client.delegateToMixnode(mixNodeID, coin);
    console.log("delegate to mixnode response: ", res);
  }
  async function findMinimumMixnodeBond() {
    const res = await state.client.minimumMixnodeBond();
    console.log("res is back ", res);
  }

  async function bondMixnode() {
    state.client.bondMixnode();
  }

  async function checkOwnsMixnodes() {
    const res = await state.client.ownsMixNode();
    console.log("owns mixnode? ", res);
  }
  function startTransactionMenu() {
    inquirer
      .prompt([
        {
          type: "list",
          name: "task",
          message: "What now?",
          choices: [
            "send_funds",
            "get_mixnodes",
            "refresh_mixnodes",
            "refresh_val_api_mixnodes",
            "min_mixn_bond",
            "bond_mixnode",
            "delegate_mixnode",
            "delegate_gateway",
            "check_owns_mixnode",
          ],
        },
      ])
      .then(({ task }) => {
        switch (task) {
          case "send_funds":
            sendFundsMenu();
            break;
          case "get_mixnodes":
            getMixnodes();
            break;
          case "refresh_mixnodes":
            refreshMixnodes();
            break;
          case "refresh_val_api_mixnodes":
            refreshValApiMixnodes();
            break;
          case "min_mixn_bond":
            findMinimumMixnodeBond();
            break;
          case "bond_mixnode":
            bondMixnode();
            break;
          case "delegate_gateway":
            delegateGateway();
            break;
          case "delegate_mixnode":
            delegateMixnode();
            break;
          case "check_owns_mixnode":
            checkOwnsMixnodes();
            break;
          default:
            return null;
        }
      });
  }

  function queryUserAccount() {
    inquirer
      .prompt([
        {
          type: "input",
          name: "query_user",
          message: "Please enter the public address of user you wish to query",
        },
      ])
      .then(async ({ query_user }) => {
        let response = "";
        try {
          const client = await ValidatorClient.connectForQuery(
            query_user,
            VALIDATOR_URLS,
            DENOM
          );
          const balance = await client.getBalance(query_user);
          response = `User ${query_user} has a balance of ${balance?.amount}${balance?.denom}`;
          console.log(response);
          return validatorCli();
        } catch (error) {
          console.log("error back ", error);
          return validatorCli();
        }
      });
  }

  async function refreshMixnodes() {
    const res = await state.client.refreshMixNodes(
      "punk1yksauczytk60x5cejaras8w6nwf7r772n3kwkp"
    );
    console.log("done:", res);
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
        console.log(
          `💰 Your balance is ${balance?.amount}${balance?.denom.toUpperCase()}`
        );

        startTransactionMenu();
      })
      .catch((err) => {
        console.log("error: ", err);
      });
  }
  function buildAWallet() {
    inquirer
      .prompt([
        {
          message: "enter your mnemonic to build wallet:",
          type: "input",
          name: "mnemonic",
        },
      ])
      .then(async ({ mnemonic }) => {
        const res = await ValidatorClient.buildWallet(mnemonic, DENOM);
        console.log("Build_Wallet Response: ", res);
      });
  }
  async function refreshValApiMixnodes() {
    const res = await state.client.refreshValidatorAPIMixNodes();
    console.log("res is back: ", res);
  }
  function getMixnodes() {
    const res = state.client.mixNodesCache;
    console.log("Mixnodes", res);
  }
  // app provides a list of possible tasks
  inquirer
    .prompt([
      {
        type: "list",
        name: "task",
        message: "Yo, What would you like to do today?",
        choices: [
          "create_account",
          "connect_account",
          "build_wallet",
          "query_user",
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
        case "build_wallet":
          buildAWallet();
          break;
        case "query_user":
          queryUserAccount();
          break;
        default:
          return null;
      }
    });
}

validatorCli();
