class Send {
    
    // send nym form
    get sendHeader() { return $("[data-testid='Send']") }
    get recipientAddress() { return $("[data-testid='recipientAddress']") }
    // get sendAmount() { return $("[data-testid='Amount']") }
    get sendAmount() { return $("#mui-5") } // TO-DO fix this selector, using #mui-5 isn't a good solution
    get next() { return $("[data-testid='Next']") }

    // confirm transaction modal
    get sendDetailsHeader() { return $("[data-testid='Send details']") }
    get from() { return $("/html/body/div[2]/div[3]/div[2]/div[1]/div[1]") }
    get to() { return $("/html/body/div[2]/div[3]/div[2]/div[2]") }
    get amount() { return $("/html/body/div[2]/div[3]/div[2]/div[3]") }
    get fee() { return $("/html/body/div[2]/div[3]/div[2]/div[4]") }

    get confirm() { return $("[data-testid='Confirm']") }


    // transaction sent
    get viewOnBlockchain() { return $("[data-testid='viewOnBlockchain']") }
    get done() { return $("[data-testid='Done']") }




}
export default new Send() 