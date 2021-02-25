export function log(text: string, thing: any) {
    let msg = JSON.stringify(thing);
    console.log(`${text}: ${msg}`);
}