class Send {
    
    get sendHeader() { return $("[data-header-testid='Send']") }  // TO-DO see if Send value can be used without changing the key name
}
export default new Send() 