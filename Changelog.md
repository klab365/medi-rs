# Changelog

All notable changes to this project will be documented in this file.

## Purpose

This file present the software status in form of a "Changelog".

## Scope

This document is valid within the scope of the work for all projects.

## 1.2.0

### Added

* Add macros for IntoCommand, FromResource, etc. to simplify command and resource handling (see readme).

## 1.1.0

### Changed

* Update Error handling to use `Result` type for better error management.
* Handler error will be boxed and with the function `get_handler_error` to retrieve the error message from the handler

## 1.0.0

* Initial release
