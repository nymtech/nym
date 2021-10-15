class WalletHome {

    get balanceCheck() {
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardHeader-root > div > span");
    }

    get punkBalance() {
        return $("#root > div > div:nth-child(1) > div:nth-child(3) > div:nth-child(1) > div > div.MuiCardContent-root > div > div > h6");
    }

    get punkAddress() {
        return $("#root > div > div:nth-child(1) > div:nth-child(3) > div:nth-child(2) > div > div.MuiCardContent-root > div > p");
    }

    get sendButton() {
        return $("#root > div > div:nth-child(1) > div:nth-child(4) > div > ul > a:nth-child(2) > div.MuiListItemText-root > span");
    }

    get receiveButton() {
        return $("#root > div > div:nth-child(1) > div:nth-child(4) > div > ul > a:nth-child(3) > div.MuiListItemText-root > span");
    }

    get bondButton(){
        return $("#root > div > div:nth-child(1) > div:nth-child(4) > div > ul > a:nth-child(4) > div.MuiListItemText-root > span");
    }

    get unBondButton(){
        return $("#root > div > div:nth-child(1) > div:nth-child(4) > div > ul > a:nth-child(5) > div.MuiListItemText-root > span");
    }

    get delegateButton(){
        return $("#root > div > div:nth-child(1) > div:nth-child(4) > div > ul > a:nth-child(6) > div.MuiListItemText-root > span");
    }

    get unDelegateButton(){
        return $("#root > div > div:nth-child(1) > div:nth-child(4) > div > ul > a:nth-child(7) > div.MuiListItemText-root > span");
    }

    get docsButton(){
        return $("#root > div > div:nth-child(1) > div:nth-child(4) > div > ul > a:nth-child(8) > div.MuiListItemText-root > span");
    }

    get logOutButton(){
        return $("#root > div > div:nth-child(1) > div:nth-child(4) > div > ul > div > div.MuiListItemText-root > span");
    }

}

module.exports = new WalletHome()