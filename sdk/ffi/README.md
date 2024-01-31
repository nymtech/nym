# FFI 
This repo contains bindings for C/C++ and Go in the respectively named directories. 

`ffi/shared/` contains shared 'internal' functions which are imported by bindings. 

The bonus of this approach is that by wrapping and matching the returned val of the `_internal` fn, you can have Rusty error propogation / handling, but also pass back e.g. a simple return code across the FFI boundary, reducing the need to pass complex types. 
