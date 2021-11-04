declare global {
    namespace NodeJS {
        interface ProcessEnv {
            NODE_ENV: 'local' | 'prod' | 'devfeature';
        }
    }
}

export { }