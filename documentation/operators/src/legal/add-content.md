# Adding Content to Legal Forum

Our aim is to establish a strong community network, sharing legal findings and other suggestions with each other. We would like to encourage all of the current and future operators to do research about the situation in the jurisdiction they operate in, to share solutions to the challenges they encountered when running Exit Gateway, and create a pull request (PR).

First of all, please join our [Node Operators Legal Forum](https://matrix.to/#/!YfoUFsJjsXbWmijbPG:nymtech.chat?via=nymtech.chat&via=matrix.org) (Matrix chat) and post any information or questions there.

To add your information to this book, you can create a PR directly to our [repository](https://github.com/nymtech/nym/tree/develop/documentation/operators/src/legal). 

**Steps to make a pull request:**

1. Write down your legal findings, suggestions, communication templates, FAQ in a text editor

2. Clone `nymtech/nym` repository or pull in case you already have it and switch to `develop` branch

```sh
# clone the repository
git clone https://github.com/nymtech/nym

# go to the directory nym
cd nym

# switch to branch develop
git checkout develop

# update the repository
git pull origin develop
```

3. Make your own branch based off `develop` and switch to it

```sh
# choose a descriptive and consise name without using <>
git checkout -b operators/legal-forum/<MY_BRANCH_NAME>

# example: git checkout -b operators/legal-forum/alice-vps-faq-template

# you can double check that you are on the right branch
git branch
```

4. Save your legal findings as `<JURISDICTION_NAME>.md` to `/nym/documentation/operators/src/legal` or add info to an existing page

5. **Don't change anything in `SUMMARY.md`**, the admins will do it when merging

6. Add, commit and push your changes

```sh
cd documentation/operators/src/legal
git add <FILE_NAME>.md
git commit -am "<DESCRIBE YOUR CHANGES>"
git push origin operators/legal-forum/<MY_BRANCH_NAME>
```
7. Open the git generated link in your browser, fill the description and click on `Create a Pull Request` button
```sh
# the url will look like this
 https://github.com/nymtech/nym/pull/new/<MY_BRANCH_NAME>
```
8. Notify others in the [Node Operators Legal Forum](https://matrix.to/#/!YfoUFsJjsXbWmijbPG:nymtech.chat?via=nymtech.chat&via=matrix.org) (Matrix chat) about the PR.
