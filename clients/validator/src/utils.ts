// eslint-disable-next-line @typescript-eslint/no-explicit-any,@typescript-eslint/explicit-module-boundary-types
export default function log(text: string, thing: any): void {
    const msg = JSON.stringify(thing);
    console.log(`${text}: ${msg}`);
}