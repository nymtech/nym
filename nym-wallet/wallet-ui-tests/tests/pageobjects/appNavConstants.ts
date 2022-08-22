class Nav {

    get lightMode() { return $("[data-testid='LightModeOutlinedIcon']") }
    get darkMode() { return $("[data-testid='ModeNightOutlinedIcon']") }
    get terminalTitle() { return $("[data-testid='terminal-header']") }
    get terminalIcon() { return $("[data-testid='TerminalIcon']") }


    get balance() { return $("[data-testid='Balance']") }
    get send() { return $("[data-testid='Send']") }
    get receive() { return $("[data-testid='Receive']") }
    get bond() { return $("[data-testid='Bond']") }
    get unbond() { return $("[data-testid='Unbond']") }
    get delegation() { return $("[data-testid='Delegation']") }



    get closeIcon() { return $("[data-testid='CloseIcon']") }

}
export default new Nav() 