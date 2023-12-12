export function appendOutput(value: string) {
  const el = document.getElementById('output') as HTMLPreElement;
  const text = document.createTextNode(`${value}\n`);
  el.appendChild(text);
}

export function appendImageOutput(url: string) {
  const el = document.getElementById('outputImage') as HTMLPreElement;
  const imgNode = document.createElement('img');
  imgNode.src = url;
  el.appendChild(imgNode);
}
