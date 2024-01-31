# Community Counsel

```admonish info
The entire content of this page is under [Creative Commons Attribution 4.0 International Public License](https://creativecommons.org/licenses/by/4.0/).
```

Running an Exit Gateway is a commitment and as such is exposed to various challenges. Besides different legal regulations typical difficulties may be dealing with VPS or ISP providers. Our strength lies in decentralised community of squads and individuals supporting each other. Sharing examples of [landing pages](landing-pages.md), templates for communication and FAQs is a great way to empower other operators sharing the mission of liberating internet. Below is a simple way how to create a pull request directly to Nym Operator Guide book.

## How to add content

Our aim is to establish a strong community network, sharing legal findings and other suggestions with each other. We would like to encourage all of the current and future operators to do research about the situation in the jurisdiction they operate in as well as solutions to any challenges when running an Exit Gateway and add those through a pull request (PR).

First of all, please join our [Node Operators Legal Forum](https://matrix.to/#/!YfoUFsJjsXbWmijbPG:nymtech.chat?via=nymtech.chat&via=matrix.org) (Matrix chat) and post any information or questions there.

To add your information to this book, you can create a PR directly to our [repository](https://github.com/nymtech/nym/tree/develop/documentation/operators/src/legal), than ping the admins in the [Legal Forum chat](https://matrix.to/#/!YfoUFsJjsXbWmijbPG:nymtech.chat?via=nymtech.chat&via=matrix.org) and we will review it as fast as possible. 

Here is how:

1. Write down your legal findings, suggestions, communication templates, FAQ in a text editor (Soon we will share a template)

2. Clone `nymtech/nym` repository or pull in case you already have it and switch to `develop` branch

```sh
# Clone the repository
git clone https://github.com/nymtech/nym

# Go to the directory nym
cd nym

# Switch to branch develop
git checkout develop

# Update the repository
git pull origin develop
```

3. Make your own branch based off `develop` and switch to it

```sh
# choose a descriptive and consise name without using <>
git branch operators/legal-forum/<MY_BRANCH_NAME>
git checkout operators/legal-forum/<MY_BRANCH_NAME>

# you can double check that you are on the right branch
git branch
```

4. Save your legal findings as `<JURISDICTION_NAME>.md` to `/nym/documentation/operators/src/legal` or add info to an existing page

5. Don't change anything in `SUMMARY.md`, the admins will do it when merging

6. Add, commit and push your changes

```sh
cd documentation/operators/src/legal
git add <FILE_NAME>.md
git commit -am "<DESCRIBE YOUR CHANGES>"
git push origin operators/legal-forum/<MY_BRANCH_NAME>
```
7. Open the git generated link in your browser, fill the description and click on `Create a Pull Request` button
```sh
# The link will look like this
 https://github.com/nymtech/nym/pull/new/<MY_BRANCH_NAME>
```
8. Notify others in the [Node Operators Legal Forum](https://matrix.to/#/!YfoUFsJjsXbWmijbPG:nymtech.chat?via=nymtech.chat&via=matrix.org) (Matrix chat) about the PR.


