@echo off
rem Builds the game, then runs a local server and two client windows. Details: scripts\play.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\play.ps1" %*
if errorlevel 1 pause
