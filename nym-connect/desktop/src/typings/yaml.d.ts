declare module '*.yml' {
  const content: { [key: string]: any };
  export default content;
}

declare module '*.yaml' {
  const content: { [key: string]: any };
  export default content;
}
