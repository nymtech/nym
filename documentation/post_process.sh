#!/bin/bash
# this is a script called by the github CI and CD workflows to post process CSS/image/href links for serving
# several mdbooks from a subdirectory

cd scripts/post-process
npm install
node index.mjs
