#!/bin/bash

#kill existing process
kill -9 $(ps aux | egrep "WebKitWeb|tauri-dri" | awk '{print $2}')
exit 0;
