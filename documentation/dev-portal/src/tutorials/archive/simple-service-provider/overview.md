# Tutorial Overview

## Components Created in this Tutorial
This tutorial involves writing two pieces of code in Typescript:

- A **User Client** (UC) with which you can access the mixnet through a browser on your local machine. You will use this to communicate with the second component outlined below.  
- A **Service Provider** (SP) which can communicate with the UC via the mixnet.

Additionally you will learn how to configure a pair of **Nym Websocket Clients** which both components use to connect to and communicate with the Mixnet.

> SPs usually run on remote servers to assure reliable uptime and to unlink sender and receiver metadata. For demonstration purposes however, you will run both components on your local machine, looping messages through the mixnet to yourself.  



```
       +----------+              +----------+             +----------+ 
       | Mix Node |<-----------> | Mix Node |<----------->| Mix Node |
       | Layer 1  |              | Layer 2  |             | Layer 3  |
       +----------+              +----------+             +----------+  
            ^                                                   ^      
            |                                                   |      
            |<--------------------------------------------------+
            |                                                          
            v                                                        
    +--------------+                                
    | Your gateway |                               
    +--------------+                               
            ^                                       
            |                                                                      
            |                                                                         
            v                                                        
+-------------------------------------------+                         
|                                           |                        
|  +------------+     +------------+        |                      
|  | Nym Client |     | Nym Client |        |                     
|  +------------+     +------------+        |                    
|        ^                  ^               |                   
|        |                  |               |                  
|        |                  |               |                 
|        v                  v               |                
|  +-------------+    +------------------+  |               
|  | User Client |    | Service Provider |  |              
|  +-------------+    +------------------+  |             
|                                           |            
+-------------------------------------------+           
            Your Local Machine          
```       

## Aims of this Tutorial 
* Create a user-friendly experience for sending data through the mixnet via a simple form accessible through a web browser. 
* Configure and use a pair of Nym Websocket Clients. 
* Send a properly formatted message through the mixnet to the SP from a browser-based GUI. 

You can find the code for these components [here](https://github.com/nymtech/developer-tutorials). You can use it as a reference while building or simply download it and follow along as you progress through the tutorial.

Notice that this tutorial attempts to use very few external libraries (the User Client codebase is basically vanilla Typescript!). This tutorial is not showing you how to build production-grade code, but **to understand how to connect and send messages to, as well as recieve messages from, the mixnet.**

```admonish note title="Sidenote: What is a Service Provider?" 
'Service Provider' is a catchall name used to refer to any type of app that can communicate with the mixnet via a Nym client. 

The first SP to have been released is the [Network Requester](https://nymtech.net/docs/nodes/network-requester.html) - a binary that receives a network request from the mixnet, performs that request (e.g. authenticating with a message server and receiving new messages for a user) and then passes the response back to the user who requested it anonymously, shielding their metadata from the message server. 

The SP you will build in this tutorial is far more simple than this. It is just to show you how to approach building something that can:
* connect to the mixnet, 
* listen for messages, and 
* perform some action with them (in this case, log them in a console and reply to the original sender). 

However, once you see how easy it is to integrate with the mixnet for traffic transport, you will be able to build apps with real-world uses easily. 
```
 
