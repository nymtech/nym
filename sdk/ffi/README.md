# FFI 
This repo contains bindings for C/C++ and Go in the respectively named directories. 

`shared/` contains shared 'internal' functions which are imported by bindings. Primarily these functions rely on managing:
* that the client is not mutated by multiple threads simultaneously 
* that client actions happen in blocking threads 

