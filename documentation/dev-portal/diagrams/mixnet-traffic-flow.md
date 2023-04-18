                                                                               
       +----------+              +----------+             +----------+                 
       | Mix Node |<-----------> | Mix Node |<----------->| Mix Node |                 
       | Layer 1  |              | Layer 2  |             | Layer 3  |                 
       +----------+              +----------+             +----------+                 
            ^                                                   ^                      
            |                                                   |                      
            |                                                   |                      
            v                                                   v                      
    +--------------+                                   +-----------------+        
    | Your gateway |                                   | Service gateway |        
    +--------------+                                   +-----------------+        
            ^                                                    ^                     
            |                                                    |                     
            |                                                    |                     
            v                                                    v                     
  +-------------------+                                +-------------------+           
  | +---------------+ |                                | +---------------+ |           
  | |  Nym client   | |                                | |  Nym Client   | |           
  | +---------------+ |                                | +---------------+ |           
  |         ^         |                                |         ^         |           
  |         |         |                                |         |         |           
  |         |         |                                |         |         |           
  |         v         |                                |         v         |           
  | +---------------+ |                                | +---------------+ |           
  | | Your app code | |                                | | Service Code  | |           
  | +---------------+ |                                | +---------------+ |           
  +-------------------+                                +-------------------+           
   Your Local Machine**                               Service Provider Machine**        


** note that depending on the technical setup, the Nym client running on these machines may
be either a seperate process or embedded in the same process as the app code via one of our SDKs. 