                                                                                                                                                                    
  +-------------------+                  +-------------------+                                                                                      
  | +---------------+ |                  |                   | Packet lost in transmission - no ack recieved!                                       
  | |  Nym client   | |                  |                   |-------------X                                                                        
  | +-------^-------+ |Send 100 packets  |                   |                                                                                      
  |         |         |----------------->|   Gateway your    |  Resend packet    +------------------+     etc...                                          
  |         |         |                  |   client is       |------------------>|                  |------------------>                                               
  |         |         |                  |   connected to    |                   | Mix node layer 1 |                                               
  |         v         | Send 100 acks    |                   |<------------------|                  |                                               
  | +---------------+ |<-----------------|                   |   Send ack        +------------------+                                               
  | | Your app code | |                  |                   |                                                                                      
  | +---------------+ |                  |                   |                                                                                      
  +-------------------+                  +-------------------+                                                                                      
   Your Local Machine                                                                                                                               
