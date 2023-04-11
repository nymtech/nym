# Preparing Your Service Provider Environment

* Now to move on to prepare our development environment for the Service Provider code. Create a directory for it:

```
# run this from the root of `simple-service-provider/`
mkdir service-provider
cd service-provider
```

* Create a `package.json`: 

```
npm init
```

* Inside your newly generated `package.json`, paste in the following code:

```json
{
    "name": "service-provider",
    "version": "1.0.0",
    "description": "",
    "main": "index.js",
    "scripts": {
        "start:dev": "nodemon",
        "test": "echo \"Error: no test specified\" && exit 1"
    },
    "devDependencies": {
        "@types/node": "^18.14.0",
        "@types/ws": "^8.5.4",
        "nodemon": "^2.0.20",
        "ts-node": "^10.9.1",
        "typescript": "^4.8.4"
    },
    "author": "",
    "license": "ISC",
    "dependencies": {
        "ws": "^8.12.0"
    }
}
```

* install dependecies: 

```
npm install
``` 

* create a `tsconfig.json` file containing the following: 

```json
{
    "compilerOptions": {
    "target": "es2017", 
    "lib": [
        "es6"
    ],
    "module": "Node16", 
    "rootDir": "src", 
    "resolveJsonModule": true, 
    "allowJs": true,                      
    "outDir": "build", 
    "esModuleInterop": true, 
    "forceConsistentCasingInFileNames": true, 
    "strict": true, 
    "noImplicitAny": true, 
    "skipLibCheck": true 
    }
}
```

* You will use [Nodemon](https://www.npmjs.com/package/nodemon) to reload your app on code changes. Create a `nodemon.json` file in the same directory which will act as our `nodemon` configuration. Paste in the following code inside that file:

```json
{
    "watch": [
        "src"
    ],
    "ext": ".ts,.js",
    "ignore": [],
    "exec": "ts-node ./src/index.ts"
}
```

* Finally, create a typescript file for our app logic: 

```
mkdir src
touch src/index.ts
```

~~~admonish note title=""
At this point your directory should look like this (check yourself with `tree -L 2 simple-service-provider`): 
```
simple-service-provider
├── service-provider
│   ├── node_modules
│   ├── nodemon.json
│   ├── package.json
│   ├── package-lock.json
│   ├── src
│   └── tsconfig.json
└── user-client
    ├── node_modules
    ├── package.json
    ├── package-lock.json
    ├── src
    └── tsconfig.json

7 directories, 7 files
```

And `service-provider/src/` should look like this: 
```
service-provider/src
└── index.ts

1 directory, 1 file
```
~~~
