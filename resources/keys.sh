#!/bin/bash

set -e

rm ed25519*

# Create key pairs

echo '\n\n' | ssh-keygen -t ed25519 -f ed25519-ca
echo '\n\n' | ssh-keygen -t ed25519 -f ed25519-user
echo '\n\n' | ssh-keygen -t ed25519 -f ed25519-host

# Create user cert

ssh-keygen \
    -s "ed25519-ca" \
    -I "cert1" \
    -z "1"  \
    -n "user1,user2" \
    -V "+2000w" \
    -O "critical:force-command=ls" \
    -O "critical:source-address=10.0.0.0/16,127.0.0.1/32" \
    "ed25519-user.pub"

ssh-keygen -L -f "ed25519-user-cert.pub"
cat "ed25519-user-cert.pub" | cut -d' ' -f2 | base64 -d > "ed25519-user-cert.pub.raw"

# Create host cert

ssh-keygen \
    -s "ed25519-ca" \
    -I "cert2" \
    -z "2"  \
    -h \
    -n "foo.example.com,bar.example.com" \
    -V "+2000w" \
    "ed25519-host.pub"

ssh-keygen -L -f "ed25519-host-cert.pub"
cat "ed25519-host-cert.pub" | cut -d' ' -f2 | base64 -d > "ed25519-host-cert.pub.raw"
