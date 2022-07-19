import Auth from '../../pageobjects/authScreens'
import Balance from '../../pageobjects/balanceScreen'


describe('Create a new account and verify it exists', () => {
    it('clicking on create account generates new mnemonic', async () => {
        await (await Auth.createAccount).click()
        await (await Auth.mnemonicPhrase).waitForDisplayed({ timeout : 1000})()
        await (await Auth.copyMnemonic).click()
        await (await expect(Auth.iSavedMnemonic).toBeClickable())
    })

    it('complete new account creation', async () => {
        await (await Auth.createAccount).click()
        let mnemonic = await (await Auth.mnemonicPhrase).getText()
        let arrayMnemonic = mnemonic.split(" ")
        // the below will print out the second word of the mnemonic saved
        console.log(arrayMnemonic[1]);
        await (await Auth.copyMnemonic).click()
        await (await Auth.iSavedMnemonic).click()

        // for each number, find the equivalent index from arrayMnemonic, capture word, then find word in mnemonicWord, and click it
        // OR for each word, find the index+1 and match it to the number seen on the screen

        await (await Auth.number).waitForDisplayed()
        let itemNumber = await (await Auth.number).getText()
        let word = await (await Auth.mnemonicWord).getText()

        arrayMnemonic.forEach(function (item, index) {
            console.log(item, index);
            for (let num = 0; num < 6; num++) {
                if ((index + 1) == itemNumber) {
                    console.log('mne word ' + word)
                    Auth.mnemonicWord.click()
                }
            }
        })

        // for (let num = 0; num < 6; num++) {
        //     await (await Auth.number).forEach(item => {
        //         let numb = item.getText()
        //         const itemArray = Array.from(numb)
        //         console.log('the numberrr ' + itemArray)
        //     })
        // }

        // const test = arrayMnemonic.some(word => word.index = itemNumber);
        // console.log('the test again ' + test);
        // await (await Auth.mnemonicWord).click()



        // var values = [];
        // let numbers = await (await Auth.number).()
        // for (var i = 0; i < numbers.length; i++) {
        //     values.push(numbers[i].value);
        // }

        // var numm = (Auth.number).children;
        // const numarray = Array.from(numm);
        // numarray.push;
        // console.log('the numbers ' + numarray)


        // the below will print out the text of the first mnemonic button
        let text = await (await Auth.mnemonicWord).getText([1])
        console.log('the mnemonic texttttt' + text)


        await (await Auth.mnemonicWord).click()

    })

})