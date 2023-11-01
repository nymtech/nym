# Message Types
[//]: # (TODO expand! )
There are two methods for sending messages through the mixnet using your client:
* `send_plain_message()` is the most simple: pass the recipient address and the message you wish to send as a string (this was previously `send_str()`). This is a nicer-to-use wrapper around `send_message()`.
* `send_message()` allows you to also define the amount of SURBs to send along with your message (which is sent as bytes). 
