#!/bin/bash
# Disable screensaver and screen lock for VNC sessions
export DISPLAY=:1

# X server settings
xset s off
xset s noblank

# XFCE4 screensaver
xfconf-query -c xfce4-screensaver -p /saver/enabled -n -t bool -s false 2>/dev/null || true
xfconf-query -c xfce4-screensaver -p /lock/enabled -n -t bool -s false 2>/dev/null || true

# XFCE4 power manager
xfconf-query -c xfce4-power-manager -p /xfce4-power-manager/blank-on-ac -n -t int -s 0 2>/dev/null || true
xfconf-query -c xfce4-power-manager -p /xfce4-power-manager/blank-on-battery -n -t int -s 0 2>/dev/null || true
xfconf-query -c xfce4-power-manager -p /xfce4-power-manager/dpms-enabled -n -t bool -s false 2>/dev/null || true
xfconf-query -c xfce4-power-manager -p /xfce4-power-manager/dpms-on-ac-off -n -t int -s 0 2>/dev/null || true
xfconf-query -c xfce4-power-manager -p /xfce4-power-manager/dpms-on-ac-sleep -n -t int -s 0 2>/dev/null || true
xfconf-query -c xfce4-power-manager -p /xfce4-power-manager/dpms-on-battery-off -n -t int -s 0 2>/dev/null || true
xfconf-query -c xfce4-power-manager -p /xfce4-power-manager/dpms-on-battery-sleep -n -t int -s 0 2>/dev/null || true
