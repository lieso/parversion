#!/bin/bash
set -e
set -o pipefail

document=$(mktemp)
echo "document: $document"

while IFS= read -r line; do
  echo "$line" >> "$document"
done

output=$(cat "$document" | cargo run -- "$@")

filename=$(echo "$output" | jq '.parsers' | md5sum | awk '{print $1}')
echo "$output" | jq '.parsers' > "./parsers/$filename.json"

echo "$output" | jq '.data'

rm "$document"
