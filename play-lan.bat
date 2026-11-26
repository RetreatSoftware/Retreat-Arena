@echo off
rem One command for the local network: builds the web version and starts the server that serves it.
rem Details: scripts\play-lan.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\play-lan.ps1" %*
if errorlevel 1 pause
