# Go FFI
> ⚠️ This is an initial version of this library in order to give developers something to experiment with. If you use this code to begin testing out Mixnet integration and run into issues, errors, or have feedback, please feel free to open an issue; feedback from developers trying to use it will help us improve it. If you have questions feel free to reach out via our [Matrix channel](https://matrix.to/#/#dev:nymtech.chat).

## Useage (WIP)
```
# if unset run from projroot 
RUST_BINARIES=target/release
echo 'export LD_LIBRARY_PATH=${LD_LIBRARY_PATH}:'${RUST_BINARIES} >> ~/.zshrc
source ~/.zshrc

# (optional) clean everything 
./clean.sh

# build everything 
./build.sh 

# manually add the following line under #include math.h` in compiled Go: 
# this is a temporary hack that will b hidden in build script
// #cgo LDFLAGS: -L../../target/release -lnym_go_ffi

# run test go file 
go run ffi/main.go
```
 