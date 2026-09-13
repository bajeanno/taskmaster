#!/bin/bash

PID_D="/var/run/taskmaster.d"
mkdir -p "${PID_D}"
chmod a+rw "${PID_D}"
