'$IMPORT_STATEMENT';

function appendOutput(value: string, id?: string) {
  const el = document.getElementById('output') as HTMLPreElement;
  const div = document.createElement('div');
  if (id) {
    div.id = id;
  }
  const text = document.createTextNode(`${value}\n`);
  div.appendChild(text);
  el.appendChild(div);
}

function appendImageOutput(url: string) {
  const el = document.getElementById('outputImage') as HTMLPreElement;
  const imgNode = document.createElement('img');
  imgNode.src = url;
  el.appendChild(imgNode);
}

/**
 * The main entry point
 */
async function main() {
  appendOutput('Tester is starting up...', 'starting');

  const url = 'https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt';
  appendOutput(`Using mixFetch to get ${url}...`);
  const args = { mode: 'unsafe-ignore-cors' };

  const resp = await mixFetch(url, args, {
    preferredGateway: '2BuMSfMW3zpeAjKXyKLhmY4QW1DXurrtSPEJ6CjX3SEh',
    preferredNetworkRequester:
      'GiRjFWrMxt58pEMuusm4yT3RxoMD1MMPrR9M2N4VWRJP.3CNZBPq4vg7v7qozjGjdPMXcvDmkbWPCgbGCjQVw9n6Z@2xU4CBE6QiiYt6EyBXSALwxkNvM7gqJfjHXaMkjiFmYW',
  });
  console.log({ resp });
  const text = await resp.text();

  appendOutput(JSON.stringify(resp, null, 2), 'text-output');
  appendOutput(JSON.stringify({ text }, null, 2));

  // // get an image
  // url = 'https://nymtech.net/favicon.svg';
  // resp = await mixFetch(url, args);
  // console.log({ resp });
  // const buffer = await resp.arrayBuffer();
  // const type = resp.headers.get('Content-Type') || 'image/svg';
  // const blobUrl = URL.createObjectURL(new Blob([buffer], { type }));
  // appendOutput(JSON.stringify({ bufferBytes: buffer.byteLength, blobUrl }, null, 2), 'image-output');
  // appendImageOutput(blobUrl);

  appendOutput('✅ Done', 'done');
}

// wait for the html to load
window.addEventListener('DOMContentLoaded', () => {
  // let's do this!
  main();
});
