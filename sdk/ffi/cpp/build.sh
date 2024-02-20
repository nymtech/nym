#!/usr/bin/env bash

C_CODE_NAME="main"
PROJECT_NAME="cpp"

clean_artifacts() {
  printf "cleaning cargo artifacts \n"
  cargo clean

  if [ -e "src/${C_CODE_NAME}" ]
  then
    printf "cleaning compiled C++ \n"
    rm src/${C_CODE_NAME}
  else
    printf "no compiled C++ to clean \n"
  fi
}

build_artifacts_and_link() {
  cargo build --release &&
  cd src/ &&
  printf "compiling cpp \n"
  g++ -std=c++11 -o main main.cpp -ldl -lpthread -L../target/release -lnym_cpp_ffi -lboost_thread &&
  export LD_LIBRARY_PATH=../target/release:$LD_LIBRARY_PATH 
  # check output for name of rust lib - can be helpful if you've changed e.g. the name of a file and the compilation is failing
  # printf "ldd main: \n"
  # ldd main
}

if [ $(pwd | awk -F/ '{print $NF}') != ${PROJECT_NAME} ]
then
  printf "please run from root dir of project"
  exit 1
fi

if [ $# -eq 0 ];
then
  build_artifacts_and_link;
  ./main;
else
  arg=$1
  if [ "$arg" == "clean" ]; then
    clean_artifacts;
    build_artifacts_and_link;
    ./main;
  else
      echo "unknown optional argument - the only available optional argument is 'clean'"
      exit 1
  fi
fi

