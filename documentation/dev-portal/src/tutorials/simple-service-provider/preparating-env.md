# Preparing Your User Client Environment 

## Prerequisites
* `NodeJS` & `npm` 
* `typescript` 

## Preparing your TypeScript environment  

* Make a new directory called `simple-service-provider-tutorial` containing a directory named `user-client`:

```
mkdir -p simple-service-provider/user-client
```

* Create a `package.json` and install dependencies: 

```
cd simple-service-provider/user-client 
npm init  
npm install typescript # allows you to write and use typescript 
npm install ts-node --save-dev # allows you to build a typescript application in a NodeJS environment 
```

* Create a `tsconfig.json` containing the following:

```json
{
    "compilerOptions": {
        "module": "commonjs",
        "esModuleInterop": true,
        "target": "es6",
        "moduleResolution": "node",
        "sourceMap": true,
        "outDir": "dist"
    },
    "lib": ["es2015"]
}
```

## Preparing your Bundler 
* We will use the [`Parcel`](https://parceljs.org/getting-started/webapp/) bundler to build and run our app locally. `Parcel` also supports hot reloading, making for a nicer developer experience when working on our app. Install it with:

```
npm install parcel-bundler
```

* Create the file structure for our frontend code: 

```
mkdir src 
touch src/index.html src/index.ts
```

~~~admonish note title=""
At this point your directory should look like this (check yourself with `tree -L 2 simple-service-provider/`): 
```
simple-service-provider/
└── user-client
    ├── node_modules
    ├── package.json
    ├── package-lock.json
    ├── src
    └── tsconfig.json

4 directories, 3 files
```

And `user-client/src/` should look like this: 
```
user-client/src
├── index.html
└── index.ts

1 directory, 2 files
```
~~~

* Time to check everything is working. Paste the following into `src/index.html`:
    
```
<!DOCTYPE html>
<html>
    <head>
        <title>App Test</title>
        <meta charset="utf-8"/>
    </head>
    <body>
        <h1>Test</h1>
        <div id="app"></div>
        <script src="index.ts"></script>
    </body>
</html>
```

* Paste the following into `src/index.ts`

```
console.log('test log')
```
    
* Add the following to `package.json` in the `"scripts"` array, above `"test"`:

```
"start": "parcel src/index.html"
```

* `npm start` should return: 

```
> user-client@1.0.0 start
> parcel src/index.html

Server running at http://localhost:1234
✨  Built in 1.57s.
```

Open [localhost:1234](http://localhost:1234/) in your browser. Your web application should be up and running, with `Test` displayed in the browser window. Checking the `console.log` output is done by right-clicking on the browser and selecting _Inspect_, then navigating to the _Console_ section of the resulting panel. You should see the message `test log` displayed there.
 