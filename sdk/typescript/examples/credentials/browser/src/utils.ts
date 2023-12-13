export function appendOutput(value: string) {
  const el = document.getElementById('credential') as HTMLPreElement;
  const text = document.createTextNode(`${value}\n`);
  el.appendChild(text);
}

