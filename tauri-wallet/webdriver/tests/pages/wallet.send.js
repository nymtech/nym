class WalletSend {

    
    get fromAddress(){
        return $("#from");
    }

    get toAddress(){
        return $("#to");
    }

    get amount(){
        return $("#amount")
    }

    //can these selectors be nicer like the id's?
    get nextButton(){
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(3) > button > span.MuiButton-label");
    }

    get sendHeader(){
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardHeader-root > div > span");
    }

    get transferAmount(){
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(2) > div > div > div:nth-child(7) > p:nth-child(2)");
    }

    get accountBalance(){
        return $("#root > div > div:nth-child(1) > div:nth-child(3) > div:nth-child(1) > div > div.MuiCardContent-root > div > div > h6");
    }

    get amountReviewAndSend(){
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(2) > div > div > div:nth-child(5) > p:nth-child(2)");
    }

    get toAddressReviewAndSend(){
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(2) > div > div > div:nth-child(3) > p:nth-child(2)");
    }

    get fromAddressReviewAndSend(){
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(2) > div > div > div:nth-child(1) > p:nth-child(2)");
    }

    get transferFeeAmount(){
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(2) > div > div > div:nth-child(7) > p:nth-child(2)");
    }

    get reviewAndSendBackButton(){
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(3) > button.MuiButtonBase-root.MuiButton-root.MuiButton-text.MuiButton-disableElevation > span.MuiButton-label");
    }

    get sendButton(){
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(3) > button.MuiButtonBase-root.MuiButton-root.MuiButton-contained.MuiButton-containedPrimary.MuiButton-disableElevation > span.MuiButton-label");
    }

    get transactionComplete(){
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(2) > div > div:nth-child(1) > p");
    }

    get transactionCompleteRecipient(){
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(2) > div > div.MuiPaper-root.MuiCard-root.MuiPaper-outlined.MuiPaper-rounded > div:nth-child(1) > div:nth-child(2) > p");

    }

    get transactionCompleteAmount(){
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(2) > div > div.MuiPaper-root.MuiCard-root.MuiPaper-outlined.MuiPaper-rounded > div:nth-child(2) > div:nth-child(2) > p");
    }

    get finishButton(){
        return $("#root > div > div:nth-child(2) > div:nth-child(2) > div > div > div > div.MuiCardContent-root > div > div:nth-child(3) > button");
    }

}

module.exports = new WalletSend()