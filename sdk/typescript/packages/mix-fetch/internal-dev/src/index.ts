import { createMixFetch } from '@nymproject/mix-fetch';

/**
 * The main entry point
 */
async function main() {
  console.log('Tester is starting up...');

  const worker = await createMixFetch();
  (window as any).mixFetch = worker;

  if (!worker) {
    console.error('Oh no! Could not create mixFetch');
  }

  // only really useful if you want to adjust some settings like traffic rate
  // (if not needed you can just pass a null)

  console.log('About to set up mixFetch...');

  const addr =
    'BUFKbUncPWL4WkQPHc7JRusdXwXKi5omS8Fz5Rr34JoZ.8iM69a1pjcMwLEdQCkmq5jdGi8tsSpbjQqk6YWQAX6Ae@3ojQD6V7skM1bSXJX7fVQvscjmcgptzdixQEaAha2ixh';
  // const addr =
  //   '9W68B5YXZjNsxziy5XF4wnRaoWSYwkCXRCshcP7YkiMV.D1DcfVJHCtdu7NYkfpXBGHtzx62wB4BGWqFJnyfu83ZR@37Vp9iTW7vhLApw2CziipBsZGf2JjRMf1cFZJkKZ4QtH';
  // const addr =
  //   '3zzhLtWvaJgn755MkRckG5aRnoTZich8ASn395iSsTgj.J1R5VuxXbh2eNHiaRbrwbKGXrrEQcHKLdzf8eg9HTB6q@3B7PsbXFuqq6rerYFLw5HPbQb4UmBqAhfWURRovMmWoj';
  // const addr =
  //   'C4w6ewbQtoaZEeoaaNw1xVASChqo4WVjNfuYEUFjZxpc.8F1D7rQXf2jGoj1Ken7PiGDM8HS2Ug79wSoc9nZ1iqh1@62F81C9GrHDRja9WCqozemRFSzFPMecY85MbGwn6efve';

  console.log('Instantiating Mix Fetch...');
  // await setupMixFetch(config, {storagePassphrase: "foomp"})

  await worker.setupMixFetch(addr, {
    clientId: 'my-new-client-11',
    clientOverride: { coverTraffic: { disableLoopCoverTrafficStream: true } },
    mixFetchOverride: { requestTimeoutMs: 60000 },
  });

  console.log('Ready!');

  console.log('Using mixFetch...');
  const url = 'https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt';
  const args = { mode: 'unsafe-ignore-cors' };

  const resp = await worker.mixFetch(url, args);
  console.log({ resp });

  const el = document.getElementById('output') as HTMLPreElement;
  const text = document.createTextNode(JSON.stringify(resp, null, 2));
  el.appendChild(text);
}

// wait for the html to load
window.addEventListener('DOMContentLoaded', () => {
  // let's do this!
  main();
});
