#!/bin/bash

killall chitchat-test

cargo build --release

for i in $(seq 10000 10004)
do
    listen_addr="127.0.0.1:$i";
    echo ${listen_addr};
    cargo run --release -- --listen_addr ${listen_addr} --seed 127.0.0.1:10000 --node_id node_$i &
done;

read
kill 0
