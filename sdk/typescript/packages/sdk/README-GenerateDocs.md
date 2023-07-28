## Install dependencies

run `yarn install` to install dependencies

## Generate docs

We use TypeDoc for document generation https://typedoc.org/

To generate docs run `yarn docs:generate`. Generated docs can be found the `./docs` directory.

To view the generated docs in a webpage run `yarn docs:serve`. The docs will be available to view at `http://localhost:3000`.

## Local document development

To support the development process we have a local server that will watch for changes to the src files and update the docs in real time.

Run `yarn docs:dev` to start a local server to view the docs. Again, The docs will be available to view at `http://localhost:3000`.
