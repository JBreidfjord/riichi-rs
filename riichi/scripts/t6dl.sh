#!/usr/bin/env bash
id=$1
shift
wget --referer "http://tenhou.net/6/?log=$id" "http://tenhou.net/5/mjlog2json.cgi?$id" -O "$id.json" "$@"
