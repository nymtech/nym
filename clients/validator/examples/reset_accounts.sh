#!/bin/bash
# If you do not yet have any accounts in your computer's keychain, you'll need to set some up:

add_account() {
    user=$1

    nymd keys add "$user" &> tmp.txt
    mnemonic=$(< tmp.txt tail -n1)
    address=$(nymd keys show "$user" -a)
    echo "$mnemonic" > "accounts/$user.key"
    echo "$address" > "accounts/$user.address"
}

cleanup() {
    rm tmp.txt
}

echo "Deleting accounts, don't worry about the scary message when it happens..."
nymd keys delete dave -y
nymd keys delete fred -y
nymd keys delete bob -y
nymd keys delete thief -y
echo "Accounts deleted."
echo ""
echo "Re-adding accounts..."
mkdir -p accounts
add_account dave
add_account fred
add_account bob
add_account thief

cleanup
echo "All done."