#!/bin/bash
DEF="/c/projects/file-io-proxy/encoding-vfs-windows/lib/winfsp-x64.def"
EXPORTS="/c/projects/file-io-proxy/encoding-vfs-windows/lib/export_names.txt"

echo "LIBRARY winfsp-x64" > "$DEF"
echo "EXPORTS" >> "$DEF"
while read -r name; do
    echo "    $name" >> "$DEF"
done < "$EXPORTS"
