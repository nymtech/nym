                                                                               
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
  +-------------------+                                        
  | +---------------+ |                               
  | |  Nym client   | |                              
  | +---------------+ |                              
  |         ^         |                             
  |         |         |                               
  |         |         |                               
  |         v         |                               
  | +---------------+ |                               
  | | Your app code | |                               
  | +---------------+ |                               
  +-------------------+                               
   Your Local Machine**                              


** note that depending on the technical setup, the Nym client running on this machine may
be either a seperate process or embedded in the same process as the app code via one of our SDKs. 