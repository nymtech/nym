import init, { NymIssuanceTicketbook } from "@nymproject/nym-credential-proxy-lib-wasm";

const NYM_CREDENTIAL_PROXY_API = "http://localhost:8080";
const API_TOKEN = "foomp";

async function main() {
  await init();

  let cryptoData = new NymIssuanceTicketbook({});

  console.log("getting partial vks");
  const partialVksRes = await fetch(`${NYM_CREDENTIAL_PROXY_API}/api/v1/ticketbook/partial-verification-keys`, {
    headers: new Headers({ "Authorization": `Bearer ${API_TOKEN}` })
  });
  const partialVks = await partialVksRes.json();
  console.debug(partialVks);

  console.log("getting master vk");
  const masterVkRes = await fetch(`${NYM_CREDENTIAL_PROXY_API}/api/v1/ticketbook/master-verification-key`, {
    headers: new Headers({ "Authorization": `Bearer ${API_TOKEN}` })
  });
  const masterVk = await masterVkRes.json();
  console.debug(masterVk);

  let request = cryptoData.buildRequestPayload(false);
  console.log(request);


  console.log("getting blinded wallet shares");
  const sharesRes = await fetch(`${NYM_CREDENTIAL_PROXY_API}/api/v1/ticketbook/obtain?include-coin-index-signatures=true&include-expiration-date-signatures=true`, {
    method: "POST",
    headers: new Headers(
      {
        "Authorization": `Bearer ${API_TOKEN}`,
        "Content-Type": "application/json"
      }
    ),
    body: request
  });

  const credentialShares = await sharesRes.json();
  console.log(credentialShares);

  console.log("unblinding shares");
  const unblinded = cryptoData.unblindWalletShares(credentialShares, partialVks, masterVk);

  const serialised = unblinded.serialise();
  console.log("serialised:\n", serialised);
}


main();
