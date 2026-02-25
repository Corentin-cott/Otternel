#!/bin/bash

trigger_test="^[[m>....^M^[[K^[[32m[14:43:14] [Server thread/INFO] [minecraft/MinecraftServer]: <pokebloque> cc"

# Simple test script to test 1.log
echo $trigger_test >> 1.log

# Simple test script to test 2.log
echo $trigger_test >> 2.log