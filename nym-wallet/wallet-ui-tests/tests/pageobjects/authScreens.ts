class Welcome {
    //Welcome landing page
    get signInButton() { return $("[data-testid='signIn']") }
    get createAccount() { return $("[data-testid='createAccount']") }

    // Existing account sign in option page
    get signInMnemonic() { return $("[data-testid='signInWithMnemonic']") }
    get signInPassword() { return $("[data-testid='signInWithPassword']") }
    get backToWelcomePage() { return $("[data-testid='backToWelcomePage']") }
    get forgotPassword() { return $("[data-testid='forgotPassword']") }

    // Sign in with mnemonic page 
    get mnemonicInput() { return $("[data-testid='mnemonicInput']") }
    get signIn() { return $("[data-testid='signIn']") }
    get backToSignInOptions() { return $("[data-testid='backToSignInOptions']") }
    get createPassword() { return $("[data-testid='createPassword']") }

    // Error
    get error() { return $("[data-testid='error']") }

    // Sign in with password 

    // Create account step 1/3
    get copyMnemonic() { return $("[data-testid='copyMnemonic']") }
    get iSavedMnemonic() { return $("[data-testid='iSavedMnemonic']") }
    get mnemonicPhrase() { return $("[data-testid='mnemonicPhrase']") }
    get backToWelcomePageFromCreate(){ return $("[data-testid='backToWelcome']")}

    // Create account step 2/3
    get number() { return $("[data-testid='number']") }
    get randomMnemonicWords() { return $("[data-testid='randomMnemonicWords']") }
    //random words // random hidden words (top and bottom row, with name and index each)
    get mnemonicWord() { return $("[data-testid='mnemonicWord']") }
    // get randomMnemonicWords() { return $("#root > div > div > div > div:nth-child(6) > div:nth-child(1) > div > div") }
    get next() { return $("[data-testid='next']") }
    get backToStep1() { return $("[data-testid='backToStep1']") }

    // Create account step 3/3
    get password() { return $("[data-testid='password']") }
    get confirmpassword() { return $("[data-testid='confirmpassword']") }
    // get next() { return $("[data-testid='next']") }
    get skipPasswordAndSignInWithMnemonic() { return $("[data-testid='skipPasswordAndSignInWithMnemonic']") }

}

export default new Welcome()