#!/bin/bash

#kill existing process
kill -9 "$(pgrep aux | grep -E "WebKitWeb|tauri-dri" | awk '{print $2}')"
exit 0;
